//! Read and edit a key's own DNS records (A, AAAA, CNAME, MX, TXT) in its
//! pkarr packet.
//!
//! A pkarr packet is one signed unit, so every write replaces all of it. The
//! functions here always start from the most recent packet that is known
//! (network, local cache, or a copy the caller kept), change only the records
//! they were asked to change, and copy every other record verbatim. In
//! particular the `_pubky` homeserver record is never shown, edited or dropped.

use crate::{create_response_vector, get_keypair_from_secret_key, get_pubky_client};
use bytes::Bytes;
use pubky::pkarr::dns::rdata::{RData, A, AAAA, MX, TXT};
use pubky::pkarr::dns::{Name, ResourceRecord, CLASS};
use pubky::pkarr::errors::{PublishError, ResolveError, SignedPacketBuildError};
use pubky::pkarr::{ResolvePolicy, SignedPacket, Timestamp};
use pubky::{Keypair, Pubky, PublicKey};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::net::{Ipv4Addr, Ipv6Addr};

/// Prefix of the homeserver record names that only the homeserver flow manages.
const RESERVED_PREFIX: &str = "_pubky";
/// Default MX preference when the caller does not give one.
const DEFAULT_MX_PRIORITY: u16 = 10;
/// How many times an edit is re-applied when a newer packet appears meanwhile.
const MAX_EDIT_ATTEMPTS: u32 = 3;

/// Stable error prefixes. The second element of an error response starts with
/// one of these, followed by `: ` and a human-readable detail.
pub mod error_codes {
    pub const INVALID_INPUT: &str = "PKDNS_INVALID_INPUT";
    pub const INVALID_RECORD: &str = "PKDNS_INVALID_RECORD";
    pub const RESERVED_NAME: &str = "PKDNS_RESERVED_NAME";
    pub const CNAME_CONFLICT: &str = "PKDNS_CNAME_CONFLICT";
    pub const PACKET_TOO_LARGE: &str = "PKDNS_PACKET_TOO_LARGE";
    pub const RESOLVE_FAILED: &str = "PKDNS_RESOLVE_FAILED";
    pub const PUBLISH_FAILED: &str = "PKDNS_PUBLISH_FAILED";
    pub const CONFLICT: &str = "PKDNS_CONFLICT";
}

fn error(code: &str, detail: impl std::fmt::Display) -> String {
    format!("{code}: {detail}")
}

/// One record as exchanged with the app. `name` is relative to the key:
/// `@` is the key itself, `*` a wildcard, `www` is `www.<key>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub ttl: u32,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u16>,
}

impl DnsRecord {
    /// Identity of a record: everything but the TTL.
    fn same_record(&self, other: &DnsRecord) -> bool {
        self.name == other.name
            && self.record_type == other.record_type
            && self.value == other.value
            && self.priority == other.priority
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct DnsRecordChanges {
    #[serde(default)]
    pub add: Vec<DnsRecord>,
    #[serde(default)]
    pub remove: Vec<DnsRecord>,
}

// ---------------------------------------------------------------------------
// Names and values
// ---------------------------------------------------------------------------

fn is_valid_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 63
        && bytes
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || *c == b'-' || *c == b'_')
        && bytes[0] != b'-'
        && bytes[bytes.len() - 1] != b'-'
}

/// Normalises and validates a relative record name.
fn normalize_relative_name(name: &str) -> Result<String, String> {
    let name = name.trim().trim_end_matches('.').to_ascii_lowercase();
    if name.is_empty() || name == "@" {
        return Ok("@".to_string());
    }
    // Leave room for `.<52-char key>` within the 253-character DNS limit.
    if name.len() > 200 {
        return Err(error(error_codes::INVALID_RECORD, "name is too long"));
    }
    for (index, label) in name.split('.').enumerate() {
        let wildcard = label == "*" && index == 0;
        if !wildcard && !is_valid_label(label) {
            return Err(error(
                error_codes::INVALID_RECORD,
                format!("invalid name `{name}`"),
            ));
        }
    }
    if name.starts_with(RESERVED_PREFIX) {
        return Err(error(
            error_codes::RESERVED_NAME,
            format!("`{name}` is managed by the homeserver settings"),
        ));
    }
    Ok(name)
}

