mod auth;
mod keypair;
mod tests;
mod types;
mod utils;

pub use auth::*;
pub use keypair::*;
pub use types::*;
pub use utils::*;

uniffi::setup_scaffolding!();

use base64::engine::general_purpose;
use base64::Engine;
use hex;
use hex::ToHex;
use ntimestamp::Timestamp;
use once_cell::sync::Lazy;
use pkarr::dns::rdata::{RData, HTTPS, SVCB};
use pkarr::dns::{Packet, ResourceRecord};
use pkarr::{dns, Keypair, PublicKey, SignedPacket};
use pubky::Client;
use pubky_common::recovery_file;
use pubky_common::session::Session;
use serde_json::json;
use std::str;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio;
use tokio::runtime::Runtime;
use tokio::time;
use url::Url;

pub struct NetworkClient {
    client: Mutex<Arc<Client>>,
}

impl NetworkClient {
    fn new() -> Self {
        Self {
            client: Mutex::new(Arc::new(Client::builder().build().unwrap())),
        }
    }

    pub fn switch_network(&self, use_testnet: bool) {
        let new_client = if use_testnet {
            Arc::new(Client::builder().testnet().build().unwrap())
        } else {
            Arc::new(Client::builder().build().unwrap())
        };

        let mut client = self.client.lock().unwrap();
        *client = new_client;
    }

    pub fn get_client(&self) -> Arc<Client> {
        self.client.lock().unwrap().clone()
    }
}

static NETWORK_CLIENT: Lazy<NetworkClient> = Lazy::new(|| NetworkClient::new());

pub fn get_pubky_client() -> Arc<Client> {
    NETWORK_CLIENT.get_client()
}

#[uniffi::export]
pub fn switch_network(use_testnet: bool) -> Vec<String> {
    NETWORK_CLIENT.switch_network(use_testnet);
    create_response_vector(
        false,
        format!(
            "Switched to {} network",
            if use_testnet { "testnet" } else { "default" }
        ),
    )
}

static TOKIO_RUNTIME: Lazy<Arc<Runtime>> =
    Lazy::new(|| Arc::new(Runtime::new().expect("Failed to create Tokio runtime")));

// Define the EventListener trait
#[uniffi::export(callback_interface)]
pub trait EventListener: Send + Sync {
    fn on_event_occurred(&self, event_data: String);
}

#[derive(uniffi::Object)]
pub struct EventNotifier {
    listener: Arc<Mutex<Option<Box<dyn EventListener>>>>,
}

impl EventNotifier {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            listener: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_listener(&self, listener: Box<dyn EventListener>) {
        let mut lock = self.listener.lock().unwrap();
        *lock = Some(listener);
    }

    pub fn remove_listener(&self) {
        let mut lock = self.listener.lock().unwrap();
        *lock = None;
    }

    pub fn notify_event(&self, event_data: String) {
        let lock = self.listener.lock().unwrap();
        if let Some(listener) = &*lock {
            listener.on_event_occurred(event_data);
        }
    }
}

static EVENT_NOTIFIER: Lazy<Arc<EventNotifier>> = Lazy::new(|| Arc::new(EventNotifier::new()));

#[uniffi::export]
pub fn set_event_listener(listener: Box<dyn EventListener>) {
    EVENT_NOTIFIER.as_ref().set_listener(listener);
}

#[uniffi::export]
pub fn remove_event_listener() {
    EVENT_NOTIFIER.as_ref().remove_listener();
}

pub fn start_internal_event_loop() {
    let event_notifier = EVENT_NOTIFIER.clone();
    let runtime = TOKIO_RUNTIME.clone();
    runtime.spawn(async move {
        let mut interval = time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            event_notifier
                .as_ref()
                .notify_event("Internal event triggered".to_string());
        }
    });
}

