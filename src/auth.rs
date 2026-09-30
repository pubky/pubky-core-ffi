// Note: The authorize function has been moved inline into the auth() function in lib.rs
// using the new approve_auth API from PubkySigner

use crate::{Capability, PubkyAuthDetails, PubkyDeepLinkDetails};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use pubky::deep_links::{DeepLink, GrantApprovalFormat, GrantRelayChannel, XCallbackParams};
use pubky::Capabilities;
use serde_json;
use std::str::FromStr;

pub fn pubky_auth_details_to_json(details: &PubkyAuthDetails) -> Result<String, String> {
    serde_json::to_string(details).map_err(|_| "Error serializing to JSON".to_string())
}

pub fn pubky_deep_link_details_to_json(details: &PubkyDeepLinkDetails) -> Result<String, String> {
    serde_json::to_string(details).map_err(|_| "Error serializing to JSON".to_string())
}

pub fn parse_pubky_auth_url(url_str: &str) -> Result<PubkyAuthDetails, String> {
    let details = parse_pubky_deep_link(url_str)?;
    match details.kind.as_str() {
        "signin" | "signup" | "signin_grant" | "signup_grant" => Ok(PubkyAuthDetails {
            relay: details.relay.ok_or_else(|| "Missing relay".to_string())?,
            capabilities: details.capabilities.unwrap_or_default(),
            secret: details.secret,
            kind: details.kind,
            homeserver: details.homeserver,
            signup_token: details.signup_token,
            client_id: details.client_id,
            client_public_key: details.client_public_key,
            approval_format: details.approval_format,
            ephemeral_public_key: details.ephemeral_public_key,
            x_source: details.x_source,
            x_success: details.x_success,
            x_error: details.x_error,
            x_cancel: details.x_cancel,
        }),
        other => Err(format!("Invalid auth URL intent '{}'", other)),
    }
}

pub fn parse_pubky_deep_link(url_str: &str) -> Result<PubkyDeepLinkDetails, String> {
    let deep_link = DeepLink::from_str(url_str).map_err(|error| error.to_string())?;
    Ok(deep_link_details(&deep_link))
}

