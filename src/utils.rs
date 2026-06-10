use crate::get_secret_key_from_keypair;
use base64::engine::general_purpose::STANDARD as base64_engine;
use base64::Engine;
use pubky::pkarr::dns::rdata::{RData, SVCParam};
use pubky::pkarr::dns::ResourceRecord;
use pubky::Keypair;
use pubky::PubkySession;
use serde_json::json;
use std::error::Error;
use std::net::{Ipv4Addr, Ipv6Addr};

// Serialize a single SVCB/HTTPS service parameter into JSON. In simple-dns 0.11
// (pkarr 6) params are a typed `SVCParam` enum rather than raw key/value bytes.
fn svc_param_to_json(param: &SVCParam) -> serde_json::Value {
    match param {
        SVCParam::Mandatory(keys) => json!(keys.iter().copied().collect::<Vec<u16>>()),
        SVCParam::Alpn(alpns) => {
            json!(alpns.iter().map(|a| a.to_string()).collect::<Vec<String>>())
        }
        SVCParam::NoDefaultAlpn => json!(true),
        SVCParam::Port(port) => json!(port),
        SVCParam::Ipv4Hint(ips) => {
            json!(ips
                .iter()
                .map(|ip| Ipv4Addr::from(*ip).to_string())
                .collect::<Vec<String>>())
        }
        SVCParam::Ech(data) => json!(base64_engine.encode(data.as_ref())),
        SVCParam::Ipv6Hint(ips) => {
            json!(ips
                .iter()
                .map(|ip| Ipv6Addr::from(*ip).to_string())
                .collect::<Vec<String>>())
        }
        SVCParam::InvalidKey => serde_json::Value::Null,
        SVCParam::Unknown(_, data) => json!(base64_engine.encode(data.as_ref())),
    }
}

pub fn r_data_to_json(r_data: &RData) -> serde_json::Value {
    match r_data {
        RData::A(a) => {
            json!({
                "type": "A",
                "address": Ipv4Addr::from(a.address).to_string()
            })
        }
        RData::AAAA(aaaa) => {
            json!({
                "type": "AAAA",
                "address": Ipv6Addr::from(aaaa.address).to_string()
            })
        }
        RData::AFSDB(afsdb) => {
            json!({
                "type": "AFSDB",
                "subtype": afsdb.subtype,
                "hostname": afsdb.hostname.to_string()
            })
        }
        RData::CAA(caa) => {
            json!({
                "type": "CAA",
                "flag": caa.flag,
                "tag": caa.tag.to_string(),
                "value": base64_engine.encode(&caa.value)
            })
        }
        RData::HINFO(hinfo) => {
            json!({
                "type": "HINFO",
                "cpu": hinfo.cpu.to_string(),
                "os": hinfo.os.to_string()
            })
        }
        RData::HTTPS(https) => {
            let mut params = serde_json::Map::new();
            for param in https.0.iter_params() {
                params.insert(param.key_code().to_string(), svc_param_to_json(param));
            }
            json!({
                "type": "HTTPS",
                "priority": https.0.priority,
                "target": https.0.target.to_string(),
                "params": params
            })
        }
        RData::MX(mx) => {
            json!({
                "type": "MX",
                "preference": mx.preference,
                "exchange": mx.exchange.to_string()
            })
        }
        RData::NAPTR(naptr) => {
            json!({
                "type": "NAPTR",
                "order": naptr.order,
                "preference": naptr.preference,
                "flags": naptr.flags.to_string(),
                "services": naptr.services.to_string(),
                "regexp": naptr.regexp.to_string(),
                "replacement": naptr.replacement.to_string()
            })
        }
        RData::NS(ns) => {
            json!({
                "type": "NS",
                "nsdname": ns.to_string()
            })
        }
        RData::NULL(_, null_record) => {
            json!({
                "type": "NULL",
                "data": base64_engine.encode(null_record.get_data())
            })
        }
        RData::OPT(opt) => {
            json!({
                "type": "OPT",
                "version": opt.version,
                "opt_codes": opt.opt_codes.iter().map(|code| {
                    json!({
                        "code": code.code,
                        "data": base64_engine.encode(&code.data)
                    })
                }).collect::<Vec<_>>()
            })
        }
        RData::PTR(ptr) => {
            json!({
                "type": "PTR",
                "ptrdname": ptr.to_string()
            })
        }
        RData::SOA(soa) => {
            json!({
                "type": "SOA",
                "mname": soa.mname.to_string(),
                "rname": soa.rname.to_string(),
                "serial": soa.serial,
                "refresh": soa.refresh,
                "retry": soa.retry,
                "expire": soa.expire,
                "minimum": soa.minimum
            })
        }
        RData::SRV(srv) => {
            json!({
                "type": "SRV",
                "priority": srv.priority,
                "weight": srv.weight,
                "port": srv.port,
                "target": srv.target.to_string()
            })
        }
        RData::SVCB(svcb) => {
            let mut params = serde_json::Map::new();
            for param in svcb.iter_params() {
                params.insert(param.key_code().to_string(), svc_param_to_json(param));
            }
            json!({
                "type": "SVCB",
                "priority": svcb.priority,
                "target": svcb.target.to_string(),
                "params": params
            })
        }
        RData::TXT(txt) => {
            let attributes = txt.attributes();
            let strings: Vec<String> = attributes
                .into_iter()
                .map(|(key, value)| match value {
                    Some(v) => format!("{}={}", key, v),
                    None => key.to_string(),
                })
                .collect();
            json!({
                "type": "TXT",
                "txt_data": strings
            })
        }
        RData::WKS(wks) => {
            json!({
                "type": "WKS",
                "address": Ipv4Addr::from(wks.address).to_string(),
                "protocol": wks.protocol,
                "bit_map": base64_engine.encode(&wks.bit_map)
            })
        }

        _ => json!({
            "type": "UNKNOWN"
        }),
    }
}