#[uniffi::export]
pub fn delete_file(url: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();
        let parsed_url = match Url::parse(&url) {
            Ok(url) => url,
            Err(_) => return create_response_vector(true, "Failed to parse URL".to_string()),
        };
        match client.delete(parsed_url).send().await {
            Ok(_) => create_response_vector(false, "Deleted successfully".to_string()),
            Err(error) => create_response_vector(true, format!("Failed to delete: {}", error)),
        }
    })
}

#[uniffi::export]
pub fn session(pubky: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();
        let public_key = match PublicKey::try_from(pubky) {
            Ok(key) => key,
            Err(error) => {
                return create_response_vector(
                    true,
                    format!("Invalid homeserver public key: {}", error),
                )
            }
        };
        let result = match client.session(&public_key).await {
            Ok(session) => session,
            Err(error) => {
                return create_response_vector(true, format!("Failed to get session: {}", error))
            }
        };
        let session: Session = match result {
            Some(session) => session,
            None => return create_response_vector(true, "No session returned".to_string()),
        };

        create_response_vector(false, session_to_json(&session))
    })
}

#[uniffi::export]
pub fn generate_secret_key() -> Vec<String> {
    let keypair = generate_keypair();
    let secret_key = get_secret_key_from_keypair(&keypair);
    let public_key = keypair.public_key();
    let uri = public_key.to_uri_string();
    let json_obj = json!({
       "secret_key": secret_key,
       "public_key": public_key.to_string(),
       "uri": uri,
    });

    let json_str = match serde_json::to_string(&json_obj) {
        Ok(json) => json,
        Err(e) => return create_response_vector(true, format!("Failed to serialize JSON: {}", e)),
    };
    start_internal_event_loop();
    create_response_vector(false, json_str)
}

#[uniffi::export]
pub fn get_public_key_from_secret_key(secret_key: String) -> Vec<String> {
    let keypair = match get_keypair_from_secret_key(&secret_key) {
        Ok(keypair) => keypair,
        Err(error) => return create_response_vector(true, error),
    };
    let public_key = keypair.public_key();
    let uri = public_key.to_uri_string();
    let json_obj = json!({
       "public_key": public_key.to_string(),
       "uri": uri,
    });

    let json_str = match serde_json::to_string(&json_obj) {
        Ok(json) => json,
        Err(e) => return create_response_vector(true, format!("Failed to serialize JSON: {}", e)),
    };
    create_response_vector(false, json_str)
}

#[uniffi::export]
pub fn publish_https(record_name: String, target: String, secret_key: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();

        let keypair = match get_keypair_from_secret_key(&secret_key) {
            Ok(keypair) => keypair,
            Err(error) => return create_response_vector(true, error),
        };

        // Create SVCB record with the target domain
        let target = match target.as_str().try_into() {
            Ok(target) => target,
            Err(e) => return create_response_vector(true, format!("Invalid target: {}", e)),
        };
        let svcb = SVCB::new(0, target);

        // Create HTTPS record
        let https_record = HTTPS(svcb);

        // Create DNS packet
        let mut packet = Packet::new_reply(0);
        let dns_name = match dns::Name::new(&record_name) {
            Ok(name) => name,
            Err(e) => return create_response_vector(true, format!("Invalid DNS name: {}", e)),
        };

        packet.answers.push(ResourceRecord::new(
            dns_name,
            dns::CLASS::IN,
            3600, // TTL in seconds
            dns::rdata::RData::HTTPS(https_record),
        ));

        let signed_packet = match SignedPacket::new(&keypair, &packet.answers, Timestamp::now()) {
            Ok(signed_packet) => signed_packet,
            Err(e) => {
                return create_response_vector(
                    true,
                    format!("Failed to create signed packet: {}", e),
                )
            }
        };
        match client
            .pkarr()
            .publish(&signed_packet, Some(Timestamp::now()))
            .await
        {
            Ok(()) => create_response_vector(false, keypair.public_key().to_string()),
            Err(e) => create_response_vector(true, format!("Failed to publish: {}", e)),
        }
    })
}