fn absolute_name(relative: &str, origin: &str) -> String {
    if relative == "@" {
        origin.to_string()
    } else {
        format!("{relative}.{origin}")
    }
}

/// Returns the relative name of a record in a packet signed by `origin`, or
/// `None` if the record is not under the key (should not happen).
fn relative_name(absolute: &str, origin: &str) -> Option<String> {
    if absolute == origin {
        return Some("@".to_string());
    }
    absolute
        .strip_suffix(origin)
        .and_then(|prefix| prefix.strip_suffix('.'))
        .map(str::to_string)
}

fn normalize_domain(value: &str) -> Result<String, String> {
    let domain = value.trim().trim_end_matches('.').to_ascii_lowercase();
    if domain.is_empty() || domain.len() > 253 || !domain.split('.').all(is_valid_label) {
        return Err(error(
            error_codes::INVALID_RECORD,
            format!("`{value}` is not a valid domain name"),
        ));
    }
    Ok(domain)
}

/// Validates an incoming record and returns it in canonical form, the same
/// form `resolve_dns_records` reports, so records can be compared.
fn normalize_record(record: &DnsRecord) -> Result<DnsRecord, String> {
    let name = normalize_relative_name(&record.name)?;
    let record_type = record.record_type.trim().to_ascii_uppercase();
    if record.ttl == 0 {
        return Err(error(error_codes::INVALID_RECORD, "TTL must be positive"));
    }
    let invalid = |what: &str| {
        error(
            error_codes::INVALID_RECORD,
            format!("`{}` is not a valid {what}", record.value),
        )
    };
    let (value, priority) = match record_type.as_str() {
        "A" => (
            record
                .value
                .trim()
                .parse::<Ipv4Addr>()
                .map_err(|_| invalid("IPv4 address"))?
                .to_string(),
            None,
        ),
        "AAAA" => (
            record
                .value
                .trim()
                .parse::<Ipv6Addr>()
                .map_err(|_| invalid("IPv6 address"))?
                .to_string(),
            None,
        ),
        "CNAME" => (normalize_domain(&record.value)?, None),
        "MX" => (
            normalize_domain(&record.value)?,
            Some(record.priority.unwrap_or(DEFAULT_MX_PRIORITY)),
        ),
        "TXT" => {
            if record.value.is_empty() {
                return Err(error(error_codes::INVALID_RECORD, "TXT value is empty"));
            }
            (record.value.clone(), None)
        }
        other => {
            return Err(error(
                error_codes::INVALID_RECORD,
                format!("unsupported record type `{other}`"),
            ))
        }
    };
    Ok(DnsRecord {
        name,
        record_type,
        ttl: record.ttl,
        value,
        priority,
    })
}

/// Converts a packet record into the app's form. Returns `None` for records
/// the app does not manage (the homeserver record and other record types).
fn record_from_resource(rr: &ResourceRecord<'_>, origin: &str) -> Option<DnsRecord> {
    let name = relative_name(&rr.name.to_string(), origin)?;
    if name.starts_with(RESERVED_PREFIX) {
        return None;
    }
    let (record_type, value, priority) = match &rr.rdata {
        RData::A(a) => ("A", Ipv4Addr::from(a.address).to_string(), None),
        RData::AAAA(aaaa) => ("AAAA", Ipv6Addr::from(aaaa.address).to_string(), None),
        RData::CNAME(cname) => ("CNAME", cname.0.to_string(), None),
        RData::MX(mx) => ("MX", mx.exchange.to_string(), Some(mx.preference)),
        RData::TXT(txt) => ("TXT", String::try_from(txt.clone()).ok()?, None),
        _ => return None,
    };
    Some(DnsRecord {
        name,
        record_type: record_type.to_string(),
        ttl: rr.ttl,
        value,
        priority,
    })
}

