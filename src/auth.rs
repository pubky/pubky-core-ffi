// Note: The authorize function has been moved inline into the auth() function in lib.rs
// using the new approve_auth API from PubkySigner

use crate::{Capability, PubkyAuthDetails};
use serde_json;
use std::collections::HashMap;
use url::Url;

pub fn pubky_auth_details_to_json(details: &PubkyAuthDetails) -> Result<String, String> {
    serde_json::to_string(details).map_err(|_| "Error serializing to JSON".to_string())
}

pub fn parse_pubky_auth_url(url_str: &str) -> Result<PubkyAuthDetails, String> {
    let url = Url::parse(url_str).map_err(|_| "Invalid URL".to_string())?;

    if url.scheme() != "pubkyauth" {
        return Err("Invalid scheme, expected 'pubkyauth'".to_string());
    }

    // Collect query pairs into a HashMap for efficient access
    let query_params: HashMap<_, _> = url.query_pairs().into_owned().collect();

    let relay = query_params
        .get("relay")
        .cloned()
        .ok_or_else(|| "Missing relay".to_string())?;

    let secret = query_params
        .get("secret")
        .cloned()
        .ok_or_else(|| "Missing secret".to_string())?;

    let capabilities_str = query_params
        .get("capabilities")
        .or_else(|| query_params.get("caps"))
        .cloned()
        .unwrap_or_default();

    // Parse capabilities
    let capabilities = if capabilities_str.is_empty() {
        Vec::new()
    } else {
        capabilities_str
            .split(',')
            .map(|capability| {
                let mut parts = capability.splitn(2, ':');
                let path = parts
                    .next()
                    .ok_or_else(|| format!("Invalid capability format in '{}'", capability))?;
                let permission = parts
                    .next()
                    .ok_or_else(|| format!("Invalid capability format in '{}'", capability))?;
                Ok(Capability {
                    path: path.to_string(),
                    permission: permission.to_string(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?
    };

    Ok(PubkyAuthDetails {
        relay,
        capabilities,
        secret,
    })
}
