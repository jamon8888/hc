//! BasemindWorkspaceFs decorator — wraps WorkspaceFileSystem to route all file reads
//! through Basemind's PII redaction pipeline.

#[cfg(feature = "basemind")]
use alex_runtime_ports::WorkspaceFileSystem;
#[cfg(feature = "basemind")]
use async_trait::async_trait;

#[cfg(feature = "basemind")]
pub struct BasemindWorkspaceFs {
    inner: Box<dyn WorkspaceFileSystem>,
    client: crate::ipc::BasemindClient,
    workspace_root: Option<String>,
}

#[cfg(feature = "basemind")]
impl BasemindWorkspaceFs {
    pub fn new(inner: Box<dyn WorkspaceFileSystem>, client: crate::ipc::BasemindClient) -> Self {
        Self {
            inner,
            client,
            workspace_root: None,
        }
    }

    /// Construct a privacy-enforcing filesystem with the root used for every
    /// loopback BaseMind request. The root is mandatory for redaction because
    /// it scopes the sidecar's workspace policy and cache key.
    pub fn with_workspace_root(
        inner: Box<dyn WorkspaceFileSystem>,
        client: crate::ipc::BasemindClient,
        workspace_root: impl Into<String>,
    ) -> Self {
        Self {
            inner,
            client,
            workspace_root: Some(workspace_root.into()),
        }
    }

    async fn redact_text(&self, text: &str) -> anyhow::Result<String> {
        let root = self
            .workspace_root
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("BasemindWorkspaceFs requires a workspace root"))?;
        let payload = self.client.redact_text(text, root).await?;
        if let Some(message) = crate::redaction::ner_coverage_refusal(&payload) {
            anyhow::bail!("{}", message);
        }
        payload["redacted_text"]
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow::anyhow!("basemind response has no redacted_text"))
    }
}

#[cfg(feature = "basemind")]
#[async_trait]
impl WorkspaceFileSystem for BasemindWorkspaceFs {
    async fn read_file(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let content = self.inner.read_file(path).await?;
        let text = String::from_utf8(content)
            .map_err(|_| anyhow::anyhow!("BasemindWorkspaceFs refuses non-UTF-8 file: {path}"))?;
        Ok(self.redact_text(&text).await?.into_bytes())
    }

    async fn read_file_text(&self, path: &str) -> anyhow::Result<String> {
        let content = self.inner.read_file_text(path).await?;
        self.redact_text(&content).await
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

    async fn open_read(&self, path: &str) -> anyhow::Result<alex_runtime_ports::WorkspaceReader> {
        anyhow::bail!(
            "BasemindWorkspaceFs refuses raw streaming reads for {path}; use read_file or read_file_text so redaction runs first"
        )
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
