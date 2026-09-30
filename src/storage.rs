use crate::{
    get_pubky_client, PubkyCoreError, StorageListOptions, StorageListPage, StorageResourceStats,
    TOKIO_RUNTIME,
};
use pubky::{PubkyResource, PubkySession, ResourcePath, ResourceStats};
use std::time::UNIX_EPOCH;

pub(crate) async fn restore_session(secret: &str) -> Result<PubkySession, PubkyCoreError> {
    Ok(get_pubky_client().restore_session(secret).await?)
}

pub(crate) fn session_path(session: &PubkySession, input: &str) -> Result<String, PubkyCoreError> {
    if input.starts_with('/') {
        return Ok(ResourcePath::parse(input)?.as_str().to_string());
    }

    let resource = input.parse::<PubkyResource>()?;
    if resource.owner != session.public_key() {
        return Err(PubkyCoreError::Validation {
            message: "addressed resource belongs to a different Pubky".to_string(),
        });
    }
    Ok(resource.path.as_str().to_string())
}

fn resource_stats(stats: ResourceStats) -> StorageResourceStats {
    let last_modified_ms = stats.last_modified.and_then(|time| {
        time.duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| u64::try_from(duration.as_millis()).ok())
    });

    StorageResourceStats {
        content_length: stats.content_length,
        content_type: stats.content_type,
        last_modified_ms,
        etag: stats.etag,
    }
}

fn apply_list_options<'a>(
    mut builder: pubky::ListBuilder<'a>,
    options: StorageListOptions,
) -> pubky::ListBuilder<'a> {
    builder = builder.reverse(options.reverse).shallow(options.shallow);
    if let Some(limit) = options.limit {
        builder = builder.limit(limit);
    }
    if let Some(cursor) = options.cursor.as_deref() {
        builder = builder.cursor(cursor);
    }
    builder
}

fn list_page(resources: Vec<PubkyResource>) -> StorageListPage {
    let entries = resources
        .into_iter()
        .map(|resource| resource.to_pubky_url())
        .collect::<Vec<_>>();
    let next_cursor = entries.last().cloned();
    StorageListPage {
        entries,
        next_cursor,
    }
}

/// Read a public resource as raw bytes.
#[uniffi::export]
pub fn public_get_bytes(address: String) -> Result<Vec<u8>, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let response = get_pubky_client().public_storage().get(address).await?;
        Ok(response.bytes().await?.to_vec())
    })
}

#[uniffi::export]
pub fn public_exists(address: String) -> Result<bool, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async { Ok(get_pubky_client().public_storage().exists(address).await?) })
}

#[uniffi::export]
pub fn public_stats(address: String) -> Result<Option<StorageResourceStats>, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        Ok(get_pubky_client()
            .public_storage()
            .stats(address)
            .await?
            .map(resource_stats))
    })
}

#[uniffi::export]
pub fn public_list(
    address: String,
    options: StorageListOptions,
) -> Result<StorageListPage, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let storage = get_pubky_client().public_storage();
        let builder = storage.list(address)?;
        Ok(list_page(
            apply_list_options(builder, options).send().await?,
        ))
    })
}

/// Read an owned public or private resource using a Grant or legacy cookie session.
#[uniffi::export]
pub fn session_get_bytes(
    path_or_address: String,
    session_secret: String,
) -> Result<Vec<u8>, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let session = restore_session(&session_secret).await?;
        let path = session_path(&session, &path_or_address)?;
        let response = session.storage().get(path).await?;
        Ok(response.bytes().await?.to_vec())
    })
}

#[uniffi::export]
pub fn session_exists(
    path_or_address: String,
    session_secret: String,
) -> Result<bool, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let session = restore_session(&session_secret).await?;
        let path = session_path(&session, &path_or_address)?;
        Ok(session.storage().exists(path).await?)
    })
}

#[uniffi::export]
pub fn session_stats(
    path_or_address: String,
    session_secret: String,
) -> Result<Option<StorageResourceStats>, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let session = restore_session(&session_secret).await?;
        let path = session_path(&session, &path_or_address)?;
        Ok(session.storage().stats(path).await?.map(resource_stats))
    })
}

#[uniffi::export]
pub fn session_list(
    path_or_address: String,
    session_secret: String,
    options: StorageListOptions,
) -> Result<StorageListPage, PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let session = restore_session(&session_secret).await?;
        let path = session_path(&session, &path_or_address)?;
        let storage = session.storage();
        let builder = storage.list(path)?;
        Ok(list_page(
            apply_list_options(builder, options).send().await?,
        ))
    })
}

#[uniffi::export]
pub fn session_put_bytes(
    path_or_address: String,
    content: Vec<u8>,
    session_secret: String,
) -> Result<(), PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let session = restore_session(&session_secret).await?;
        let path = session_path(&session, &path_or_address)?;
        session.storage().put(path, content).await?;
        Ok(())
    })
}

#[uniffi::export]
pub fn session_delete(
    path_or_address: String,
    session_secret: String,
) -> Result<(), PubkyCoreError> {
    TOKIO_RUNTIME.block_on(async {
        let session = restore_session(&session_secret).await?;
        let path = session_path(&session, &path_or_address)?;
        session.storage().delete(path).await?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubky::PublicKey;

    #[test]
    fn list_page_uses_last_entry_as_cursor() {
        let owner =
            PublicKey::try_from("ufibwbmed6jeq9k4p583go95wofakh9fwpp4k734trq79pd9u1uy").unwrap();
        let first = PubkyResource::new(owner.clone(), "/pub/example/a").unwrap();
        let second = PubkyResource::new(owner, "/pub/example/b").unwrap();
        let expected = second.to_pubky_url();

        let page = list_page(vec![first, second]);
        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.next_cursor.as_deref(), Some(expected.as_str()));
    }
}