#[uniffi::export]
pub fn resolve_https(public_key: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let public_key = match public_key.as_str().try_into() {
            Ok(key) => key,
            Err(e) => return create_response_vector(true, format!("Invalid public key: {}", e)),
        };

        let client = get_pubky_client();

        match client.pkarr().resolve(&public_key).await {
            Some(signed_packet) => {
                // Extract HTTPS records from the signed packet
                let https_records: Vec<serde_json::Value> = signed_packet
                    .all_resource_records()
                    .filter_map(|record| {
                        if let dns::rdata::RData::HTTPS(https) = &record.rdata {
                            // Create a JSON object
                            let mut https_json = serde_json::json!({
                                "name": record.name.to_string(),
                                "class": format!("{:?}", record.class),
                                "ttl": record.ttl,
                                "priority": https.0.priority,
                                "target": https.0.target.to_string(),
                            });

                            // Access specific parameters using the constants from SVCB
                            if let Some(port_param) = https.0.get_param(SVCB::PORT) {
                                if port_param.len() == 2 {
                                    let port = u16::from_be_bytes([port_param[0], port_param[1]]);
                                    https_json["port"] = serde_json::json!(port);
                                }
                            }

                            // Access ALPN parameter if needed
                            if let Some(alpn_param) = https.0.get_param(SVCB::ALPN) {
                                // Parse ALPN protocols (list of character strings)
                                let mut position = 0;
                                let mut alpn_protocols = Vec::new();
                                while position < alpn_param.len() {
                                    let length = alpn_param[position] as usize;
                                    position += 1;
                                    if position + length <= alpn_param.len() {
                                        let protocol = String::from_utf8_lossy(
                                            &alpn_param[position..position + length],
                                        );
                                        alpn_protocols.push(protocol.to_string());
                                        position += length;
                                    } else {
                                        break; // Malformed ALPN parameter
                                    }
                                }
                                https_json["alpn"] = serde_json::json!(alpn_protocols);
                            }
                            // TODO: Add other parameters as needed.
                            Some(https_json)
                        } else {
                            None
                        }
                    })
                    .collect();

                if https_records.is_empty() {
                    return create_response_vector(true, "No HTTPS records found".to_string());
                }

                // Create JSON response
                let json_obj = json!({
                    "public_key": public_key.to_string(),
                    "https_records": https_records,
                    "last_seen": signed_packet.last_seen(),
                    "timestamp": signed_packet.timestamp(),
                });

                let json_str = match serde_json::to_string(&json_obj) {
                    Ok(json) => json,
                    Err(e) => {
                        return create_response_vector(
                            true,
                            format!("Failed to serialize JSON: {}", e),
                        )
                    }
                };

                create_response_vector(false, json_str)
            }
            None => create_response_vector(true, "No signed packet found".to_string()),
        }
    })
}

#[uniffi::export]
pub fn get_signup_token(homeserver_pubky: String, admin_password: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();

        let response = match client
            .get(&format!(
                "https://{homeserver_pubky}/admin/generate_signup_token"
            ))
            .header("X-Admin-Password", admin_password)
            .send()
            .await
        {
            Ok(res) => res,
            Err(error) => {
                return create_response_vector(
                    true,
                    format!("Failed to get signup token: {}", error),
                )
            }
        };

        match response.text().await {
            Ok(signup_token) => create_response_vector(false, signup_token),
            Err(error) => {
                create_response_vector(true, format!("Failed to read signup token: {}", error))
            }
        }
    })
}

#[uniffi::export]
pub fn sign_up(
    secret_key: String,
    homeserver: String,
    signup_token: Option<String>,
) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();
        let keypair = match get_keypair_from_secret_key(&secret_key) {
            Ok(keypair) => keypair,
            Err(error) => return create_response_vector(true, error),
        };

        let homeserver_public_key = match PublicKey::try_from(homeserver) {
            Ok(key) => key,
            Err(error) => {
                return create_response_vector(
                    true,
                    format!("Invalid homeserver public key: {}", error),
                )
            }
        };

        match client
            .signup(&keypair, &homeserver_public_key, signup_token.as_deref())
            .await
        {
            Ok(session) => create_response_vector(false, session_to_json(&session)),
            Err(error) => create_response_vector(true, format!("signup failure: {}", error)),
        }
    })
}