/// Builds a packet record from a normalised app record.
fn resource_from_record(
    record: &DnsRecord,
    origin: &str,
) -> Result<ResourceRecord<'static>, String> {
    let owner = absolute_name(&record.name, origin);
    // Names were validated above. `new_unchecked` also accepts the `*` label,
    // which `Name::new` rejects.
    let name = Name::new_unchecked(&owner).into_owned();
    let rdata: RData<'static> = match record.record_type.as_str() {
        "A" => RData::A(A {
            address: record
                .value
                .parse::<Ipv4Addr>()
                .map_err(|e| error(error_codes::INVALID_RECORD, e))?
                .into(),
        }),
        "AAAA" => RData::AAAA(AAAA {
            address: record
                .value
                .parse::<Ipv6Addr>()
                .map_err(|e| error(error_codes::INVALID_RECORD, e))?
                .into(),
        }),
        "CNAME" => RData::CNAME(Name::new_unchecked(&record.value).into_owned().into()),
        "MX" => RData::MX(MX {
            preference: record.priority.unwrap_or(DEFAULT_MX_PRIORITY),
            exchange: Name::new_unchecked(&record.value).into_owned(),
        }),
        "TXT" => RData::TXT(
            TXT::try_from(record.value.as_str())
                .map_err(|e| error(error_codes::INVALID_RECORD, e))?
                .into_owned(),
        ),
        other => {
            return Err(error(
                error_codes::INVALID_RECORD,
                format!("unsupported record type `{other}`"),
            ))
        }
    };
    Ok(ResourceRecord::new(name, CLASS::IN, record.ttl, rdata))
}

// ---------------------------------------------------------------------------
// Pure packet logic
// ---------------------------------------------------------------------------

/// The next packet timestamp: now, but always later than the base packet so
/// the new packet supersedes it even if the device clock is behind.
fn next_timestamp(base: Option<&SignedPacket>) -> Timestamp {
    let now = Timestamp::now();
    match base {
        Some(packet) if packet.timestamp() >= now => packet.timestamp() + 1,
        _ => now,
    }
}

/// Applies `changes` to `base` and signs the result.
///
/// Every record of `base` that is not removed is copied unchanged, including
/// the `_pubky` homeserver record and record types the app does not manage.
/// Adding a record that already exists (same name, type and value) replaces
/// its TTL. Removing a record that does not exist is a no-op.
pub fn apply_record_changes(
    keypair: &Keypair,
    base: Option<&SignedPacket>,
    changes: &DnsRecordChanges,
    timestamp: Timestamp,
) -> Result<SignedPacket, String> {
    let origin = keypair.public_key().z32();
    let adds = changes
        .add
        .iter()
        .map(normalize_record)
        .collect::<Result<Vec<_>, _>>()?;
    let removes = changes
        .remove
        .iter()
        .map(normalize_record)
        .collect::<Result<Vec<_>, _>>()?;

    let mut records: Vec<ResourceRecord<'static>> = Vec::new();
    // Names and types of everything kept, to enforce the CNAME rule.
    let mut kept: Vec<(String, String)> = Vec::new();

    if let Some(packet) = base {
        for rr in packet.all_resource_records() {
            if let Some(existing) = record_from_resource(rr, &origin) {
                let removed = removes.iter().any(|r| r.same_record(&existing));
                let replaced = adds.iter().any(|a| a.same_record(&existing));
                if removed || replaced {
                    continue;
                }
                kept.push((existing.name.clone(), existing.record_type.clone()));
            } else if let Some(name) = relative_name(&rr.name.to_string(), &origin) {
                // Not managed here (e.g. `_pubky` HTTPS): kept as-is, and it
                // still counts as "another record" for the CNAME rule.
                kept.push((name, "OTHER".to_string()));
            }
            records.push(rr.clone().into_owned());
        }
    }

    let mut added_names = HashSet::new();
    for add in &adds {
        records.push(resource_from_record(add, &origin)?);
        kept.push((add.name.clone(), add.record_type.clone()));
        added_names.insert(add.name.clone());
    }

    // A CNAME must be the only record at its name. Only names touched by this
    // edit are checked, so pre-existing conflicts never block a delete.
    for name in &added_names {
        let at_name: Vec<_> = kept.iter().filter(|(n, _)| n == name).collect();
        let has_cname = at_name.iter().any(|(_, t)| t == "CNAME");
        if has_cname && at_name.len() > 1 {
            return Err(error(
                error_codes::CNAME_CONFLICT,
                format!("`{name}` has a CNAME, which cannot share its name with other records"),
            ));
        }
    }

    SignedPacket::new(keypair, &records, timestamp).map_err(|e| match e {
        SignedPacketBuildError::PacketTooLarge(size) => error(error_codes::PACKET_TOO_LARGE, size),
        other => error(error_codes::INVALID_RECORD, other),
    })
}

