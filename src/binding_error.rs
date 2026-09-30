use pubky::errors::{AuthError, BuildError, PkarrError, RequestError};

/// Structured errors for the additive typed binding APIs.
///
/// Legacy functions keep returning `[error, data]` string vectors for backwards
/// compatibility. New APIs return `Result<T, PubkyCoreError>` so native callers
/// can branch on an error category and, for server errors, the HTTP status.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PubkyCoreError {
    #[error("HTTP transport error: {message}")]
    Transport { message: String },

    #[error("Server responded with {status}: {message}")]
    Server { status: u16, message: String },

    #[error("Invalid request: {message}")]
    Validation { message: String },

    #[error("Failed to decode response JSON: {message}")]
    DecodeJson { message: String },

    #[error("PKARR operation failed: {message}")]
    Pkarr { message: String, retryable: bool },

    #[error("Failed to parse URL: {message}")]
    Parse { message: String },

    #[error("Authentication failed: {message}")]
    Authentication { message: String, expired: bool },

    #[error("Client construction failed: {message}")]
    Build { message: String },

    #[error("Binding state error: {message}")]
    State { message: String },
}

impl From<pubky::Error> for PubkyCoreError {
    fn from(error: pubky::Error) -> Self {
        match error {
            pubky::Error::Request(error) => error.into(),
            pubky::Error::Pkarr(error) => error.into(),
            pubky::Error::Parse(error) => Self::Parse {
                message: error.to_string(),
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
                message: error.to_string(),
            },
            RequestError::Server { status, message } => Self::Server {
                status: status.as_u16(),
                message,
            },
            RequestError::Validation { message } => Self::Validation { message },
            RequestError::DecodeJson { message } => Self::DecodeJson { message },
        }
    }
}

impl From<PkarrError> for PubkyCoreError {
    fn from(error: PkarrError) -> Self {
        let retryable = error.is_retryable();
        Self::Pkarr {
            message: error.to_string(),
            retryable,
        }
    }
}

impl From<AuthError> for PubkyCoreError {
    fn from(error: AuthError) -> Self {
        let expired = matches!(error, AuthError::RequestExpired);
        Self::Authentication {
            message: error.to_string(),
            expired,
        }
    }
}

impl From<BuildError> for PubkyCoreError {
    fn from(error: BuildError) -> Self {
        Self::Build {
            message: error.to_string(),
        }
    }
}

impl From<reqwest::Error> for PubkyCoreError {
    fn from(error: reqwest::Error) -> Self {
        Self::Transport {
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for PubkyCoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::DecodeJson {
            message: error.to_string(),
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
            PubkyCoreError::Server { status, message } => {
                assert_eq!(status, 403);
                assert_eq!(message, "missing capability");
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
