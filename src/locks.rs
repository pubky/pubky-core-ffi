use crate::storage::{restore_session, session_path};
use crate::{PubkyCoreError, StorageLockInfo, TOKIO_RUNTIME};
use pubky::{SessionStorage, StorageLock};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(uniffi::Object)]
pub struct PubkyStorageLock {
    storage: SessionStorage,
    lock: Mutex<Option<StorageLock>>,
}

impl PubkyStorageLock {
    fn current_lock(&self) -> Result<StorageLock, PubkyCoreError> {
        let guard = self.lock.lock().unwrap();
        let lock = guard.as_ref().ok_or_else(|| PubkyCoreError::State {
            details: "Storage lock has already been released".to_string(),
        })?;
        Ok(lock.clone())
    }
}

#[uniffi::export]
impl PubkyStorageLock {
    pub fn info(&self) -> Result<StorageLockInfo, PubkyCoreError> {
        let lock = self.current_lock()?;
        Ok(StorageLockInfo {
            path: lock.path().as_str().to_string(),
            token: lock.token().to_string(),
            timeout_seconds: lock.timeout().as_secs(),
        })
    }

    pub fn refresh(&self, timeout_seconds: u64) -> Result<StorageLockInfo, PubkyCoreError> {
        TOKIO_RUNTIME.block_on(async {
            let mut lock = self.current_lock()?;
            self.storage
                .refresh_lock(&mut lock, Duration::from_secs(timeout_seconds))
                .await?;
            let info = StorageLockInfo {
                path: lock.path().as_str().to_string(),
                token: lock.token().to_string(),
                timeout_seconds: lock.timeout().as_secs(),
            };
            *self.lock.lock().unwrap() = Some(lock);
            Ok(info)
        })
    }

    pub fn put(&self, content: Vec<u8>) -> Result<(), PubkyCoreError> {
        TOKIO_RUNTIME.block_on(async {
            let lock = self.current_lock()?;
            self.storage.put_locked(&lock, content).await?;
            Ok(())
        })
    }

    pub fn delete(&self) -> Result<(), PubkyCoreError> {
        TOKIO_RUNTIME.block_on(async {
            let lock = self.current_lock()?;
            self.storage.delete_locked(&lock).await?;
            Ok(())
        })
    }

    pub fn unlock(&self) -> Result<(), PubkyCoreError> {
        TOKIO_RUNTIME.block_on(async {
            let lock = self.current_lock()?;
            self.storage.unlock(&lock).await?;
            *self.lock.lock().unwrap() = None;
            Ok(())
        })
    }
}

#[uniffi::export]
pub fn session_lock(
    path_or_address: String,
    session_secret: String,
    timeout_seconds: u64,
) -> Result<Arc<PubkyStorageLock>, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let session = restore_session(&session_secret).await?;
        let path = session_path(&session, &path_or_address)?;
        let storage = session.storage();
        let lock = storage
            .lock(path, Duration::from_secs(timeout_seconds))
            .await?;
        Ok(Arc::new(PubkyStorageLock {
            storage,
            lock: Mutex::new(Some(lock)),
        }))
    })
}