/// JSON describing a key's managed records.
pub fn records_json(public_key: &PublicKey, packet: Option<&SignedPacket>) -> serde_json::Value {
    let origin = public_key.z32();
    let Some(packet) = packet else {
        return json!({
            "public_key": origin,
            "found": false,
            "timestamp": null,
            "signed_packet": null,
            "packet_size": 0,
            "records": [],
            "other_records": 0,
        });
    };
    let mut records = Vec::new();
    let mut other_records = 0;
    for rr in packet.all_resource_records() {
        match record_from_resource(rr, &origin) {
            Some(record) => records.push(record),
            None => other_records += 1,
        }
    }
    json!({
        "public_key": origin,
        "found": true,
        "timestamp": packet.timestamp().as_u64(),
        "signed_packet": hex::encode(packet.as_bytes()),
        "packet_size": packet.encoded_packet().len(),
        "records": records,
        "other_records": other_records,
    })
}

/// Parses a packet the app stored earlier (hex of `signed_packet`), verifying
/// that it belongs to `public_key` and that its signature is valid.
fn parse_known_packet(public_key: &PublicKey, hex_packet: &str) -> Result<SignedPacket, String> {
    let bytes = hex::decode(hex_packet.trim())
        .map_err(|e| error(error_codes::INVALID_INPUT, format!("stored packet: {e}")))?;
    if bytes.len() < 32 || bytes[..32] != public_key.as_bytes()[..] {
        return Err(error(
            error_codes::INVALID_INPUT,
            "stored packet belongs to a different key",
        ));
    }
    SignedPacket::from_relay_payload(public_key, &Bytes::copy_from_slice(&bytes[32..]))
        .map_err(|e| error(error_codes::INVALID_INPUT, format!("stored packet: {e}")))
}

