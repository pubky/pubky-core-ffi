use crate::{
    get_pubky_client, get_pubky_http_client, EventStreamConfig, PubkyCoreError, PubkyStorageEvent,
    TOKIO_RUNTIME,
};
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use pubky::{EventCursor, EventStreamBuilder, PublicKey};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

static EVENT_STREAM_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static EVENT_STREAM_TASKS: Lazy<Mutex<HashMap<String, JoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[uniffi::export(callback_interface)]
pub trait PubkyEventStreamListener: Send + Sync {
    fn on_event(&self, event: PubkyStorageEvent);
    fn on_error(&self, message: String);
    fn on_complete(&self);
}

fn parse_public_key(value: &str, label: &str) -> Result<PublicKey, PubkyCoreError> {
    PublicKey::try_from(value).map_err(|error| PubkyCoreError::Validation {
        message: format!("invalid {label} public key: {error}"),
    })
}

/// Start a historical or live homeserver event subscription.
///
/// The returned ID owns the native stream task and must be passed to
/// `stop_event_stream` when the caller no longer wants events.
#[uniffi::export]
pub fn start_event_stream(
    config: EventStreamConfig,
    listener: Box<dyn PubkyEventStreamListener>,
) -> Result<String, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        if config.users.is_empty() {
            return Err(PubkyCoreError::Validation {
                message: "at least one event-stream user is required".to_string(),
            });
        }

        let users = config
            .users
            .iter()
            .map(|user| {
                Ok((
                    parse_public_key(&user.public_key, "user")?,
                    user.cursor.map(EventCursor::new),
                ))
            })
            .collect::<Result<Vec<_>, PubkyCoreError>>()?;

        let mut builder = if let Some(homeserver) = config.homeserver.as_deref() {
            let homeserver = parse_public_key(homeserver, "homeserver")?;
            EventStreamBuilder::for_homeserver(get_pubky_http_client(), &homeserver)
                .add_users(users.iter().map(|(user, cursor)| (user, *cursor)))?
        } else {
            let (first_user, first_cursor) = &users[0];
            let builder =
                EventStreamBuilder::for_user(get_pubky_http_client(), first_user, *first_cursor);
            builder.add_users(users.iter().skip(1).map(|(user, cursor)| (user, *cursor)))?
        };

        for path in config.paths {
            builder = builder.path(path);
        }
        if let Some(limit) = config.limit {
            builder = builder.limit(limit);
        }
        if let Some(limit) = config.max_event_bytes {
            let limit = usize::try_from(limit).map_err(|_| PubkyCoreError::Validation {
                message: "max_event_bytes exceeds the platform limit".to_string(),
            })?;
            builder = builder.max_event_bytes(limit);
        }
        if config.live {
            builder = builder.live();
        }
        if config.reverse {
            builder = builder.reverse();
        }
        if let Some(secret) = config.session_secret {
            let session = get_pubky_client().restore_session(&secret).await?;
            builder = builder.session(&session);
        }

        let mut stream = builder.subscribe().await?;
        let listener: Arc<dyn PubkyEventStreamListener> = listener.into();
        let id = format!(
            "event-stream-{}",
            EVENT_STREAM_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let task_id = id.clone();
        let task_listener = listener.clone();
        let (start_tx, start_rx) = tokio::sync::oneshot::channel();
        let task = TOKIO_RUNTIME.spawn(async move {
            if start_rx.await.is_err() {
                return;
            }
            while let Some(result) = stream.next().await {
                match result {
                    Ok(event) => task_listener.on_event(PubkyStorageEvent {
                        event_type: event.event_type.as_str().to_string(),
                        resource: event.resource.to_pubky_url(),
                        cursor: event.cursor.id(),
                        content_hash: event
                            .event_type
                            .content_hash()
                            .map(|hash| hash.to_hex().to_string()),
                    }),
                    Err(error) => {
                        task_listener.on_error(error.to_string());
                        EVENT_STREAM_TASKS.lock().unwrap().remove(&task_id);
                        return;
                    }
                }
            }
            task_listener.on_complete();
            EVENT_STREAM_TASKS.lock().unwrap().remove(&task_id);
        });

        EVENT_STREAM_TASKS.lock().unwrap().insert(id.clone(), task);
        let _ = start_tx.send(());
        Ok(id)
    })
}

#[uniffi::export]
pub fn stop_event_stream(subscription_id: String) -> bool {
    match EVENT_STREAM_TASKS.lock().unwrap().remove(&subscription_id) {
        Some(task) => {
            task.abort();
            true
        }
        None => false,
    }
}

#[uniffi::export]
pub fn stop_all_event_streams() -> u64 {
    let mut tasks = EVENT_STREAM_TASKS.lock().unwrap();
    let count = tasks.len() as u64;
    for (_, task) in tasks.drain() {
        task.abort();
    }
    count
}
