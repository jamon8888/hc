use crate::util::errors::{AlexError, AlexResult};
use alex_services_core::workspace_persistence as storage;
pub(crate) use alex_services_core::workspace_persistence::{
    WorkspacePersistenceData, WORKSPACE_PERSISTENCE_FORMAT_VERSION,
};
use std::path::Path;

pub(crate) fn validate_workspace_persistence_data(
    data: &WorkspacePersistenceData,
    miniapp_root: &Path,
) -> AlexResult<()> {
    storage::validate_workspace_persistence_data(data, miniapp_root).map_err(Into::into)
}
pub(crate) fn unsupported_workspace_persistence(detail: impl AsRef<str>) -> AlexError {
    storage::unsupported_workspace_persistence(detail).into()
}