#[uniffi::export]
pub fn republish_homeserver(secret_key: String, homeserver: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();
        let keypair = match get_keypair_from_secret_key(&secret_key) {
            Ok(keypair) => keypair,
            Err(error) => return create_response_vector(true, error),
        };

        let homeserver_public_key = match PublicKey::try_from(homeserver) {
            Ok(key) => key,
            Err(error) => {
                return create_response_vector(
                    true,
                    format!("Invalid homeserver public key: {}", error),
                )
            }
        };

        match client
            .republish_homeserver(&keypair, &homeserver_public_key)
            .await
        {
            Ok(_) => {
                create_response_vector(false, "Homeserver republished successfully".to_string())
            }
            Err(error) => {
                create_response_vector(true, format!("Failed to republish homeserver: {}", error))
            }
        }
    })
}

#[uniffi::export]
pub fn sign_in(secret_key: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();
        let keypair = match get_keypair_from_secret_key(&secret_key) {
            Ok(keypair) => keypair,
            Err(error) => return create_response_vector(true, error),
        };
        match client.signin(&keypair).await {
            Ok(session) => create_response_vector(false, session_to_json(&session)),
            Err(error) => create_response_vector(true, format!("Failed to sign in: {}", error)),
        }
    })
}

#[uniffi::export]
pub fn sign_out(secret_key: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();
        let keypair = match get_keypair_from_secret_key(&secret_key) {
            Ok(keypair) => keypair,
            Err(error) => return create_response_vector(true, error),
        };
        match client.signout(&keypair.public_key()).await {
            Ok(_) => create_response_vector(false, "Sign out success".to_string()),
            Err(error) => create_response_vector(true, format!("Failed to sign out: {}", error)),
        }
    })
}

#[uniffi::export]
pub fn put(url: String, content: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    let content_bytes = content.into_bytes();
    runtime.block_on(async {
        let client = get_pubky_client();
        let trimmed_url = url.trim_end_matches('/');
        let parsed_url = match Url::parse(&trimmed_url) {
            Ok(url) => url,
            Err(_) => return create_response_vector(true, "Failed to parse URL".to_string()),
        };
        match client.put(parsed_url).body(content_bytes).send().await {
            Ok(_) => create_response_vector(false, trimmed_url.to_string()),
            Err(error) => create_response_vector(true, format!("Failed to put: {}", error)),
        }
    })
}

#[uniffi::export]
pub fn get(url: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();
        let trimmed_url = url.trim_end_matches('/');
        let parsed_url = match Url::parse(&trimmed_url) {
            Ok(url) => url,
            Err(_) => return create_response_vector(true, "Failed to parse URL".to_string()),
        };
        let response = match client.get(parsed_url).send().await {
            Ok(res) => res,
            Err(_) => return create_response_vector(true, "Request failed".to_string()),
        };
        if !response.status().is_success() {
            return create_response_vector(true, format!("Request failed: {}", response.status()));
        }
        let bytes = match response.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return create_response_vector(true, format!("Error reading response: {}", e))
            }
        };
        match str::from_utf8(&bytes) {
            Ok(s) => create_response_vector(false, s.to_string()),
            Err(_) => {
                let base64 = base64::encode(&bytes);
                create_response_vector(false, format!("base64:{}", base64))
            }
        }
    })
}

