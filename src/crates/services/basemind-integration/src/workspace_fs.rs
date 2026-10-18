//! BasemindWorkspaceFs — decorator over LocalWorkspaceFs that redirects content reads to Basemind.

#[cfg(feature = "basemind")]
use std::sync::Arc;

#[cfg(feature = "basemind")]
use openbitfun_runtime_ports::{
    WorkspaceDirEntry, WorkspaceFileSystem, WorkspaceMetadata, WorkspaceReader,
    WorkspaceWriter,
};
#[cfg(feature = "basemind")]
use crate::ipc::BasemindClient;
#[cfg(feature = "basemind")]
use anyhow::Result;

/// Decorator that delegates metadata ops to LocalWorkspaceFs but redirects
/// content reads to Basemind for PII redaction.
#[cfg(feature = "basemind")]
pub struct BasemindWorkspaceFs {
    inner: openbitfun_services_core::workspace::LocalWorkspaceFs,
    basemind: Arc<BasemindClient>,
    redact_filenames: bool,
}

#[cfg(feature = "basemind")]
impl BasemindWorkspaceFs {
    pub fn new(basemind: Arc<BasemindClient>) -> Self {
        Self {
            inner: openbitfun_services_core::workspace::LocalWorkspaceFs,
            basemind,
            redact_filenames: false,
        }
    }

    pub fn with_filename_redaction(mut self, enabled: bool) -> Self {
        self.redact_filenames = enabled;
        self
    }
}

#[cfg(feature = "basemind")]
#[async_trait::async_trait]
impl WorkspaceFileSystem for BasemindWorkspaceFs {
    // Metadata ops: pure delegation to inner
    async fn metadata(
        &self,
        path: &str,
        follow_symlinks: bool,
    ) -> Result<Option<openbitfun_runtime_ports::WorkspaceMetadata>> {
        self.inner.metadata(path, follow_symlinks).await
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        self.inner.exists(path).await
    }

    async fn is_file(&self, path: &str) -> Result<bool> {
        self.inner.is_file(path).await
    }

    async fn is_dir(&self, path: &str) -> Result<bool> {
        self.inner.is_dir(path).await
    }

    async fn read_dir(&self, path: &str) -> Result<Vec<openbitfun_runtime_ports::WorkspaceDirEntry>> {
        let entries = self.inner.read_dir(path).await?;
        if self.redact_filenames {
            // TODO: Apply lightweight PII redaction to entry.name
            // For now, return as-is
        }
        Ok(entries)
    }

    async fn read_dir_bounded(
        &self,
        path: &str,
        max_entries: usize,
    ) -> Result<Vec<openbitfun_runtime_ports::WorkspaceDirEntry>> {
        let entries = self.inner.read_dir_bounded(path, max_entries).await?;
        if self.redact_filenames {
            // TODO: Apply lightweight PII redaction to entry.name
        }
        Ok(entries)
    }

    // Content reads: redirect to Basemind (returns redacted markdown)
    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let redacted = self.basemind.extract_and_redact_path(path).await?;
        Ok(redacted.markdown.as_bytes().to_vec())
    }

    async fn read_file_text(&self, path: &str) -> Result<String> {
        let redacted = self.basemind.extract_and_redact_path(path).await?;
        Ok(redacted.markdown.to_string())
    }

    async fn read_file_bounded(
        &self,
        path: &str,
        max_bytes: usize,
    ) -> Result<Option<Vec<u8>>> {
        let redacted = self.basemind.extract_and_redact_path(path).await?;
        let bytes = redacted.markdown.as_bytes();
        if bytes.len() > max_bytes {
            Ok(None)
        } else {
            Ok(Some(bytes.to_vec()))
        }
    }

    async fn read_file_text_bounded(
        &self,
        path: &str,
        max_bytes: usize,
    ) -> Result<Option<String>> {
        self.read_file_bounded(path, max_bytes)
            .await?
            .map(String::from_utf8)
            .transpose()
            .map_err(Into::into)
    }

    // Write ops: pure delegation (never redact on write)
    async fn write_file(&self, path: &str, contents: &[u8]) -> Result<()> {
        self.inner.write_file(path, contents).await
    }

    async fn open_write_new(&self, path: &str) -> Result<openbitfun_runtime_ports::WorkspaceWriter> {
        self.inner.open_write_new(path).await
    }

    async fn atomic_replace(&self, from: &str, to: &str) -> Result<()> {
        self.inner.atomic_replace(from, to).await
    }

    async fn open_read(&self, path: &str) -> Result<openbitfun_runtime_ports::WorkspaceReader> {
        self.inner.open_read(path).await
    }

    async fn remove_file(&self, path: &str) -> Result<()> {
        self.inner.remove_file(path).await
    }

    async fn remove_dir(&self, path: &str, recursive: bool) -> Result<()> {
        self.inner.remove_dir(path, recursive).await
    }

    async fn create_dir_all(&self, path: &str) -> Result<()> {
        self.inner.create_dir_all(path).await
    }

    async fn set_permissions(&self, path: &str, permissions: u32) -> Result<()> {
        self.inner.set_permissions(path, permissions).await
    }

    async fn set_modified(&self, path: &str, modified: std::time::SystemTime) -> Result<()> {
        self.inner.set_modified(path, modified).await
    }

    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        self.inner.rename(from, to).await
    }

    async fn path_kind_no_follow(&self, path: &str) -> Result<Option<openbitfun_runtime_ports::WorkspacePathKind>> {
        self.inner.path_kind_no_follow(path).await
    }
}

// Feature-gated constructor for services-core
#[cfg(feature = "basemind")]
pub fn basemind_workspace_services(
    workspace_root: String,
    basemind: Arc<BasemindClient>,
) -> openbitfun_runtime_ports::WorkspaceServices {
    openbitfun_runtime_ports::WorkspaceServices {
        fs: Arc::new(BasemindWorkspaceFs::new(basemind)),
        shell: Arc::new(openbitfun_services_core::workspace::LocalWorkspaceShell::new(workspace_root)),
    }
}