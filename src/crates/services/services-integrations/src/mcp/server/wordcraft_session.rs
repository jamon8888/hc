//! WordCraft headless editing sessions served over the MCP stdio registry.
//!
//! One `wordcraft-cli mcp` server per workspace session with a document open
//! (map decision, ticket 05): local routing owns only the session definition —
//! the resolved concurrency-free argv/env/working-directory — while the
//! remote path also owns the workspace stdio transport and its retained MCP
//! session registry. Spawning is done exclusively through the
//! `process_runtime`/`process_manager` path inside `MCPConnection`
//! (`CREATE_NO_WINDOW` policy lives there, never re-implemented here).

#[cfg(feature = "remote-ssh-concrete")]
use super::{MCPConnection, MCPServerProcess, MCPServerStatus, MCPServerTimeouts, MCPServerType};
use crate::mcp::{MCPRuntimeError, MCPRuntimeResult};
#[cfg(feature = "remote-ssh-concrete")]
use crate::remote_ssh::{shell_quote_posix, SSHConnectionManager, WorkspaceStdio};
#[cfg(feature = "remote-ssh-concrete")]
use serde_json::Value;
#[cfg(feature = "remote-ssh-concrete")]
use std::collections::HashMap;
#[cfg(feature = "remote-ssh-concrete")]
use std::sync::{Arc, Weak};
use std::time::Duration;
#[cfg(feature = "remote-ssh-concrete")]
use tokio::sync::{Mutex, RwLock};

/// The headless CLI command resolved by the managed-runtime order
/// (explicit path → system PATH → managed component, see services-core
/// `wordcraft` / `managed_runtime`).
pub const WORDCRAFT_CLI_COMMAND: &str = "wordcraft-cli";

/// Idle threshold after the last tool call before the session owner may reap a
/// headless WordCraft server.
pub const WORDCRAFT_IDLE_REAP: Duration = Duration::from_secs(10 * 60);

/// Author name embedded into tracked changes; never contains secrets.
pub const WORDCRAFT_AUTHOR_PREFIX: &str = "Alex Agent";

/// Valid session-id shape: ASCII alphanumeric plus `-`, `_`, `.`.
pub const WORDCRAFT_SESSION_ID_MAX_LEN: usize = 64;

/// Definition of one headless WordCraft session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordcraftSessionSpec {
    /// Validated session identifier (labels the server id and tracked-change
    /// authorship).
    session_id: String,
    /// Active workspace folder — both automation roots (read = write).
    workspace_root: String,
    /// Isolated WordCraft profile dir under Alex state
    /// (isolated recovery copies; never the user's own WordCraft profile).
    profile_dir: String,
}

/// A capability fact obtained from the workspace owner before starting a
/// remote WordCraft session. The owner must resolve and advertise the pinned
/// executable; the controller never guesses by searching the owner's PATH.
#[cfg(feature = "remote-ssh-concrete")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordcraftRemoteRuntime {
    command: String,
    version: String,
    mcp_supported: bool,
}

#[cfg(feature = "remote-ssh-concrete")]
impl WordcraftRemoteRuntime {
    pub fn new(
        command: impl Into<String>,
        version: impl Into<String>,
        mcp_supported: bool,
    ) -> MCPRuntimeResult<Self> {
        let command = command.into();
        let version = version.into();
        if !command.starts_with('/') || command.contains('\\') {
            return Err(MCPRuntimeError::configuration(
                "Remote WordCraft runtime command must be an absolute POSIX path",
            ));
        }
        if version.trim().is_empty() {
            return Err(MCPRuntimeError::configuration(
                "Remote WordCraft runtime version must not be empty",
            ));
        }
        if !mcp_supported {
            return Err(MCPRuntimeError::not_implemented(
                "Remote WordCraft runtime does not advertise MCP headless editing",
            ));
        }
        Ok(Self {
            command,
            version,
            mcp_supported,
        })
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn mcp_supported(&self) -> bool {
        self.mcp_supported
    }
}

/// Stable identity used when attaching to a retained remote WordCraft owner.
/// Every field is revalidated before an existing session is reused.
#[cfg(feature = "remote-ssh-concrete")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordcraftRemoteSessionIdentity {
    connection_id: String,
    session_id: String,
    workspace_root: String,
    profile_dir: String,
    runtime_command: String,
    runtime_version: String,
}

#[cfg(feature = "remote-ssh-concrete")]
impl WordcraftRemoteSessionIdentity {
    fn from_spec(
        connection_id: &str,
        spec: &WordcraftSessionSpec,
        runtime: &WordcraftRemoteRuntime,
    ) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            session_id: spec.session_id.clone(),
            workspace_root: normalize_remote_workspace_path(&spec.workspace_root),
            profile_dir: spec.profile_dir.clone(),
            runtime_command: runtime.command.clone(),
            runtime_version: runtime.version.clone(),
        }
    }

    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn workspace_root(&self) -> &str {
        &self.workspace_root
    }

    fn key(&self) -> WordcraftRemoteSessionKey {
        WordcraftRemoteSessionKey {
            connection_id: self.connection_id.clone(),
            workspace_root: self.workspace_root.clone(),
            session_id: self.session_id.clone(),
        }
    }
}