/**
* Resolve a signed packet from a public key
* @param public_key The public key to resolve
* @returns A vector with two elements: the first element is a boolean indicating success or failure,
* and the second element is the response data (either an error message or the resolved signed packet)
**/
#[uniffi::export]
pub fn resolve(public_key: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let public_key = match public_key.as_str().try_into() {
            Ok(key) => key,
            Err(e) => {
                return create_response_vector(true, format!("Invalid zbase32 encoded key: {}", e))
            }
        };
        let client = get_pubky_client();

        match client.pkarr().resolve(&public_key).await {
            Some(signed_packet) => {
                let all_records: Vec<_> = signed_packet.all_resource_records().collect();
                // Convert each ResourceRecord to a JSON value, handling errors appropriately
                let json_records: Vec<serde_json::Value> = all_records
                    .iter()
                    .filter_map(|record| match resource_record_to_json(record) {
                        Ok(json_value) => Some(json_value),
                        Err(e) => {
                            eprintln!("Error converting record to JSON: {}", e);
                            None
                        }
                    })
                    .collect();

                let bytes = signed_packet.as_bytes();
                let public_key = &bytes[..32];
                let signature = &bytes[32..96];
                let timestamp = signed_packet.timestamp();
                let dns_packet = &bytes[104..];
                let hex: String = signed_packet.encode_hex();

                let json_obj = json!({
                    "signed_packet": hex,
                    "public_key": general_purpose::STANDARD.encode(public_key),
                    "signature": general_purpose::STANDARD.encode(signature),
                    "timestamp": timestamp,
                    "last_seen": signed_packet.last_seen(),
                    "dns_packet": general_purpose::STANDARD.encode(dns_packet),
                    "records": json_records
                });

                let json_str = serde_json::to_string(&json_obj)
                    .expect("Failed to convert JSON object to string");

                create_response_vector(false, json_str)
            }
            None => create_response_vector(true, "No signed packet found".to_string()),
        }
    })
}

#[uniffi::export]
pub fn publish(record_name: String, record_content: String, secret_key: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();

        let keypair = match get_keypair_from_secret_key(&secret_key) {
            Ok(keypair) => keypair,
            Err(error) => return create_response_vector(true, error),
        };

        let mut packet = dns::Packet::new_reply(0);

        let dns_name = match dns::Name::new(&record_name) {
            Ok(name) => name,
            Err(e) => {
                return create_response_vector(true, format!("Failed to create DNS name: {}", e))
            }
        };

        let record_content_str: &str = record_content.as_str();

        let txt_record = match record_content_str.try_into() {
            Ok(value) => RData::TXT(value),
            Err(e) => {
                return create_response_vector(
                    true,
                    format!("Failed to convert string to TXT record: {}", e),
                )
            }
        };

        packet.answers.push(dns::ResourceRecord::new(
            dns_name,
            dns::CLASS::IN,
            30,
            txt_record,
        ));

        match SignedPacket::new(&keypair, &packet.answers, Timestamp::now()) {
            Ok(signed_packet) => {
                match client
                    .pkarr()
                    .publish(&signed_packet, Some(Timestamp::now()))
                    .await
                {
                    Ok(()) => create_response_vector(false, keypair.public_key().to_string()),
                    Err(e) => create_response_vector(true, format!("Failed to publish: {}", e)),
                }
            }
            Err(e) => {
                create_response_vector(true, format!("Failed to create signed packet: {}", e))
            }
        }
    })
}
#[uniffi::export]
pub fn list(url: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();
        let trimmed_url = url.trim_end_matches('/');
        let parsed_url = match Url::parse(&trimmed_url) {
            Ok(url) => url,
            Err(_) => return create_response_vector(true, "Failed to parse URL".to_string()),
        };
        let list_builder = match client.list(parsed_url) {
            Ok(list) => list,
            Err(error) => {
                return create_response_vector(true, format!("Failed to list: {}", error))
            }
        };
        // Execute the non-Send part synchronously
        let send_future = list_builder.send();
        let send_res = match send_future.await {
            Ok(res) => res,
            Err(error) => {
                return create_response_vector(
                    true,
                    format!("Failed to send list request: {}", error),
                )
            }
        };
        let json_string = match serde_json::to_string(&send_res) {
            Ok(json) => json,
            Err(error) => {
                return create_response_vector(true, format!("Failed to serialize JSON: {}", error))
            }
        };
        create_response_vector(false, json_string)
    })
}

#[uniffi::export]
pub fn auth(url: String, secret_key: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(authorize(url, secret_key))
}