fn deep_link_details(deep_link: &DeepLink) -> PubkyDeepLinkDetails {
    let callbacks = deep_link.x_callback();
    match deep_link {
        DeepLink::Signin(link) => {
            let params = link.params();
            auth_deep_link_details(
                link.scheme().as_str(),
                link.intent(),
                link.to_string(),
                params.relay.to_string(),
                &params.capabilities,
                Some(params.secret),
                None,
                None,
                None,
                None,
                None,
                None,
                callbacks,
            )
        }
        DeepLink::Signup(link) => {
            let params = link.params();
            auth_deep_link_details(
                link.scheme().as_str(),
                link.intent(),
                link.to_string(),
                params.relay.to_string(),
                &params.capabilities,
                Some(params.secret),
                Some(params.homeserver.z32()),
                params.signup_token.clone(),
                None,
                None,
                None,
                None,
                callbacks,
            )
        }
        DeepLink::DirectSignup(link) => {
            let params = link.params();
            non_auth_deep_link_details(
                link.scheme().as_str(),
                link.intent(),
                link.to_string(),
                Some(params.homeserver.z32()),
                params.signup_token.clone(),
                None,
                None,
                None,
                callbacks,
            )
        }
        DeepLink::SigninGrant(link) => {
            let params = link.params();
            auth_deep_link_details(
                link.scheme().as_str(),
                link.intent(),
                link.to_string(),
                params.relay.to_string(),
                &params.capabilities,
                grant_shared_secret(params.relay_channel),
                None,
                None,
                Some(params.client_id.to_string()),
                Some(params.client_pk.z32()),
                Some(grant_approval_format(params.approval_format)),
                grant_ephemeral_public_key(params.relay_channel),
                callbacks,
            )
        }
        DeepLink::SignupGrant(link) => {
            let params = link.params();
            auth_deep_link_details(
                link.scheme().as_str(),
                link.intent(),
                link.to_string(),
                params.relay.to_string(),
                &params.capabilities,
                grant_shared_secret(params.relay_channel),
                Some(params.homeserver.z32()),
                params.signup_token.clone(),
                Some(params.client_id.to_string()),
                Some(params.client_pk.z32()),
                Some(grant_approval_format(params.approval_format)),
                grant_ephemeral_public_key(params.relay_channel),
                callbacks,
            )
        }
        DeepLink::SeedExport(link) => {
            let params = link.params();
            non_auth_deep_link_details(
                link.scheme().as_str(),
                link.intent(),
                link.to_string(),
                None,
                None,
                Some(URL_SAFE_NO_PAD.encode(params.secret)),
                None,
                None,
                callbacks,
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn auth_deep_link_details(
    scheme: &str,
    kind: &str,
    url: String,
    relay: String,
    capabilities: &Capabilities,
    secret: Option<[u8; 32]>,
    homeserver: Option<String>,
    signup_token: Option<String>,
    client_id: Option<String>,
    client_public_key: Option<String>,
    approval_format: Option<String>,
    ephemeral_public_key: Option<String>,
    callbacks: &XCallbackParams,
) -> PubkyDeepLinkDetails {
    PubkyDeepLinkDetails {
        scheme: scheme.to_string(),
        kind: kind.to_string(),
        url,
        relay: Some(relay),
        capabilities: Some(convert_capabilities(capabilities)),
        secret: secret.map(|secret| URL_SAFE_NO_PAD.encode(secret)),
        homeserver,
        signup_token,
        client_id,
        client_public_key,
        approval_format,
        ephemeral_public_key,
        x_source: callbacks.x_source.clone(),
        x_success: callbacks.x_success.clone(),
        x_error: callbacks.x_error.clone(),
        x_cancel: callbacks.x_cancel.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn non_auth_deep_link_details(
    scheme: &str,
    kind: &str,
    url: String,
    homeserver: Option<String>,
    signup_token: Option<String>,
    secret: Option<String>,
    client_id: Option<String>,
    client_public_key: Option<String>,
    callbacks: &XCallbackParams,
) -> PubkyDeepLinkDetails {
    PubkyDeepLinkDetails {
        scheme: scheme.to_string(),
        kind: kind.to_string(),
        url,
        relay: None,
        capabilities: None,
        secret,
        homeserver,
        signup_token,
        client_id,
        client_public_key,
        approval_format: None,
        ephemeral_public_key: None,
        x_source: callbacks.x_source.clone(),
        x_success: callbacks.x_success.clone(),
        x_error: callbacks.x_error.clone(),
        x_cancel: callbacks.x_cancel.clone(),
    }
}

fn grant_shared_secret(channel: GrantRelayChannel) -> Option<[u8; 32]> {
    match channel {
        GrantRelayChannel::SharedSecret(secret) => Some(secret),
        GrantRelayChannel::Hpke { .. } => None,
    }
}

fn grant_ephemeral_public_key(channel: GrantRelayChannel) -> Option<String> {
    match channel {
        GrantRelayChannel::SharedSecret(_) => None,
        GrantRelayChannel::Hpke {
            ephemeral_public_key,
        } => Some(URL_SAFE_NO_PAD.encode(ephemeral_public_key)),
    }
}

fn grant_approval_format(format: GrantApprovalFormat) -> String {
    match format {
        GrantApprovalFormat::BareGrant => "bare_grant",
        GrantApprovalFormat::SignedApprovalV1 => "signed_approval_v1",
    }
    .to_string()
}

fn convert_capabilities(capabilities: &Capabilities) -> Vec<Capability> {
    capabilities
        .iter()
        .map(|capability| {
            let capability = capability.to_string();
            let (path, permission) = capability
                .rsplit_once(':')
                .unwrap_or((capability.as_str(), ""));
            Capability {
                path: path.to_string(),
                permission: permission.to_string(),
            }
        })
        .collect()
}
