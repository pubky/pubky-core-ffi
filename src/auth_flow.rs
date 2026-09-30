use crate::{
    export_cookie_session_secret, export_grant_session_secret, get_keypair_from_secret_key,
    get_pubky_http_client, parse_client_id, session_to_json_with_cookie_secret,
    session_to_json_with_grant_secret, GrantAuthFlowConfig, GrantAuthFlowStateRecord,
    PubkyCoreError, GRANT_AUTH_FLOW, TOKIO_RUNTIME,
};
use pubky::deep_links::XCallbackParams;
use pubky::{
    AuthFlowKind, Capabilities, GrantAuthFlowState, Keypair, PubkyGrantAuthFlow, PublicKey,
};
use url::Url;

fn bytes_32(bytes: Vec<u8>, name: &str) -> Result<[u8; 32], PubkyCoreError> {
    bytes.try_into().map_err(|_| PubkyCoreError::Validation {
        message: format!("{name} must contain exactly 32 bytes"),
    })
}

fn state_record(flow: &PubkyGrantAuthFlow) -> Result<GrantAuthFlowStateRecord, PubkyCoreError> {
    let state = flow.save_local().ok_or_else(|| PubkyCoreError::State {
        message: "Grant flow does not use an exportable local signer".to_string(),
    })?;
    Ok(GrantAuthFlowStateRecord {
        authorization_url: state.authorization_url,
        client_key_secret: state.client_key_secret.to_vec(),
    })
}

async fn session_json(session: &pubky::PubkySession) -> Result<String, PubkyCoreError> {
    let secret = export_grant_session_secret(session)
        .await
        .map_err(|message| PubkyCoreError::State { message })?;
    session_to_json_with_grant_secret(session, &secret)
        .await
        .map_err(|message| PubkyCoreError::State { message })
}

#[uniffi::export]
pub fn start_grant_auth_flow_with_config(
    config: GrantAuthFlowConfig,
) -> Result<GrantAuthFlowStateRecord, PubkyCoreError> {
    let caps = Capabilities::try_from(config.capabilities.as_str()).map_err(|error| {
        PubkyCoreError::Validation {
            message: format!("invalid capabilities: {error}"),
        }
    })?;
    let client_id = parse_client_id(&config.client_id)
        .map_err(|message| PubkyCoreError::Validation { message })?;
    let auth_kind = match config.homeserver.as_deref() {
        Some(homeserver) => AuthFlowKind::signup(
            PublicKey::try_from(homeserver).map_err(|error| PubkyCoreError::Validation {
                message: format!("invalid homeserver public key: {error}"),
            })?,
            config.signup_token,
        ),
        None => AuthFlowKind::signin(),
    };
    let relay = config
        .relay
        .as_deref()
        .map(Url::parse)
        .transpose()
        .map_err(|error| PubkyCoreError::Parse {
            message: error.to_string(),
        })?;
    let client_secret = config
        .client_secret
        .map(|secret| bytes_32(secret, "client_secret"))
        .transpose()?;
    let client_keypair = config
        .client_key_secret
        .map(|secret| {
            bytes_32(secret, "client_key_secret").map(|secret| Keypair::from_secret(&secret))
        })
        .transpose()?;

    let mut builder =
        PubkyGrantAuthFlow::builder(&caps, auth_kind, client_id).client(get_pubky_http_client());
    if let Some(relay) = relay {
        builder = builder.relay(relay);
    }
    if let Some(secret) = client_secret {
        builder = builder.client_secret(secret);
    }
    if let Some(keypair) = client_keypair {
        builder = builder.client_keypair(keypair);
    }
    builder = builder.x_callback(XCallbackParams {
        x_source: config.x_source,
        x_success: config.x_success,
        x_error: config.x_error,
        x_cancel: config.x_cancel,
    });

    let flow = builder.start()?;
    let state = state_record(&flow)?;
    *GRANT_AUTH_FLOW.lock().unwrap() = Some(flow);
    Ok(state)
}

