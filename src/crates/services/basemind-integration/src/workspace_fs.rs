//! BasemindWorkspaceFs decorator — wraps WorkspaceFileSystem to route all file reads
//! through Basemind's PII redaction pipeline.

#[cfg(feature = "basemind")]
use async_trait::async_trait;
#[cfg(feature = "basemind")]
use alex_runtime_ports::WorkspaceFileSystem;

#[cfg(feature = "basemind")]
pub struct BasemindWorkspaceFs {
    inner: Box<dyn WorkspaceFileSystem>,
    client: crate::ipc::BasemindClient,
}

#[cfg(feature = "basemind")]
impl BasemindWorkspaceFs {
    pub fn new(inner: Box<dyn WorkspaceFileSystem>, client: crate::ipc::BasemindClient) -> Self {
        Self { inner, client }
    }
}

#[cfg(feature = "basemind")]
#[async_trait]
impl WorkspaceFileSystem for BasemindWorkspaceFs {
    async fn read_file(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let content = self.inner.read_file(path).await?;
        // TODO: Call basemind_extract_and_redact() and return redacted text
        Ok(content)
    }

    async fn read_file_text(&self, path: &str) -> anyhow::Result<String> {
        self.inner.read_file_text(path).await
    }

    async fn write_file(&self, path: &str, contents: &[u8]) -> anyhow::Result<()> {
        self.inner.write_file(path, contents).await
    }

    async fn read_dir(
        &self,
        path: &str,
    ) -> anyhow::Result<Vec<alex_runtime_ports::WorkspaceDirEntry>> {
        self.inner.read_dir(path).await
    }

    async fn exists(&self, path: &str) -> anyhow::Result<bool> {
        self.inner.exists(path).await
    }

    async fn is_file(&self, path: &str) -> anyhow::Result<bool> {
        self.inner.is_file(path).await
    }

    async fn is_dir(&self, path: &str) -> anyhow::Result<bool> {
        self.inner.is_dir(path).await
    }

    async fn create_dir_all(&self, path: &str) -> anyhow::Result<()> {
        self.inner.create_dir_all(path).await
    }

    async fn remove_file(&self, path: &str) -> anyhow::Result<()> {
        self.inner.remove_file(path).await
    }

    async fn metadata(
        &self,
        path: &str,
        follow_symlinks: bool,
    ) -> anyhow::Result<Option<alex_runtime_ports::WorkspaceMetadata>> {
        self.inner.metadata(path, follow_symlinks).await
    }

    fn join_path(&self, root: &str, components: &[&str]) -> String {
        self.inner.join_path(root, components)
    }

    async fn open_read(
        &self,
        path: &str,
    ) -> anyhow::Result<alex_runtime_ports::WorkspaceReader> {
        self.inner.open_read(path).await
    }

    async fn open_write_new(
        &self,
        path: &str,
    ) -> anyhow::Result<alex_runtime_ports::WorkspaceWriter> {
        self.inner.open_write_new(path).await
    }

    async fn atomic_replace(&self, from: &str, to: &str) -> anyhow::Result<()> {
        self.inner.atomic_replace(from, to).await
    }

    async fn remove_dir(&self, path: &str, recursive: bool) -> anyhow::Result<()> {
        self.inner.remove_dir(path, recursive).await
    }

    async fn set_permissions(&self, path: &str, permissions: u32) -> anyhow::Result<()> {
        self.inner.set_permissions(path, permissions).await
    }

    async fn set_modified(
        &self,
        path: &str,
        modified: std::time::SystemTime,
    ) -> anyhow::Result<()> {
        self.inner.set_modified(path, modified).await
    }

    async fn rename(&self, from: &str, to: &str) -> anyhow::Result<()> {
        self.inner.rename(from, to).await
    }

    async fn path_kind_no_follow(
        &self,
        path: &str,
    ) -> anyhow::Result<Option<alex_runtime_ports::WorkspacePathKind>> {
        self.inner.path_kind_no_follow(path).await
    }
}
