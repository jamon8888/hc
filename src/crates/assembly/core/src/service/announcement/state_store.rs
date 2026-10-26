use super::types::AnnouncementState;
use crate::infrastructure::app_paths::PathManager;
use crate::util::errors::{AlexError, AlexResult};
use std::sync::Arc;

pub struct AnnouncementStateStore {
    inner: alex_services_integrations::announcement::AnnouncementStateStore,
}

impl AnnouncementStateStore {
    pub fn new(path_manager: &Arc<PathManager>) -> Self {
        Self {
            inner: alex_services_integrations::announcement::AnnouncementStateStore::new(
                path_manager.user_config_dir(),
            ),
        }
    }

    /// Load state from disk.  Returns a default state if the file does not exist.
    pub async fn load(&self) -> AlexResult<AnnouncementState> {
        self.inner.load().await.map_err(map_state_store_error)
    }

    /// Persist state to disk.
    pub async fn save(&self, state: &AnnouncementState) -> AlexResult<()> {
        self.inner.save(state).await.map_err(map_state_store_error)
    }
}

fn map_state_store_error(
    err: alex_services_integrations::announcement::AnnouncementStateStoreError,
) -> AlexError {
    if err.is_deserialization() {
        AlexError::Deserialization(err.to_string())
    } else if err.is_serialization() {
        AlexError::serialization(err.to_string())
    } else {
        AlexError::io(err.to_string())
    }
}