/// Key for the one owner process retained per remote connection/workspace/
/// document session. The registry deliberately has no idle reaper: ownership
/// ends only through an explicit stop or process failure.
#[cfg(feature = "remote-ssh-concrete")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WordcraftRemoteSessionKey {
    connection_id: String,
    workspace_root: String,
    session_id: String,
}

#[cfg(feature = "remote-ssh-concrete")]
impl WordcraftRemoteSessionKey {
    pub fn new(connection_id: &str, spec: &WordcraftSessionSpec) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            workspace_root: normalize_remote_workspace_path(&spec.workspace_root),
            session_id: spec.session_id.clone(),
        }
    }
}

/// One validated WordCraft MCP session definition (headless or live bridge),
/// consumable by the MCP registry as a local stdio server definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordcraftMcpSession {
    pub server_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub working_directory: Option<String>,
}

/// Alias kept for readability of the headless definition shape.
pub type WordcraftHeadlessSession = WordcraftMcpSession;

/// A live headless MCP owner for a remote workspace.
///
/// This type deliberately keeps the transport and the MCP process together:
/// stopping the session tears down the process on the workspace host, while
/// the controller retains only the protocol connection and never a local copy
/// of the document.
#[cfg(feature = "remote-ssh-concrete")]
pub struct WordcraftRemoteMcpSession {
    process: MCPServerProcess,
    connection: Arc<MCPConnection>,
    identity: WordcraftRemoteSessionIdentity,
}

#[cfg(feature = "remote-ssh-concrete")]
impl WordcraftRemoteMcpSession {
    pub async fn connect(
        ssh_manager: &SSHConnectionManager,
        connection_id: &str,
        spec: &WordcraftSessionSpec,
        runtime: &WordcraftRemoteRuntime,
        timeouts: MCPServerTimeouts,
    ) -> MCPRuntimeResult<Self> {
        let transport = spec
            .open_remote_headless_stdio(ssh_manager, connection_id, runtime)
            .await?;
        Self::from_workspace_stdio(connection_id, spec, runtime, transport, timeouts).await
    }

    async fn from_workspace_stdio(
        connection_id: &str,
        spec: &WordcraftSessionSpec,
        runtime: &WordcraftRemoteRuntime,
        transport: WorkspaceStdio,
        timeouts: MCPServerTimeouts,
    ) -> MCPRuntimeResult<Self> {
        let mut process = MCPServerProcess::new(
            spec.server_id(),
            format!("WordCraft ({})", spec.session_id()),
            MCPServerType::Remote,
        );
        process.start_workspace_stdio(transport, timeouts).await?;
        let connection = process.connection().ok_or_else(|| {
            MCPRuntimeError::mcp("Remote WordCraft started without an MCP connection")
        })?;
        Ok(Self {
            process,
            connection,
            identity: WordcraftRemoteSessionIdentity::from_spec(connection_id, spec, runtime),
        })
    }

    pub fn connection(&self) -> &Arc<MCPConnection> {
        &self.connection
    }

    pub fn identity(&self) -> &WordcraftRemoteSessionIdentity {
        &self.identity
    }

    pub async fn call_tool(
        &self,
        operation: &str,
        params: Option<Value>,
    ) -> MCPRuntimeResult<crate::mcp::protocol::MCPToolResult> {
        self.connection.call_tool(operation, params).await
    }

    pub async fn stop(&mut self) -> MCPRuntimeResult<()> {
        self.process.stop().await
    }

    async fn is_disconnected(&self) -> bool {
        self.connection.is_closed()
            || matches!(
                self.process.status().await,
                MCPServerStatus::Failed
                    | MCPServerStatus::NeedsAuth
                    | MCPServerStatus::Reconnecting
                    | MCPServerStatus::Stopped
            )
    }
}

/// Retains remote WordCraft owners for the lifetime of the owning Alex host.
/// This is the in-process seam for the planned host-managed daemon: callers
/// must supply a freshly negotiated runtime fact, and a mismatched identity is
/// never silently attached to an existing process.
#[cfg(feature = "remote-ssh-concrete")]
#[derive(Clone, Default)]
pub struct WordcraftRemoteSessionRegistry {
    sessions:
        Arc<RwLock<HashMap<WordcraftRemoteSessionKey, Arc<Mutex<WordcraftRemoteMcpSession>>>>>,
    session_locks: Arc<Mutex<HashMap<WordcraftRemoteSessionKey, Weak<Mutex<()>>>>>,
}

