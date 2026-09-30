use serde::Serialize;

/// Configuration used when rebuilding the process-wide Pubky client.
///
/// All timeout values are milliseconds. `None` preserves Pubky's default for
/// that setting. Reconfiguration affects subsequently created sessions and
/// requests; sessions already restored by a call are not retained globally.
#[derive(Clone, Debug, Default, uniffi::Record)]
pub struct PubkyClientConfig {
    pub use_testnet: bool,
    pub testnet_host: Option<String>,
    pub request_timeout_ms: Option<u64>,
    pub read_timeout_ms: Option<u64>,
    pub pool_max_idle_per_host: Option<u64>,
    pub max_error_body_bytes: Option<u64>,
    pub user_agent_extra: Option<String>,
}

#[derive(Clone, Debug, Default, uniffi::Record)]
pub struct StorageListOptions {
    pub reverse: bool,
    pub shallow: bool,
    pub limit: Option<u16>,
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct StorageListPage {
    pub entries: Vec<String>,
    /// Pass this value as `cursor` to retrieve the next page.
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct StorageResourceStats {
    pub content_length: Option<u64>,
    pub content_type: Option<String>,
    /// Milliseconds since the Unix epoch.
    pub last_modified_ms: Option<u64>,
    pub etag: Option<String>,
}

/// Complete configuration for a local Grant authentication flow.
#[derive(Clone, Debug, Default, uniffi::Record)]
pub struct GrantAuthFlowConfig {
    pub capabilities: String,
    pub client_id: String,
    /// Presence selects a signup flow; absence selects signin.
    pub homeserver: Option<String>,
    pub signup_token: Option<String>,
    pub relay: Option<String>,
    /// Exactly 32 bytes. A random relay secret is used when omitted.
    pub client_secret: Option<Vec<u8>>,
    /// Exactly 32 bytes. A random proof-of-possession key is used when omitted.
    pub client_key_secret: Option<Vec<u8>>,
    pub x_source: Option<String>,
    pub x_success: Option<String>,
    pub x_error: Option<String>,
    pub x_cancel: Option<String>,
}

/// Sensitive, temporary state for resuming a pending local Grant flow.
#[derive(Clone, Debug, uniffi::Record)]
pub struct GrantAuthFlowStateRecord {
    pub authorization_url: String,
    pub client_key_secret: Vec<u8>,
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct EventStreamUser {
    pub public_key: String,
    pub cursor: Option<u64>,
}

#[derive(Clone, Debug, Default, uniffi::Record)]
pub struct EventStreamConfig {
    pub users: Vec<EventStreamUser>,
    pub homeserver: Option<String>,
    pub paths: Vec<String>,
    pub limit: Option<u16>,
    pub max_event_bytes: Option<u64>,
    pub live: bool,
    pub reverse: bool,
    /// Required for private `/priv/...` paths.
    pub session_secret: Option<String>,
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct PubkyStorageEvent {
    /// `PUT` or `DEL`.
    pub event_type: String,
    pub resource: String,
    pub cursor: u64,
    /// Blake3 hash in hexadecimal for `PUT`; absent for `DEL`.
    pub content_hash: Option<String>,
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct StorageLockInfo {
    pub path: String,
    pub token: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct Capability {
    pub path: String,
    pub permission: String,
}

#[derive(Debug, Serialize)]
pub struct PubkyAuthDetails {
    pub relay: String,
    pub capabilities: Vec<Capability>,
    pub secret: String,
    /// Auth URL intent. Legacy URLs (`pubkyauth:///?...`) have no intent host
    /// and are treated as "signin".
    pub kind: String,
    /// Homeserver public key (bare z-base32) from the `hs` param of signup links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homeserver: Option<String>,
    /// Signup token from the `st` param of signup links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signup_token: Option<String>,
    /// Grant client id from the `cid` param of grant auth links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Grant client public key from the `cpk` param of grant auth links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_public_key: Option<String>,
    /// x-callback-url source app name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_source: Option<String>,
    /// x-callback-url success callback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_success: Option<String>,
    /// x-callback-url error callback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_error: Option<String>,
    /// x-callback-url cancel callback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_cancel: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PubkyDeepLinkDetails {
    pub scheme: String,
    pub kind: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<Capability>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homeserver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signup_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_success: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_cancel: Option<String>,
}
