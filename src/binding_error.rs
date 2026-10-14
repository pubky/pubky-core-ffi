use pubky::errors::{AuthError, BuildError, PkarrError, RequestError};

/// Structured errors for the additive typed binding APIs.
///
/// Legacy functions keep returning `[error, data]` string vectors for backwards
/// compatibility. New APIs return `Result<T, PubkyCoreError>` so native callers
/// can branch on an error category and, for server errors, the HTTP status.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PubkyCoreError {
    #[error("HTTP transport error: {details}")]
    Transport { details: String },

    #[error("Server responded with {status}: {details}")]
    Server { status: u16, details: String },

    #[error("Invalid request: {details}")]
    Validation { details: String },

    #[error("Failed to decode response JSON: {details}")]
    DecodeJson { details: String },

    #[error("PKARR operation failed: {details}")]
    Pkarr { details: String, retryable: bool },

    #[error("Failed to parse URL: {details}")]
    Parse { details: String },

    #[error("Authentication failed: {details}")]
    Authentication { details: String, expired: bool },

    #[error("Client construction failed: {details}")]
    Build { details: String },

    #[error("Binding state error: {details}")]
    State { details: String },
}

impl From<pubky::Error> for PubkyCoreError {
    fn from(error: pubky::Error) -> Self {
        match error {
            pubky::Error::Request(error) => error.into(),
            pubky::Error::Pkarr(error) => error.into(),
            pubky::Error::Parse(error) => Self::Parse {
                details: error.to_string(),
            },
            pubky::Error::Authentication(error) => error.into(),
            pubky::Error::Build(error) => error.into(),
        }
    }
}

impl From<RequestError> for PubkyCoreError {
    fn from(error: RequestError) -> Self {
        match error {
            RequestError::Transport(error) => Self::Transport {
                details: error.to_string(),
            },
            RequestError::Server { status, message } => Self::Server {
                status: status.as_u16(),
                details: message,
            },
            RequestError::Validation { message } => Self::Validation { details: message },
            RequestError::DecodeJson { message } => Self::DecodeJson { details: message },
        }
    }
}

impl From<PkarrError> for PubkyCoreError {
    fn from(error: PkarrError) -> Self {
        let retryable = error.is_retryable();
        Self::Pkarr {
            details: error.to_string(),
            retryable,
        }
    }
}

impl From<AuthError> for PubkyCoreError {
    fn from(error: AuthError) -> Self {
        let expired = matches!(error, AuthError::RequestExpired);
        Self::Authentication {
            details: error.to_string(),
            expired,
        }
    }
}

impl From<BuildError> for PubkyCoreError {
    fn from(error: BuildError) -> Self {
        Self::Build {
            details: error.to_string(),
        }
    }
}

impl From<reqwest::Error> for PubkyCoreError {
    fn from(error: reqwest::Error) -> Self {
        Self::Transport {
            details: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for PubkyCoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::DecodeJson {
            details: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubky::StatusCode;

    #[test]
    fn preserves_server_status_and_message() {
        let error = PubkyCoreError::from(RequestError::Server {
            status: StatusCode::FORBIDDEN,
            message: "missing capability".to_string(),
        });

        match error {
            PubkyCoreError::Server { status, details } => {
                assert_eq!(status, 403);
                assert_eq!(details, "missing capability");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn marks_expired_auth_requests() {
        let error = PubkyCoreError::from(AuthError::RequestExpired);
        assert!(matches!(
            error,
            PubkyCoreError::Authentication { expired: true, .. }
        ));
    }
}
