use serde::Serialize;

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
