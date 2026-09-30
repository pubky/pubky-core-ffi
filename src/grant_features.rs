use crate::{
    get_pubky_http_client, Capability, CustomGrantProof, PubkyCoreError, VerifiedCustomGrantProof,
    TOKIO_RUNTIME,
};
use pubky::custom_pop::{verify_custom_grant_pop, CustomPop};
use pubky::{GrantCredential, PubkySession, StoragePath};
use std::time::Duration;

fn validation_error(details: impl ToString) -> PubkyCoreError {
    PubkyCoreError::Validation {
        details: details.to_string(),
    }
}

fn capability_record(capability: &pubky::Capability) -> Capability {
    let capability = capability.to_string();
    let (path, permission) = capability
        .rsplit_once(':')
        .unwrap_or((capability.as_str(), ""));
    Capability {
        path: path.to_string(),
        permission: permission.to_string(),
    }
}

fn local_grant_session(secret: &str) -> Result<PubkySession, PubkyCoreError> {
    let credential = GrantCredential::from_shared_secret(secret)?;
    Ok(PubkySession::from_grant_credential(
        get_pubky_http_client(),
        credential,
    ))
}

/// Create an application-defined proof using a Grant session's proof-of-possession key.
/// This operation is local and does not contact the homeserver.
#[uniffi::export]
pub fn create_custom_grant_proof(
    session_secret: String,
    data_json: String,
) -> Result<CustomGrantProof, PubkyCoreError> {
    let data = serde_json::from_str(&data_json).map_err(validation_error)?;
    let session = local_grant_session(&session_secret)?;
    let grant = session.as_grant().ok_or_else(|| PubkyCoreError::State {
        details: "Session is not grant-backed".to_string(),
    })?;

    TOKIO_RUNTIME.block_on(async {
        let proof = grant.create_custom_pop(data).await.map_err(|error| {
            PubkyCoreError::Authentication {
                expired: matches!(error, pubky::custom_pop::CustomPopError::GrantExpired),
                details: error.to_string(),
            }
        })?;
        Ok(CustomGrantProof {
            grant: proof.grant,
            proof: proof.pop,
        })
    })
}

/// Verify a custom Grant proof without contacting a homeserver.
/// The caller remains responsible for application-level freshness and replay checks.
#[uniffi::export]
pub fn verify_custom_grant_proof(
    proof: CustomGrantProof,
    clock_skew_seconds: u64,
) -> Result<VerifiedCustomGrantProof, PubkyCoreError> {
    let proof = CustomPop {
        grant: proof.grant,
        pop: proof.proof,
    };
    let verified = verify_custom_grant_pop(&proof, Duration::from_secs(clock_skew_seconds))
        .map_err(validation_error)?;
    let claims = verified.grant_claims();

    Ok(VerifiedCustomGrantProof {
        identity: verified.identity().z32(),
        client_id: claims.client_id.to_string(),
        client_public_key: claims.cnf.z32(),
        grant_id: claims.jti.to_string(),
        capabilities: claims.caps.iter().map(capability_record).collect(),
        grant_issued_at: claims.iat,
        grant_expires_at: claims.exp,
        proof_issued_at: verified.iat(),
        nonce: verified.nonce().to_string(),
        data_json: serde_json::to_string(verified.data())?,
    })
}

/// Return the approved encryption-key scopes stored in a Grant credential.
/// `None` identifies a legacy bare Grant; an empty vector is a signed approval with no keys.
#[uniffi::export]
pub fn grant_encryption_key_scopes(
    session_secret: String,
) -> Result<Option<Vec<String>>, PubkyCoreError> {
    Ok(GrantCredential::restore_encryption_keys(&session_secret)?
        .map(|(_, keys)| keys.scopes().map(ToString::to_string).collect()))
}

/// Derive a 32-byte content key for a file covered by an approved encryption-key scope.
/// `None` identifies a legacy bare Grant that has no signed approval.
#[uniffi::export]
pub fn derive_grant_encryption_key(
    session_secret: String,
    path: String,
) -> Result<Option<Vec<u8>>, PubkyCoreError> {
    let path = StoragePath::new(&path).map_err(validation_error)?;
    let Some((_, keys)) = GrantCredential::restore_encryption_keys(&session_secret)? else {
        return Ok(None);
    };
    let key = keys.derive_for_path(&path).map_err(validation_error)?;
    Ok(Some(key.to_vec()))
}

/// Return the confidential signed approval embedded in a V2 Grant credential.
#[uniffi::export]
pub fn grant_signed_approval(session_secret: String) -> Result<Option<String>, PubkyCoreError> {
    let credential = GrantCredential::from_shared_secret(&session_secret)?;
    Ok(credential.signed_approval().map(ToString::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_json_custom_proof_data_before_using_credentials() {
        let error = create_custom_grant_proof("not-a-credential".into(), "not-json".into())
            .expect_err("invalid JSON must fail");
        assert!(matches!(error, PubkyCoreError::Validation { .. }));
    }
}
