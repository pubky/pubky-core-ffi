# Pubky 0.14 Binding Parity

This project binds Pubky's high-level client capabilities rather than exposing
Rust implementation types such as `RequestBuilder`, generic serde helpers, or
builder closures. Generated Swift, Kotlin, and Python APIs are produced from
the same UniFFI surface.

| Pubky capability | FFI surface |
| --- | --- |
| Mainnet/testnet and HTTP configuration | `configure_client`, `switch_network` |
| Public byte reads | `public_get_bytes` |
| Public existence and metadata | `public_exists`, `public_stats` |
| Public pagination | `public_list`, `StorageListOptions`, `StorageListPage` |
| Authenticated public/private byte reads | `session_get_bytes` |
| Authenticated existence and metadata | `session_exists`, `session_stats` |
| Authenticated pagination | `session_list` |
| Authenticated binary writes/deletes | `session_put_bytes`, `session_delete` |
| Grant signin/signup flow | `start_grant_auth_flow_with_config` |
| Grant flow persistence | `save_grant_auth_flow`, `restore_grant_auth_flow` |
| Blocking/non-blocking Grant completion | `await_grant_auth_flow`, `poll_grant_auth_flow` |
| Grant flow cancellation | `cancel_grant_auth_flow` |
| Blocking direct signin | `sign_in_grant_blocking`, `sign_in_cookie_blocking` |
| Historical/live event streams | `start_event_stream`, `stop_event_stream`, `stop_all_event_streams` |
| Private event streams | `EventStreamConfig.session_secret` |
| WebDAV locks | `session_lock`, `PubkyStorageLock` methods |
| Typed failures | `PubkyCoreError` |

The legacy string-vector APIs remain available unchanged for downstream
compatibility.

## Intentionally internal Rust primitives

The following are implementation mechanisms rather than missing mobile client
capabilities:

- `reqwest::RequestBuilder`, `Response`, and generic request bodies
- Generic `get_json<T>` / `put_json<T>` (mobile callers encode and decode their
  own bytes or JSON)
- PKARR builder closures (network selection remains exposed)
- Raw encrypted relay channel actors used underneath authentication flows
- Delegated browser proof-of-possession callbacks; the mobile bindings expose
  local-key Grant flows
- Filesystem-path session and recovery helpers; mobile apps persist returned
  secrets and recovery bytes in platform-managed storage