#[cfg(feature = "remote-ssh-concrete")]
impl WordcraftRemoteSessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    async fn lock_for(&self, key: &WordcraftRemoteSessionKey) -> Arc<Mutex<()>> {
        let mut locks = self.session_locks.lock().await;
        if let Some(lock) = locks.get(key).and_then(Weak::upgrade) {
            return lock;
        }
        locks.retain(|_, lock| lock.strong_count() > 0);
        let lock = Arc::new(Mutex::new(()));
        locks.insert(key.clone(), Arc::downgrade(&lock));
        lock
    }

    /// Attach to an existing owner only when its full identity still matches;
    /// otherwise stop the old owner before starting a replacement.
    pub async fn connect_or_attach(
        &self,
        ssh_manager: &SSHConnectionManager,
        connection_id: &str,
        spec: &WordcraftSessionSpec,
        runtime: &WordcraftRemoteRuntime,
        timeouts: MCPServerTimeouts,
    ) -> MCPRuntimeResult<Arc<Mutex<WordcraftRemoteMcpSession>>> {
        let key = WordcraftRemoteSessionKey::new(connection_id, spec);
        let key_lock = self.lock_for(&key).await;
        let _key_guard = key_lock.lock().await;
        let identity = WordcraftRemoteSessionIdentity::from_spec(connection_id, spec, runtime);
        if let Some(existing) = {
            let sessions = self.sessions.read().await;
            sessions.get(&key).cloned()
        } {
            let reusable = {
                let session = existing.lock().await;
                session.identity() == &identity && !session.is_disconnected().await
            };
            if reusable {
                return Ok(existing);
            }
            let removed = {
                let mut sessions = self.sessions.write().await;
                if sessions
                    .get(&key)
                    .is_some_and(|current| Arc::ptr_eq(current, &existing))
                {
                    sessions.remove(&key)
                } else {
                    None
                }
            };
            if let Some(old) = removed {
                let stop_result = {
                    let mut session = old.lock().await;
                    session.stop().await
                };
                if let Err(error) = stop_result {
                    self.sessions.write().await.insert(key.clone(), old);
                    return Err(error);
                }
            }
        }

        let session =
            WordcraftRemoteMcpSession::connect(ssh_manager, connection_id, spec, runtime, timeouts)
                .await?;
        self.register_inner(key, session).await
    }

    /// Register an already-connected owner, such as one restored by a future
    /// host-managed daemon attach. The key and full identity must agree.
    pub async fn register(
        &self,
        key: WordcraftRemoteSessionKey,
        session: WordcraftRemoteMcpSession,
    ) -> MCPRuntimeResult<Arc<Mutex<WordcraftRemoteMcpSession>>> {
        let key_lock = self.lock_for(&key).await;
        let _key_guard = key_lock.lock().await;
        self.register_inner(key, session).await
    }

    async fn register_inner(
        &self,
        key: WordcraftRemoteSessionKey,
        session: WordcraftRemoteMcpSession,
    ) -> MCPRuntimeResult<Arc<Mutex<WordcraftRemoteMcpSession>>> {
        let identity = session.identity().clone();
        if identity.key() != key {
            let mut session = session;
            let cleanup = session.stop().await;
            if let Err(cleanup_error) = cleanup {
                return Err(MCPRuntimeError::process(format!(
                    "Remote WordCraft session identity changed while attaching; cleanup failed: {cleanup_error}"
                )));
            }
            return Err(MCPRuntimeError::configuration(
                "Remote WordCraft session key does not match its owner identity",
            ));
        }
        let session = Arc::new(Mutex::new(session));
        if let Some(existing) = {
            let sessions = self.sessions.read().await;
            sessions.get(&key).cloned()
        } {
            let same_identity = existing.lock().await.identity() == &identity;
            if !same_identity {
                let cleanup = session.lock().await.stop().await;
                if let Err(cleanup_error) = cleanup {
                    return Err(MCPRuntimeError::process(format!(
                        "Remote WordCraft session identity changed while attaching; cleanup failed: {cleanup_error}"
                    )));
                }
                return Err(MCPRuntimeError::configuration(
                    "Remote WordCraft session identity changed while attaching",
                ));
            }
            if !existing.lock().await.is_disconnected().await {
                if let Err(first_error) = session.lock().await.stop().await {
                    if let Err(second_error) = session.lock().await.stop().await {
                        return Err(MCPRuntimeError::process(format!(
                            "Failed to clean up duplicate remote WordCraft session: {first_error}; retry failed: {second_error}"
                        )));
                    }
                }
                return Ok(existing);
            }
            let removed = {
                let mut sessions = self.sessions.write().await;
                if sessions
                    .get(&key)
                    .is_some_and(|current| Arc::ptr_eq(current, &existing))
                {
                    sessions.remove(&key)
                } else {
                    None
                }
            };
            if let Some(old) = removed {
                let stop_result = {
                    let mut session = old.lock().await;
                    session.stop().await
                };
                if let Err(error) = stop_result {
                    self.sessions.write().await.insert(key.clone(), old);
                    let cleanup = session.lock().await.stop().await;
                    return match cleanup {
                        Ok(()) => Err(error),
                        Err(cleanup_error) => Err(MCPRuntimeError::process(format!(
                            "Failed to stop replaced remote WordCraft session: {error}; new session cleanup failed: {cleanup_error}"
                        ))),
                    };
                }
            }
        }
        let mut sessions = self.sessions.write().await;
        sessions.insert(key, session.clone());
        Ok(session)
    }

    pub async fn attached(
        &self,
        key: &WordcraftRemoteSessionKey,
    ) -> Option<Arc<Mutex<WordcraftRemoteMcpSession>>> {
        self.sessions.read().await.get(key).cloned()
    }

    pub async fn stop(&self, key: &WordcraftRemoteSessionKey) -> MCPRuntimeResult<()> {
        let key_lock = self.lock_for(key).await;
        let _key_guard = key_lock.lock().await;
        let session = {
            let sessions = self.sessions.read().await;
            sessions.get(key).cloned()
        };
        if let Some(session) = session {
            session.lock().await.stop().await?;
            let mut sessions = self.sessions.write().await;
            if sessions
                .get(key)
                .is_some_and(|current| Arc::ptr_eq(current, &session))
            {
                sessions.remove(key);
            }
        }
        Ok(())
    }
}