pub fn resource_record_to_json(rr: &ResourceRecord) -> serde_json::Value {
    json!({
        "name": rr.name.to_string(),
        "ttl": rr.ttl,
        "rdata": r_data_to_json(&rr.rdata)
    })
}

pub fn create_response_vector(error: bool, data: String) -> Vec<String> {
    vec![error.to_string(), data]
}

// Note: This function is currently disabled as the new pkarr API doesn't expose
// from_str_to_rdata or RDataType. This would need to be reimplemented if needed.
pub fn parse_dns_answers(
    _answers: &Vec<serde_json::Value>,
) -> Result<Vec<ResourceRecord<'_>>, Box<dyn Error>> {
    Err("parse_dns_answers is not supported in the upgraded pkarr version".into())
}

pub fn session_to_json(session: &PubkySession) -> String {
    let info = session.info();
    // z32() keeps the bare z-base32 form; PublicKey::to_string() now prepends
    // "pubky", which would break consumers building pubky:// URLs from this field.
    let json_obj = json!({
        "pubky": info.public_key().z32(),
        "capabilities": info.capabilities().iter().map(|c| c.to_string()).collect::<Vec<String>>(),
    });

    serde_json::to_string(&json_obj).unwrap_or_else(|e| format!("Failed to serialize JSON: {}", e))
}

pub fn session_to_json_with_secret(session: &PubkySession, session_secret: &str) -> String {
    let info = session.info();
    let json_obj = json!({
        "pubky": info.public_key().z32(),
        "capabilities": info.capabilities().iter().map(|c| c.to_string()).collect::<Vec<String>>(),
        "session_secret": session_secret,
    });

    serde_json::to_string(&json_obj).unwrap_or_else(|e| format!("Failed to serialize JSON: {}", e))
}

pub fn keypair_to_json_string(keypair: &Keypair, mnemonic: Option<&str>) -> Result<String, String> {
    let secret_key = get_secret_key_from_keypair(keypair);
    let public_key = keypair.public_key();
    let uri = public_key.to_uri_string();

    let json_obj = if let Some(mnemonic_str) = mnemonic {
        json!({
            "secret_key": secret_key,
            "public_key": public_key.z32(),
            "uri": uri,
            "mnemonic": mnemonic_str
        })
    } else {
        json!({
            "secret_key": secret_key,
            "public_key": public_key.z32(),
            "uri": uri
        })
    };

    serde_json::to_string(&json_obj).map_err(|_| "Error serializing to JSON".to_string())
}