fn most_recent(packets: impl IntoIterator<Item = Option<SignedPacket>>) -> Option<SignedPacket> {
    packets
        .into_iter()
        .flatten()
        .fold(None, |best, packet| match best {
            Some(best) if best.more_recent_than(&packet) || best.is_same_as(&packet) => Some(best),
            _ => Some(packet),
        })
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

/// Resolves the most recent packet from the network and the local cache, also
/// considering a packet the app stored. A network failure is an error unless
/// the network reported that no packet exists: writing on top of a packet we
/// could not read is how records get lost.
async fn resolve_latest(
    public_key: &PublicKey,
    known: Option<SignedPacket>,
) -> Result<Option<SignedPacket>, String> {
    let client = get_pubky_client();
    resolve_latest_with(client.client().pkarr(), public_key, known).await
}

async fn resolve_latest_with(
    pkarr: &pubky::pkarr::Client,
    public_key: &PublicKey,
    known: Option<SignedPacket>,
) -> Result<Option<SignedPacket>, String> {
    let network = match pkarr.resolve(public_key, ResolvePolicy::NetworkOnly).await {
        Ok(packet) => Some(packet),
        Err(ResolveError::NotFound) => None,
        Err(e) => return Err(error(error_codes::RESOLVE_FAILED, e)),
    };
    let cached = pkarr
        .resolve(public_key, ResolvePolicy::CacheOnly)
        .await
        .ok();
    Ok(most_recent([network, cached, known]))
}

/// Checks that a key's current packet can be read before the SDK publishes its
/// `_pubky` record.
///
/// The SDK treats a failed network read as "no packet" and would then publish
/// `_pubky` alone, dropping every other record of the key. A successful read
/// also puts the packet in the client's cache, which the SDK reads as well, so
/// it builds on top of it. A key with no packet yet passes.
pub(crate) async fn require_readable_packet(
    client: &Pubky,
    public_key: &PublicKey,
) -> Result<(), String> {
    resolve_latest_with(client.client().pkarr(), public_key, None)
        .await
        .map(|_| ())
}

fn parse_optional_known(
    public_key: &PublicKey,
    known_packet: Option<String>,
) -> Result<Option<SignedPacket>, String> {
    match known_packet.filter(|p| !p.trim().is_empty()) {
        Some(hex_packet) => parse_known_packet(public_key, &hex_packet).map(Some),
        None => Ok(None),
    }
}

fn parse_public_key(public_key: &str) -> Result<PublicKey, String> {
    PublicKey::try_from(public_key)
        .map_err(|e| error(error_codes::INVALID_INPUT, format!("public key: {e}")))
}

fn respond(result: Result<serde_json::Value, String>) -> Vec<String> {
    match result {
        Ok(value) => create_response_vector(false, value.to_string()),
        Err(message) => create_response_vector(true, message),
    }
}

/**
* Reads the A, AAAA, CNAME, MX and TXT records a key publishes.
*
* Records under `_pubky` and other record types are not listed but counted in
* `other_records`; editing never drops them.
*
* `known_packet` is an optional `signed_packet` returned earlier; the most
* recent of it, the network and the local cache is reported.
*
* @returns `[error, json]`. The JSON has `public_key`, `found`, `timestamp`,
* `signed_packet` (hex, to store for `known_packet`), `packet_size` (bytes of
* the 1000 allowed), `records` and `other_records`.
**/
#[uniffi::export]
pub fn resolve_dns_records(public_key: String, known_packet: Option<String>) -> Vec<String> {
    respond(
        crate::TOKIO_RUNTIME
            .clone()
            .block_on(resolve_dns_records_inner(public_key, known_packet)),
    )
}

async fn resolve_dns_records_inner(
    public_key: String,
    known_packet: Option<String>,
) -> Result<serde_json::Value, String> {
    let public_key = parse_public_key(&public_key)?;
    let known = parse_optional_known(&public_key, known_packet)?;
    let latest = resolve_latest(&public_key, known).await?;
    Ok(records_json(&public_key, latest.as_ref()))
}

/**
* Adds and removes DNS records and publishes the packet.
*
* `changes_json` is `{"add": [record], "remove": [record]}`, where a record is
* `{"name", "type", "ttl", "value", "priority"?}`. Every other record in the
* packet, including `_pubky`, is kept. If the network cannot be read the edit
* is refused. If a newer packet appears while publishing, the edit is
* re-applied on top of it.
*
* @returns `[error, json]` with the same JSON as `resolve_dns_records` for the
* published packet. Errors start with a `PKDNS_*` code.
**/
#[uniffi::export]
pub fn edit_dns_records(
    secret_key: String,
    changes_json: String,
    known_packet: Option<String>,
) -> Vec<String> {
    respond(
        crate::TOKIO_RUNTIME
            .clone()
            .block_on(edit_dns_records_inner(
                secret_key,
                changes_json,
                known_packet,
            )),
    )
}

async fn edit_dns_records_inner(
    secret_key: String,
    changes_json: String,
    known_packet: Option<String>,
) -> Result<serde_json::Value, String> {
    let keypair = get_keypair_from_secret_key(&secret_key)
        .map_err(|e| error(error_codes::INVALID_INPUT, e))?;
    let public_key = keypair.public_key();
    let changes: DnsRecordChanges = serde_json::from_str(&changes_json)
        .map_err(|e| error(error_codes::INVALID_INPUT, format!("changes: {e}")))?;
    let mut known = parse_optional_known(&public_key, known_packet)?;
    let client = get_pubky_client();

    for _ in 0..MAX_EDIT_ATTEMPTS {
        // The stored packet only seeds the first attempt; retries re-read.
        let base = resolve_latest(&public_key, known.take()).await?;
        let packet = apply_record_changes(
            &keypair,
            base.as_ref(),
            &changes,
            next_timestamp(base.as_ref()),
        )?;
        match client.client().pkarr().publish(&packet).await {
            Ok(_) => return Ok(records_json(&public_key, Some(&packet))),
            // Someone published a newer packet meanwhile: re-read it and
            // re-apply this edit on top.
            Err(PublishError::NotMostRecent) => continue,
            Err(e) => return Err(error(error_codes::PUBLISH_FAILED, e)),
        }
    }
    Err(error(
        error_codes::CONFLICT,
        "the records kept changing while publishing; try again",
    ))
}

/**
* Re-announces a key's current packet to the DHT without re-signing it, so
* records do not expire when nobody else republishes them.
*
* Needs no secret key and never changes the packet, so it cannot overwrite a
* newer edit. `known_packet` (a stored `signed_packet`) is re-announced when
* the network no longer has the packet.
*
* @returns `[error, json]` with the same JSON as `resolve_dns_records` plus
* `republished` (false when there is nothing to republish).
**/
#[uniffi::export]
pub fn republish_packet(public_key: String, known_packet: Option<String>) -> Vec<String> {
    respond(
        crate::TOKIO_RUNTIME
            .clone()
            .block_on(republish_packet_inner(public_key, known_packet)),
    )
}

async fn republish_packet_inner(
    public_key: String,
    known_packet: Option<String>,
) -> Result<serde_json::Value, String> {
    let public_key = parse_public_key(&public_key)?;
    let known = parse_optional_known(&public_key, known_packet)?;
    let client = get_pubky_client();
    let pkarr = client.client().pkarr();
    // Unlike an edit, a failed network read is fine here: re-announcing an
    // existing signed packet can never replace a newer one.
    let network = pkarr
        .resolve(&public_key, ResolvePolicy::NetworkOnly)
        .await
        .ok();
    let cached = pkarr
        .resolve(&public_key, ResolvePolicy::CacheOnly)
        .await
        .ok();
    let Some(packet) = most_recent([network, cached, known]) else {
        let mut value = records_json(&public_key, None);
        value["republished"] = json!(false);
        return Ok(value);
    };
    let republished = match pkarr.publish(&packet).await {
        Ok(_) => true,
        // The network already has something newer, which is fine.
        Err(PublishError::NotMostRecent) => false,
        Err(e) => return Err(error(error_codes::PUBLISH_FAILED, e)),
    };
    let mut value = records_json(&public_key, Some(&packet));
    value["republished"] = json!(republished);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubky::pkarr::dns::rdata::{HTTPS, SVCB};

    fn record(name: &str, record_type: &str, value: &str) -> DnsRecord {
        DnsRecord {
            name: name.to_string(),
            record_type: record_type.to_string(),
            ttl: 300,
            value: value.to_string(),
            priority: None,
        }
    }

    fn homeserver_packet(keypair: &Keypair, extra: &[DnsRecord]) -> SignedPacket {
        let origin = keypair.public_key().z32();
        let mut records = vec![ResourceRecord::new(
            Name::new_unchecked(&format!("_pubky.{origin}")).into_owned(),
            CLASS::IN,
            3600,
            RData::HTTPS(HTTPS(SVCB::new(
                0,
                Name::new_unchecked("homeserver.example").into_owned(),
            ))),
        )];
        for record in extra {
            records
                .push(resource_from_record(&normalize_record(record).unwrap(), &origin).unwrap());
        }
        SignedPacket::new(keypair, &records, Timestamp::now()).unwrap()
    }

    fn listed(keypair: &Keypair, packet: &SignedPacket) -> Vec<DnsRecord> {
        serde_json::from_value(records_json(&keypair.public_key(), Some(packet))["records"].clone())
            .unwrap()
    }

    fn has_homeserver(keypair: &Keypair, packet: &SignedPacket) -> bool {
        let origin = keypair.public_key().z32();
        packet
            .all_resource_records()
            .any(|rr| rr.name.to_string() == format!("_pubky.{origin}"))
    }

    #[test]
    fn edits_keep_the_homeserver_record() {
        let keypair = Keypair::random();
        let base = homeserver_packet(&keypair, &[record("@", "A", "192.0.2.1")]);

        let added = apply_record_changes(
            &keypair,
            Some(&base),
            &DnsRecordChanges {
                add: vec![record("*", "AAAA", "2001:DB8::1")],
                remove: vec![],
            },
            next_timestamp(Some(&base)),
        )
        .unwrap();
        assert!(has_homeserver(&keypair, &added));
        assert_eq!(
            listed(&keypair, &added),
            vec![
                record("@", "A", "192.0.2.1"),
                record("*", "AAAA", "2001:db8::1")
            ]
        );
        assert!(added.more_recent_than(&base));

        let removed = apply_record_changes(
            &keypair,
            Some(&added),
            &DnsRecordChanges {
                add: vec![],
                remove: vec![
                    record("@", "A", "192.0.2.1"),
                    record("*", "AAAA", "2001:db8::1"),
                ],
            },
            next_timestamp(Some(&added)),
        )
        .unwrap();
        assert!(has_homeserver(&keypair, &removed));
        assert!(listed(&keypair, &removed).is_empty());
        assert_eq!(
            records_json(&keypair.public_key(), Some(&removed))["other_records"],
            1
        );
    }

    #[test]
    fn records_round_trip_through_the_packet() {
        let keypair = Keypair::random();
        let mut mx = record("@", "MX", "Mail.Example.com.");
        mx.priority = Some(5);
        let long_txt = "v=spf1 ".to_string() + &"a".repeat(300);
        let packet = apply_record_changes(
            &keypair,
            None,
            &DnsRecordChanges {
                add: vec![
                    record("www", "CNAME", "example.com"),
                    mx,
                    record("_dnslink.docs", "TXT", &long_txt),
                ],
                remove: vec![],
            },
            Timestamp::now(),
        )
        .unwrap();
        let mut expected_mx = record("@", "MX", "mail.example.com");
        expected_mx.priority = Some(5);
        assert_eq!(
            listed(&keypair, &packet),
            vec![
                record("www", "CNAME", "example.com"),
                expected_mx,
                record("_dnslink.docs", "TXT", &long_txt),
            ]
        );
    }

    #[test]
    fn rejects_invalid_and_conflicting_records() {
        let keypair = Keypair::random();
        let base = homeserver_packet(&keypair, &[record("www", "A", "192.0.2.1")]);
        let try_add = |r: DnsRecord| {
            apply_record_changes(
                &keypair,
                Some(&base),
                &DnsRecordChanges {
                    add: vec![r],
                    remove: vec![],
                },
                Timestamp::now(),
            )
            .unwrap_err()
        };
        assert!(try_add(record("@", "A", "2001:db8::1")).starts_with(error_codes::INVALID_RECORD));
        assert!(try_add(record("a..b", "A", "192.0.2.1")).starts_with(error_codes::INVALID_RECORD));
        assert!(try_add(record("_pubky", "TXT", "x")).starts_with(error_codes::RESERVED_NAME));
        assert!(
            try_add(record("www", "CNAME", "example.com")).starts_with(error_codes::CNAME_CONFLICT)
        );
        assert!(try_add(record("@", "TXT", &"a".repeat(1200)))
            .starts_with(error_codes::PACKET_TOO_LARGE));
        // The homeserver record cannot be removed through this API.
        let remove_homeserver = apply_record_changes(
            &keypair,
            Some(&base),
            &DnsRecordChanges {
                add: vec![],
                remove: vec![record("_pubky", "TXT", "x")],
            },
            Timestamp::now(),
        );
        assert!(remove_homeserver
            .unwrap_err()
            .starts_with(error_codes::RESERVED_NAME));
    }

    #[test]
    fn stored_packets_are_verified() {
        let keypair = Keypair::random();
        let packet = homeserver_packet(&keypair, &[]);
        let stored = hex::encode(packet.as_bytes());
        let parsed = parse_known_packet(&keypair.public_key(), &stored).unwrap();
        assert!(parsed.is_same_as(&packet));

        let other = Keypair::random();
        assert!(parse_known_packet(&other.public_key(), &stored).is_err());

        let mut tampered = packet.as_bytes().to_vec();
        let last = tampered.len() - 1;
        tampered[last] ^= 1;
        assert!(parse_known_packet(&keypair.public_key(), &hex::encode(tampered)).is_err());
    }
}
