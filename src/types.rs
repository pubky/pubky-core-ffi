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
    /// "signin" or "signup". Legacy URLs (`pubkyauth:///?...`) have no intent
    /// host and are treated as "signin", matching pubky 0.9.1's own parser.
    pub kind: String,
    /// Homeserver public key (bare z-base32) from the `hs` param of signup links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homeserver: Option<String>,
    /// Signup token from the `st` param of signup links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signup_token: Option<String>,
}
