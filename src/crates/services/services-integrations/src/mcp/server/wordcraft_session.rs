//! WordCraft headless editing sessions served over the MCP stdio registry.
//!
//! One `wordcraft-cli mcp` server per workspace session with a document open
//! (map decision, ticket 05): the spawn is the MCP runtime's local-server
//! lifecycle; this module owns only the session *definition* — the resolved
//! concurrency-free argv/env/working-directory surfaced to the registry — plus
//! the shared idle-reap decision. Spawning is done exclusively through the
//! `process_runtime`/`process_manager` path inside `MCPConnection`
//! (`CREATE_NO_WINDOW` policy lives there, never re-implemented here).

use crate::mcp::{MCPRuntimeError, MCPRuntimeResult};
use std::time::Duration;

/// The headless CLI command resolved by the managed-runtime order
/// (explicit path → system PATH → managed component, see services-core
/// `wordcraft` / `managed_runtime`).
pub const WORDCRAFT_CLI_COMMAND: &str = "wordcraft-cli";

/// Idle threshold after the last tool call before the session owner may reap a
/// headless WordCraft server.
pub const WORDCRAFT_IDLE_REAP: Duration = Duration::from_secs(10 * 60);

/// Author name embedded into tracked changes; never contains secrets.
pub const WORDCRAFT_AUTHOR_PREFIX: &str = "OpenBitFun Agent";

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
    /// Isolated WordCraft profile dir under OpenBitFun state
    /// (isolated recovery copies; never the user's own WordCraft profile).
    profile_dir: String,
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
        assert_eq!(s.author(), "OpenBitFun Agent (ws-a1b2)");
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
                "OpenBitFun Agent (ws-a1b2)",
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