#[uniffi::export]
pub fn parse_auth_url(url: String) -> Vec<String> {
    let parsed_details = match parse_pubky_auth_url(&url) {
        Ok(details) => details,
        Err(error) => return create_response_vector(true, error),
    };
    match pubky_auth_details_to_json(&parsed_details) {
        Ok(json) => create_response_vector(false, json),
        Err(error) => create_response_vector(true, error),
    }
}

#[uniffi::export]
pub fn create_recovery_file(secret_key: String, passphrase: String) -> Vec<String> {
    if secret_key.is_empty() || passphrase.is_empty() {
        return create_response_vector(
            true,
            "Secret key and passphrase must not be empty".to_string(),
        );
    }
    let keypair = match get_keypair_from_secret_key(&secret_key) {
        Ok(keypair) => keypair,
        Err(error) => return create_response_vector(true, error),
    };
    let recovery_file_bytes = recovery_file::create_recovery_file(&keypair, &passphrase);
    let recovery_file = base64::encode(&recovery_file_bytes);
    create_response_vector(false, recovery_file)
}

#[uniffi::export]
pub fn decrypt_recovery_file(recovery_file: String, passphrase: String) -> Vec<String> {
    if recovery_file.is_empty() || passphrase.is_empty() {
        return create_response_vector(
            true,
            "Recovery file and passphrase must not be empty".to_string(),
        );
    }
    let recovery_file_bytes = match base64::decode(&recovery_file) {
        Ok(bytes) => bytes,
        Err(error) => {
            return create_response_vector(
                true,
                format!("Failed to decode recovery file: {}", error),
            )
        }
    };
    let keypair = match recovery_file::decrypt_recovery_file(&recovery_file_bytes, &passphrase) {
        Ok(keypair) => keypair,
        Err(_) => {
            return create_response_vector(true, "Failed to decrypt recovery file".to_string())
        }
    };
    let secret_key = get_secret_key_from_keypair(&keypair);
    create_response_vector(false, secret_key)
}

#[uniffi::export]
pub fn get_homeserver(pubky: String) -> Vec<String> {
    let runtime = TOKIO_RUNTIME.clone();
    runtime.block_on(async {
        let client = get_pubky_client();
        let public_key = match PublicKey::try_from(pubky) {
            Ok(key) => key,
            Err(error) => {
                return create_response_vector(true, format!("Invalid public key: {}", error))
            }
        };

        match client.get_homeserver(&public_key).await {
            Some(homeserver) => create_response_vector(false, homeserver),
            None => {
                create_response_vector(true, "No homeserver found for this public key".to_string())
            }
        }
    })
}

#[uniffi::export]
pub fn generate_mnemonic_phrase() -> Vec<String> {
    match generate_mnemonic() {
        Ok(mnemonic) => create_response_vector(false, mnemonic),
        Err(error) => create_response_vector(true, error),
    }
}

#[uniffi::export]
pub fn mnemonic_phrase_to_keypair(mnemonic_phrase: String) -> Vec<String> {
    match mnemonic_to_keypair(&mnemonic_phrase) {
        Ok(keypair) => match keypair_to_json_string(&keypair, None) {
            Ok(json_str) => create_response_vector(false, json_str),
            Err(error) => create_response_vector(true, error),
        },
        Err(error) => create_response_vector(true, error),
    }
}

#[uniffi::export]
pub fn generate_mnemonic_phrase_and_keypair() -> Vec<String> {
    match generate_mnemonic_and_keypair() {
        Ok((mnemonic, keypair)) => match keypair_to_json_string(&keypair, Some(&mnemonic)) {
            Ok(json_str) => create_response_vector(false, json_str),
            Err(error) => create_response_vector(true, error),
        },
        Err(error) => create_response_vector(true, error),
    }
}

#[uniffi::export]
pub fn validate_mnemonic_phrase(mnemonic_phrase: String) -> Vec<String> {
    let is_valid = validate_mnemonic(&mnemonic_phrase);
    create_response_vector(false, is_valid.to_string())
}
