use crate::get_secret_key_from_keypair;
use base64::engine::general_purpose::STANDARD as base64_engine;
use base64::Engine;
use pkarr::dns::rdata::RData;
use pkarr::dns::ResourceRecord;
use pkarr::Keypair;
use pubky_common::session::SessionInfo;
use serde_json::json;
use std::error::Error;
use std::net::{Ipv4Addr, Ipv6Addr};

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
            for (key, value) in https.0.iter_params() {
                params.insert(key.to_string(), json!(base64_engine.encode(value)));
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
            for (key, value) in svcb.iter_params() {
                params.insert(key.to_string(), json!(base64_engine.encode(value)));
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

pub fn session_to_json(session: &SessionInfo) -> String {
    let json_obj = json!({
        "pubky": session.public_key().to_string(),
        "capabilities": session.capabilities().iter().map(|c| c.to_string()).collect::<Vec<String>>(),
    });

    serde_json::to_string(&json_obj).unwrap_or_else(|e| format!("Failed to serialize JSON: {}", e))
}

pub fn session_to_json_with_secret(session: &SessionInfo, session_secret: &str) -> String {
    let json_obj = json!({
        "pubky": session.public_key().to_string(),
        "capabilities": session.capabilities().iter().map(|c| c.to_string()).collect::<Vec<String>>(),
        "session_secret": session_secret,
    });

    serde_json::to_string(&json_obj).unwrap_or_else(|e| format!("Failed to serialize JSON: {}", e))
}

pub fn keypair_to_json_string(keypair: &Keypair, mnemonic: Option<&str>) -> Result<String, String> {
    let secret_key = get_secret_key_from_keypair(keypair);
    let public_key = keypair.public_key();

    let json_obj = if let Some(mnemonic_str) = mnemonic {
        json!({
            "secret_key": secret_key,
            "public_key": public_key.to_string(),
            "mnemonic": mnemonic_str
        })
    } else {
        json!({
            "secret_key": secret_key,
            "public_key": public_key.to_string()
        })
    };

    serde_json::to_string(&json_obj).map_err(|_| "Error serializing to JSON".to_string())
}