/// Canonical lexical form used only for remote workspace identity. Remote
/// paths are POSIX strings even when the controller runs on another OS.
fn normalize_remote_workspace_path(value: &str) -> String {
    let absolute = value.starts_with('/');
    let mut components = Vec::new();
    for component in value.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components.last().is_some_and(|last| *last != "..") {
                    components.pop();
                } else if !absolute {
                    components.push("..");
                }
            }
            component => components.push(component),
        }
    }
    let joined = components.join("/");
    if absolute {
        if joined.is_empty() {
            "/".to_string()
        } else {
            format!("/{joined}")
        }
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

impl WordcraftSessionSpec {
    /// Builds a spec with the read and write automation roots both mapped to
    /// the active workspace (map ticket 05).
    pub fn new(
        session_id: impl Into<String>,
        workspace_root: impl Into<String>,
        profile_dir: impl Into<String>,
    ) -> MCPRuntimeResult<Self> {
        let session_id: String = session_id.into();
        let workspace_root: String = workspace_root.into();
        let profile_dir: String = profile_dir.into();
        Self::validate_id(&session_id)?;
        Self::validate_value(&workspace_root, "workspace root")?;
        Self::validate_value(&profile_dir, "profile dir")?;
        Ok(Self {
            session_id,
            workspace_root,
            profile_dir,
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn workspace_root(&self) -> &str {
        &self.workspace_root
    }

    pub fn profile_dir(&self) -> &str {
        &self.profile_dir
    }

    /// The registry-visible server name for this session.
    pub fn server_id(&self) -> String {
        format!("wordcraft-{}", self.session_id)
    }

    /// Tracked-changes author for every edit of this session.
    pub fn author(&self) -> String {
        format!("{} ({})", WORDCRAFT_AUTHOR_PREFIX, self.session_id)
    }

    fn validate_id(id: &str) -> MCPRuntimeResult<()> {
        if id.is_empty() || id.len() > WORDCRAFT_SESSION_ID_MAX_LEN {
            return Err(MCPRuntimeError::configuration(
                "WordCraft session id must be 1..64 characters",
            ));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(MCPRuntimeError::configuration(
                "WordCraft session id must be ASCII alphanumeric plus '-', '_', '.'",
            ));
        }
        if id.starts_with('-') || id.starts_with('.') {
            return Err(MCPRuntimeError::configuration(
                "WordCraft session id must not start with '-' or '.'",
            ));
        }
        Ok(())
    }

    fn validate_value(value: &str, label: &str) -> MCPRuntimeResult<()> {
        if value.trim().is_empty() {
            return Err(MCPRuntimeError::configuration(format!(
                "WordCraft session {label} must not be empty"
            )));
        }
        if value.chars().any(|c| c.is_control()) {
            return Err(MCPRuntimeError::configuration(format!(
                "WordCraft session {label} must not contain control characters"
            )));
        }
        Ok(())
    }

    /// Builds the config-less local-server definition (argv/env/cwd) for this
    /// session.
    pub fn build_headless_session(&self) -> WordcraftMcpSession {
        WordcraftMcpSession {
            server_id: self.server_id(),
            command: WORDCRAFT_CLI_COMMAND.to_string(),
            args: vec![
                "mcp".to_string(),
                // Tracked changes always on for the agent, with a reviewable
                // session author (map ticket 04). Passing --author at startup
                // is authoritative; the agent asserts review.trackChanges
                // {"value": true} per batch through tool guidance.
                "--author".to_string(),
                self.author(),
                // Both roots map onto the active workspace: editing in place
                // is the product need; anything outside is refused by
                // upstream's automation gate with a readable error.
                "--automation-read-root".to_string(),
                self.workspace_root.clone(),
                "--automation-write-root".to_string(),
                self.workspace_root.clone(),
            ],
            env: vec![(
                "WORDCRAFT_PROFILE_DIR".to_string(),
                self.profile_dir.clone(),
            )],
            // The server resolves relative paths against this directory.
            working_directory: Some(self.workspace_root.clone()),
        }
    }

    /// Builds the command executed by the workspace owner for a remote
    /// headless session. The workspace root and profile remain POSIX strings;
    /// they are quoted for the target shell and are never interpreted through
    /// the controller's filesystem APIs.
    #[cfg(feature = "remote-ssh-concrete")]
    pub fn build_remote_headless_command(
        &self,
        runtime: &WordcraftRemoteRuntime,
    ) -> MCPRuntimeResult<String> {
        Self::validate_remote_path(&self.workspace_root, "workspace root")?;
        Self::validate_remote_path(&self.profile_dir, "profile dir")?;
        let author = shell_quote_posix(&self.author());
        let workspace_root = shell_quote_posix(&self.workspace_root);
        let profile_dir = shell_quote_posix(&self.profile_dir);
        Ok(format!(
            "cd {workspace_root} && WORDCRAFT_PROFILE_DIR={profile_dir} {command} mcp --author {author} --automation-read-root {workspace_root} --automation-write-root {workspace_root}",
            command = shell_quote_posix(runtime.command()),
        ))
    }

    #[cfg(feature = "remote-ssh-concrete")]
    fn validate_remote_path(value: &str, label: &str) -> MCPRuntimeResult<()> {
        if !value.starts_with('/') || value.contains('\\') {
            return Err(MCPRuntimeError::configuration(format!(
                "Remote WordCraft {label} must be an absolute POSIX path"
            )));
        }
        Ok(())
    }

    /// Opens the headless WordCraft process on the workspace owner. The
    /// returned transport must be kept by the session owner for the lifetime
    /// of the MCP connection; dropping it tears down the owner-side process.
    #[cfg(feature = "remote-ssh-concrete")]
    pub async fn open_remote_headless_stdio(
        &self,
        ssh_manager: &SSHConnectionManager,
        connection_id: &str,
        runtime: &WordcraftRemoteRuntime,
    ) -> MCPRuntimeResult<WorkspaceStdio> {
        let command = self.build_remote_headless_command(runtime)?;
        ssh_manager
            .open_workspace_stdio(connection_id, &command)
            .await
            .map_err(|error| {
                MCPRuntimeError::process(format!(
                    "Failed to start WordCraft on the remote workspace owner: {error}"
                ))
            })
    }
}

/// Idle-reap decision for the session owner: a headless server is reapable once
/// no tool activity happened for `WORDCRAFT_IDLE_REAP`.
pub fn wordcraft_session_idle_reap_due(last_activity_epoch_ms: u64, now_epoch_ms: u64) -> bool {
    let elapsed = now_epoch_ms.saturating_sub(last_activity_epoch_ms);
    elapsed >= WORDCRAFT_IDLE_REAP.as_millis() as u64
}

/// Live-editor bridge (`live-if-open`, map ticket 04): when the target document
/// is open in the visible WordCraft window, agent tool calls bridge to *that*
/// session via `wordcraft-cli mcp --connect 127.0.0.1:<port>`.
///
/// Deliberately different from the headless definition: no automation roots
/// (upstream refuses `--connect` combined with `--automation-*-root`) and no
/// profile-dir env (the window's own process owns its profile). The control key
/// travels only as env of this short-lived bridge child; upstream sends it to
/// loopback addresses exclusively, which is the only supported connect target
/// here (remote workspace needs ticket 14's provisioning path instead).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordcraftLiveBridgeSpec {
    session_id: String,
    control_port: u16,
    control_key: String,
}

impl WordcraftLiveBridgeSpec {
    pub fn new(
        session_id: impl Into<String>,
        control_port: u16,
        control_key: impl Into<String>,
    ) -> MCPRuntimeResult<Self> {
        let session_id: String = session_id.into();
        let control_key: String = control_key.into();

        // Reuse the session-id rules (same naming/validation surface).
        Self::validate_id(&session_id)?;
        if control_port == 0 {
            return Err(MCPRuntimeError::configuration(
                "WordCraft control port must be 1..=65535",
            ));
        }
        if control_key.trim().is_empty() || control_key.chars().any(|c| c.is_control()) {
            return Err(MCPRuntimeError::configuration(
                "WordCraft control key must be non-empty and free of control characters",
            ));
        }
        Ok(Self {
            session_id,
            control_port,
            control_key,
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn control_port(&self) -> u16 {
        self.control_port
    }

    /// Builds the bridge definition that tunnels MCP into the visible app.
    pub fn build_session(&self) -> WordcraftMcpSession {
        WordcraftMcpSession {
            server_id: format!("wordcraft-live-{}", self.session_id),
            command: WORDCRAFT_CLI_COMMAND.to_string(),
            args: vec![
                "mcp".to_string(),
                "--connect".to_string(),
                format!("127.0.0.1:{}", self.control_port),
            ],
            env: vec![(
                "WORDCRAFT_CONTROL_KEY".to_string(),
                self.control_key.clone(),
            )],
            working_directory: None,
        }
    }

    fn validate_id(id: &str) -> MCPRuntimeResult<()> {
        if id.is_empty() || id.len() > WORDCRAFT_SESSION_ID_MAX_LEN {
            return Err(MCPRuntimeError::configuration(
                "WordCraft session id must be 1..64 characters",
            ));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(MCPRuntimeError::configuration(
                "WordCraft session id must be ASCII alphanumeric plus '-', '_', '.'",
            ));
        }
        if id.starts_with('-') || id.starts_with('.') {
            return Err(MCPRuntimeError::configuration(
                "WordCraft session id must not start with '-' or '.'",
            ));
        }
        Ok(())
    }
}

/// Route decision for one agent edit target (the `live-if-open` rule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordcraftSessionRoute {
    /// The visible window holds the document: bridge tool calls to that
    /// session so the user watches edits happen live and unsaved work is safe.
    Live(WordcraftMcpSession),
    /// Nothing holds the file: the headless definition edits the file directly.
    Headless(WordcraftMcpSession),
    /// Single-writer refusal: the file is (or was) open but the live bridge is
    /// gone. Never silently race headless writes against a dirty or
    /// freshly-closed window — the caller gets a loud reason instead.
    Refused(String),
}

/// Decides where agent work on a document goes this turn. `document_open_in_visible_app`
/// is the desktop adapter's view (window/session state, ticket 05); `live_bridge`
/// is the reachable `--connect` spec or `None`.
pub fn wordcraft_route_session(
    document_open_in_visible_app: bool,
    live_bridge: Option<WordcraftLiveBridgeSpec>,
    headless: &WordcraftSessionSpec,
) -> MCPRuntimeResult<WordcraftSessionRoute> {
    if !document_open_in_visible_app {
        return Ok(WordcraftSessionRoute::Headless(
            headless.build_headless_session(),
        ));
    }
    match live_bridge {
        Some(bridge) => Ok(WordcraftSessionRoute::Live(bridge.build_session())),
        None => Ok(WordcraftSessionRoute::Refused(
            "WordCraft document is open in the visible app but its control bridge is \
             unavailable. Restore the window/control session (Recheck) or close the \
             document (saving first) before headless editing; refusing to race another \
             writer on this file."
                .to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> WordcraftSessionSpec {
        WordcraftSessionSpec::new(
            "ws-a1b2",
            "/work/projects/demo",
            "/state/wordcraft/profiles/ws-a1b2",
        )
        .expect("valid spec")
    }

    #[test]
    fn builds_author_and_server_id() {
        let s = spec();
        assert_eq!(s.server_id(), "wordcraft-ws-a1b2");
        assert_eq!(s.author(), "Alex Agent (ws-a1b2)");
    }

    #[cfg(feature = "remote-ssh-concrete")]
    #[test]
    fn remote_session_keys_ignore_trailing_slashes() {
        // `/srv/project` and `/srv/project/` address the same remote directory,
        // so both must collapse to one registry key (one owner process).
        let plain = WordcraftSessionSpec::new("ws-a1b2", "/work/projects/demo", "/state/p")
            .expect("plain root valid");
        let slashed = WordcraftSessionSpec::new("ws-a1b2", "/work/projects/demo/", "/state/p")
            .expect("trailing root valid");
        assert_eq!(
            WordcraftRemoteSessionKey::new("conn-1", &plain),
            WordcraftRemoteSessionKey::new("conn-1", &slashed)
        );
        assert!(plain.workspace_root.ends_with("/demo"));
        // The spec keeps the raw root: only registry keys are canonicalized,
        // so argv/application behavior is unchanged.
        assert_eq!(plain.workspace_root, "/work/projects/demo");
    }

    #[cfg(feature = "remote-ssh-concrete")]
    #[test]
    fn normalize_remote_workspace_path_is_lexical_posix() {
        assert_eq!(
            normalize_remote_workspace_path("/srv//project/"),
            "/srv/project"
        );
        assert_eq!(
            normalize_remote_workspace_path("/srv/project/./"),
            "/srv/project"
        );
        assert_eq!(
            normalize_remote_workspace_path("/srv/project/sub/../"),
            "/srv/project"
        );
        // Absolute roots stay absolute even when `..` resolves upward.
        assert_eq!(
            normalize_remote_workspace_path("/srv/other/../project"),
            "/srv/project"
        );
    }

    #[test]
    fn headless_session_args_shape_roots_and_cwd() {
        let session = spec().build_headless_session();
        assert_eq!(session.command, "wordcraft-cli");
        assert_eq!(
            session.args,
            vec![
                "mcp",
                "--author",
                "Alex Agent (ws-a1b2)",
                "--automation-read-root",
                "/work/projects/demo",
                "--automation-write-root",
                "/work/projects/demo",
            ]
        );
        assert_eq!(
            session.env,
            vec![(
                "WORDCRAFT_PROFILE_DIR".to_string(),
                "/state/wordcraft/profiles/ws-a1b2".to_string()
            )]
        );
        assert_eq!(
            session.working_directory.as_deref(),
            Some("/work/projects/demo")
        );
        assert_eq!(session.server_id, "wordcraft-ws-a1b2");
    }

    #[test]
    fn rejects_bad_session_ids_loudly() {
        for bad in [
            "",
            "has space",
            "yes/newline",
            "-leading-dash",
            ".leading-dot",
            "a".repeat(65).as_str(),
        ] {
            let err =
                WordcraftSessionSpec::new(bad, "/work", "/state/p").expect_err("expected reject");
            assert!(
                err.to_string().contains("session id"),
                "unexpected error for {bad}: {err}"
            );
        }
        // Max length boundary is accepted.
        WordcraftSessionSpec::new("a".repeat(64), "/work", "/state/p")
            .expect("max length id valid");
    }

    #[test]
    fn rejects_empty_or_control_characters_in_paths() {
        let e1 = WordcraftSessionSpec::new("s1", "   ", "/state/p").expect_err("empty workspace");
        assert!(e1.to_string().contains("workspace root"));

        let e2 = WordcraftSessionSpec::new("s1", "/work/a\tb", "/state/p")
            .expect_err("control char workspace");
        assert!(e2.to_string().contains("control characters"));

        let e3 = WordcraftSessionSpec::new("s1", "/work", "").expect_err("empty profile");
        assert!(e3.to_string().contains("profile dir"));
    }

    #[test]
    fn idle_reap_due_after_threshold_only() {
        let reap = WORDCRAFT_IDLE_REAP.as_millis() as u64;
        assert!(!wordcraft_session_idle_reap_due(1_000, 1_000 + reap - 1));
        assert!(wordcraft_session_idle_reap_due(1_000, 1_000 + reap));
        // Saturates before the clock regresses (last activity in the future).
        assert!(!wordcraft_session_idle_reap_due(9_999, 1_000));
    }

    #[test]
    fn live_bridge_definition_uses_connect_without_roots() {
        let bridge =
            WordcraftLiveBridgeSpec::new("ws-a1b2", 7981, "abcd1234").expect("valid bridge spec");
        let session = bridge.build_session();
        assert_eq!(session.server_id, "wordcraft-live-ws-a1b2");
        assert_eq!(session.command, "wordcraft-cli");
        assert_eq!(session.args, vec!["mcp", "--connect", "127.0.0.1:7981"]);
        // No automation roots on the live path (upstream refuse combination),
        // no profile env (the visible app owns its own profile process).
        assert_eq!(
            session.env,
            vec![("WORDCRAFT_CONTROL_KEY".to_string(), "abcd1234".to_string())]
        );
        assert_eq!(session.working_directory, None);
    }

    #[test]
    fn live_bridge_rejects_bad_specs_loudly() {
        let port0 =
            WordcraftLiveBridgeSpec::new("s1", 0, "key").expect_err("port 0 must be refused");
        assert!(port0.to_string().contains("control port"));

        let empty_key =
            WordcraftLiveBridgeSpec::new("s1", 7981, "  ").expect_err("empty key must be refused");
        assert!(empty_key.to_string().contains("control key"));

        let control_key = WordcraftLiveBridgeSpec::new("s1", 7981, "k\ny")
            .expect_err("control-char key must be refused");
        assert!(control_key.to_string().contains("control characters"));

        let bad_id = WordcraftLiveBridgeSpec::new("bad id", 7981, "key")
            .expect_err("bad id must be refused");
        assert!(bad_id.to_string().contains("session id"));
    }

    #[cfg(feature = "remote-ssh-concrete")]
    #[test]
    fn remote_headless_command_keeps_target_paths_and_quotes_shell_values() {
        let spec = WordcraftSessionSpec::new(
            "remote-1",
            "/srv/work space/project",
            "/var/lib/alex/wordcraft/profiles/remote-1",
        )
        .unwrap();

        let runtime = WordcraftRemoteRuntime::new(
            "/opt/alex/wordcraft/current/bin/wordcraft-cli",
            "1.0.0",
            true,
        )
        .unwrap();
        let command = spec.build_remote_headless_command(&runtime).unwrap();
        assert!(command.starts_with(
            "cd '/srv/work space/project' && WORDCRAFT_PROFILE_DIR=/var/lib/alex/wordcraft/profiles/remote-1"
        ));
        assert!(command.contains("--automation-read-root '/srv/work space/project'"));
        assert!(command.contains("--automation-write-root '/srv/work space/project'"));
        assert!(!command.contains("/Users/") && !command.contains("\\\\"));
    }

    #[cfg(feature = "remote-ssh-concrete")]
    #[test]
    fn remote_headless_command_rejects_non_posix_roots() {
        let runtime = WordcraftRemoteRuntime::new(
            "/opt/alex/wordcraft/current/bin/wordcraft-cli",
            "1.0.0",
            true,
        )
        .unwrap();
        let relative = WordcraftSessionSpec::new("remote-1", "workspace", "/var/lib/alex")
            .unwrap()
            .build_remote_headless_command(&runtime)
            .expect_err("relative workspace root must be rejected");
        assert!(relative.to_string().contains("absolute POSIX"));

        let windows = WordcraftSessionSpec::new("remote-1", "C:\\workspace", "/var/lib/alex")
            .unwrap()
            .build_remote_headless_command(&runtime)
            .expect_err("Windows workspace root must be rejected");
        assert!(windows.to_string().contains("absolute POSIX"));
    }

    #[cfg(feature = "remote-ssh-concrete")]
    #[test]
    fn remote_runtime_requires_verified_owner_capability() {
        let path = WordcraftRemoteRuntime::new("wordcraft-cli", "1.0.0", true)
            .expect_err("PATH lookup must not be implicit on the owner");
        assert!(path.to_string().contains("absolute POSIX"));

        let unsupported = WordcraftRemoteRuntime::new(
            "/opt/alex/wordcraft/current/bin/wordcraft-cli",
            "1.0.0",
            false,
        )
        .expect_err("MCP capability must be advertised by the owner");
        assert!(unsupported.to_string().contains("does not advertise MCP"));
    }

    #[cfg(all(unix, feature = "remote-ssh-concrete"))]
    #[tokio::test]
    async fn remote_registry_requires_matching_owner_identity() {
        let spec = WordcraftSessionSpec::new(
            "remote-1",
            "/srv/work/project",
            "/var/lib/alex/wordcraft/remote-1",
        )
        .unwrap();
        let runtime = WordcraftRemoteRuntime::new(
            "/opt/alex/wordcraft/current/bin/wordcraft-cli",
            "1.0.0",
            true,
        )
        .unwrap();
        let script = r#"
            while IFS= read -r line; do
                id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
                printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"workspace-fixture","version":"1"}}}\n' "$id"
            done
        "#;
        let transport =
            WorkspaceStdio::spawn_local_process("sh", &["-c".to_string(), script.to_string()])
                .expect("spawn workspace-owner fixture");
        let session = WordcraftRemoteMcpSession::from_workspace_stdio(
            "connection-1",
            &spec,
            &runtime,
            transport,
            MCPServerTimeouts::default(),
        )
        .await
        .expect("owner handshake should succeed");
        let registry = WordcraftRemoteSessionRegistry::new();
        let key = WordcraftRemoteSessionKey::new("connection-1", &spec);
        let attached = registry
            .register(key.clone(), session)
            .await
            .expect("matching owner should register");
        assert!(registry.attached(&key).await.is_some());
        assert_eq!(
            attached.lock().await.identity().workspace_root(),
            "/srv/work/project"
        );

        let wrong_key = key.clone();
        let identity = attached.lock().await.identity().clone();
        let mismatch = WordcraftRemoteSessionIdentity {
            connection_id: "connection-2".to_string(),
            ..identity
        };
        let mismatch_session = WordcraftRemoteMcpSession {
            process: MCPServerProcess::new(
                "wordcraft-remote-1".to_string(),
                "WordCraft remote-1".to_string(),
                MCPServerType::Remote,
            ),
            connection: attached.lock().await.connection().clone(),
            identity: mismatch,
        };
        let error = match registry.register(wrong_key, mismatch_session).await {
            Ok(_) => panic!("wrong owner identity must be refused"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("key does not match"));

        registry
            .stop(&key)
            .await
            .expect("explicit stop should release owner");
        assert!(registry.attached(&key).await.is_none());
    }

    #[test]
    fn routing_follows_live_if_open_else_headless() {
        let headless_spec =
            WordcraftSessionSpec::new("ws-x", "/work/demo", "/state/p/ws-x").unwrap();
        let live_spec = WordcraftLiveBridgeSpec::new("ws-x", 7981, "abcd1234").unwrap();

        // Document not open: headless, regardless of plausible bridge.
        match wordcraft_route_session(false, Some(live_spec.clone()), &headless_spec).unwrap() {
            WordcraftSessionRoute::Headless(session) => {
                assert!(session
                    .args
                    .windows(2)
                    .any(|w| w[0] == "--automation-write-root"));
            }
            other => panic!("expected headless, got {other:?}"),
        }

        // Document open + reachable bridge: live.
        match wordcraft_route_session(true, Some(live_spec.clone()), &headless_spec).unwrap() {
            WordcraftSessionRoute::Live(session) => {
                assert!(session.args.windows(2).any(|w| w[0] == "--connect"));
                assert!(!session.args.iter().any(|a| a.starts_with("--automation")));
            }
            other => panic!("expected live, got {other:?}"),
        }

        // Document open but bridge missing: loud refusal, never a silent flip.
        match wordcraft_route_session(true, None, &headless_spec).unwrap() {
            WordcraftSessionRoute::Refused(reason) => {
                assert!(reason.contains("refusing to race"));
            }
            other => panic!("expected refusal, got {other:?}"),
        }
    }
}