#[uniffi::export]
pub fn save_grant_auth_flow() -> Result<GrantAuthFlowStateRecord, PubkyCoreError> {
    let guard = GRANT_AUTH_FLOW.lock().unwrap();
    let flow = guard.as_ref().ok_or_else(|| PubkyCoreError::State {
        message: "No Grant auth flow is in progress".to_string(),
    })?;
    state_record(flow)
}

#[uniffi::export]
pub fn restore_grant_auth_flow(state: GrantAuthFlowStateRecord) -> Result<String, PubkyCoreError> {
    let state = GrantAuthFlowState {
        authorization_url: state.authorization_url,
        client_key_secret: bytes_32(state.client_key_secret, "client_key_secret")?,
    };
    let flow = PubkyGrantAuthFlow::restore(state, get_pubky_http_client())?;
    let authorization_url = flow.authorization_url().to_string();
    *GRANT_AUTH_FLOW.lock().unwrap() = Some(flow);
    Ok(authorization_url)
}

/// Probe a pending Grant flow once without blocking the UI thread.
#[uniffi::export]
pub fn poll_grant_auth_flow() -> Result<Option<String>, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let flow = GRANT_AUTH_FLOW
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| PubkyCoreError::State {
                message: "No Grant auth flow is in progress".to_string(),
            })?;

        match flow.try_poll_once().await {
            Ok(Some(session)) => {
                let json = session_json(&session).await?;
                Ok(Some(json))
            }
            Ok(None) => {
                *GRANT_AUTH_FLOW.lock().unwrap() = Some(flow);
                Ok(None)
            }
            Err(error) => {
                *GRANT_AUTH_FLOW.lock().unwrap() = Some(flow);
                Err(error.into())
            }
        }
    })
}

#[uniffi::export]
pub fn await_grant_auth_flow() -> Result<String, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let flow = GRANT_AUTH_FLOW
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| PubkyCoreError::State {
                message: "No Grant auth flow is in progress".to_string(),
            })?;
        let session = flow.await_approval().await?;
        session_json(&session).await
    })
}

#[uniffi::export]
pub fn cancel_grant_auth_flow() {
    *GRANT_AUTH_FLOW.lock().unwrap() = None;
}

#[uniffi::export]
pub fn sign_in_grant_blocking(
    secret_key: String,
    client_id: String,
) -> Result<String, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let keypair = get_keypair_from_secret_key(&secret_key)
            .map_err(|message| PubkyCoreError::Validation { message })?;
        let client_id = parse_client_id(&client_id)
            .map_err(|message| PubkyCoreError::Validation { message })?;
        let session = crate::get_pubky_client()
            .signer(keypair)
            .signin_blocking(client_id)
            .await?;
        session_json(&session).await
    })
}

#[uniffi::export]
#[allow(deprecated)]
pub fn sign_in_cookie_blocking(secret_key: String) -> Result<String, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let keypair = get_keypair_from_secret_key(&secret_key)
            .map_err(|message| PubkyCoreError::Validation { message })?;
        let session = crate::get_pubky_client()
            .signer(keypair)
            .signin_cookie_blocking()
            .await?;
        let secret = export_cookie_session_secret(&session)
            .map_err(|message| PubkyCoreError::State { message })?;
        Ok(session_to_json_with_cookie_secret(&session, &secret))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_fixed_length_flow_secrets() {
        assert!(bytes_32(vec![7; 32], "secret").is_ok());
        assert!(matches!(
            bytes_32(vec![7; 31], "secret"),
            Err(PubkyCoreError::Validation { .. })
        ));
    }

    #[test]
    fn rejects_invalid_config_before_starting_network_work() {
        let result = start_grant_auth_flow_with_config(GrantAuthFlowConfig {
            capabilities: "/pub/example/:rw".to_string(),
            client_id: "example.app".to_string(),
            client_secret: Some(vec![0; 31]),
            ..GrantAuthFlowConfig::default()
        });
        assert!(matches!(result, Err(PubkyCoreError::Validation { .. })));
    }
}
