//! WordCraft editor surface API: capability status and local document open.
//!
//! Deliberately *local-only* on this host (map ticket 06): the visible editor is
//! a `ControllerLocal` surface, and documents living in remote workspaces are
//! refused loudly until the remote provisioning design lands. Status reporting
//! is the single factual record from `alex-services-core::wordcraft`
//! (explicit path -> system PATH -> managed component), so UI entries and the
//! agent derive the same answer; a missing or unusable binary degrades loudly
//! with remediation the user can act on — never a silent local fallback.

use crate::api::app_state::AppState;
use crate::api::path_target::{resolve_desktop_path_target, DesktopPathTarget};
use alex_core::infrastructure::get_path_manager_arc;
use alex_core::service::wordcraft_bridge::{
    get_global_wordcraft_bridge, WordcraftEmbeddedCommandResponse,
};
use alex_core::util::process_manager;
use alex_services_core::wordcraft::{
    WordcraftCapability, WordcraftProbe, WordcraftStatus, WordcraftWebCapability,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

const WORDCRAFT_DOC_EXTENSIONS: &[&str] = &["docx"];

static WORDCRAFT_PROBE: OnceLock<WordcraftProbe> = OnceLock::new();

fn wordcraft_probe() -> Result<&'static WordcraftProbe, String> {
    WORDCRAFT_PROBE
        .get_or_init(|| WordcraftProbe::new(get_path_manager_arc().managed_runtimes_dir()));
    WORDCRAFT_PROBE
        .get()
        .ok_or_else(|| "WordCraft capability probe is unavailable".to_string())
}

/// WordCraft capability record (`available | unsupported | not-installed |
/// error`), optionally forcing a fresh detection (`recheck: true` — the Recheck
/// action).
#[tauri::command]
pub async fn wordcraft_status_command(recheck: bool) -> Result<WordcraftCapability, String> {
    let probe = wordcraft_probe()?;
    if recheck {
        Ok(probe.recheck())
    } else {
        Ok((*probe.current()).clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordcraftOpenDocumentRequest {
    /// Document path (workspace-relative paths resolve through the workspace
    /// service; runtime URIs resolve through the runtime roots). `None` opens
    /// the WordCraft start screen (its own blank/templates flow) for the
    /// "New WordCraft document" entry.
    #[serde(default)]
    pub path: Option<String>,
    /// Explicit workspace identity for resolution; defaults to the active one.
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// Legacy field, still refused loudly: targets a remote workspace.
    #[serde(default)]
    pub connection_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordcraftOpenDocumentResponse {
    pub app_path: String,
    pub control_host: String,
    pub control_port: u16,
    /// Isolated WordCraft profile dir; the control key file lands inside it as
    /// `control-key.local-<port>` (read only by Alex's desktop adapter,
    /// never logged, never exposed to the agent).
    pub profile_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordcraftWebHostResponse {
    pub url: String,
    pub origin: String,
    pub session_id: String,
    pub version: String,
    pub protocol_version: u32,
    pub command_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordcraftEmbeddedCommandResponseRequest {
    pub request_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
    pub ok: bool,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Completes an Agent request that was delivered to the visible embedded
/// WordCraft iframe. The desktop adapter never executes the operation itself;
/// it only returns the iframe's correlated result to Core.
#[tauri::command]
pub async fn wordcraft_complete_embedded_command(
    request: WordcraftEmbeddedCommandResponseRequest,
) -> Result<(), String> {
    if let Some(session_id) = request.session_id.as_deref() {
        touch_wordcraft_web_session(session_id);
    }
    get_global_wordcraft_bridge()
        .complete(WordcraftEmbeddedCommandResponse {
            request_id: request.request_id,
            ok: request.ok,
            result: request.result,
            error: request.error,
        })
        .await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordcraftWebReleaseRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WordcraftWebHostRequest {}

struct WordcraftWebSession {
    last_activity: Instant,
    frontend_workbench: Arc<crate::frontend_workbench::FrontendWorkbenchManager>,
}

static WORDCRAFT_WEB_SESSIONS: OnceLock<Mutex<HashMap<String, WordcraftWebSession>>> =
    OnceLock::new();
static WORDCRAFT_WEB_REAPER_STARTED: OnceLock<()> = OnceLock::new();

const WORDCRAFT_WEB_SESSION_REAP_INTERVAL: Duration = Duration::from_secs(15 * 60);
const WORDCRAFT_WEB_SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);

fn wordcraft_web_sessions() -> &'static Mutex<HashMap<String, WordcraftWebSession>> {
    WORDCRAFT_WEB_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn touch_wordcraft_web_session(session_id: &str) {
    if let Ok(mut sessions) = wordcraft_web_sessions().lock() {
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = Instant::now();
        }
    }
}

fn start_wordcraft_web_session_reaper() {
    if WORDCRAFT_WEB_REAPER_STARTED.set(()).is_err() {
        return;
    }

    tokio::spawn(async {
        let mut interval = tokio::time::interval(WORDCRAFT_WEB_SESSION_REAP_INTERVAL);
        loop {
            interval.tick().await;
            match wordcraft_web_sessions().lock() {
                Ok(mut sessions) => {
                    let expired = sessions
                        .iter()
                        .filter(|(_, session)| {
                            session.last_activity.elapsed() >= WORDCRAFT_WEB_SESSION_IDLE_TIMEOUT
                        })
                        .map(|(session_id, _)| session_id.clone())
                        .collect::<Vec<_>>();
                    for session_id in expired {
                        if let Some(session) = sessions.remove(&session_id) {
                            session
                                .frontend_workbench
                                .unregister_wordcraft_surface(&session_id);
                        }
                    }
                }
                Err(_) => {
                    log::warn!("WordCraft web session registry is poisoned; skipping session reap");
                }
            }
        }
    });
}

fn wordcraft_web_url(origin: &str, token: &str, session_id: &str) -> String {
    format!(
        "{origin}/wordcraft/{session_id}/{token}/index.html?wordcraftToken={token}&sessionId={session_id}&host=parent"
    )
}

fn wordcraft_frontend_origin_for(is_windows: bool) -> &'static str {
    if is_windows {
        "http://alex-ui.localhost"
    } else {
        "alex-ui://localhost"
    }
}

fn wordcraft_frontend_origin() -> &'static str {
    wordcraft_frontend_origin_for(cfg!(target_os = "windows"))
}

#[tauri::command]
pub async fn wordcraft_web_host_command(
    app: AppHandle,
    _request: Option<WordcraftWebHostRequest>,
) -> Result<WordcraftWebHostResponse, String> {
    let capability = wordcraft_probe()?.recheck();
    if capability.web.status != WordcraftStatus::Available {
        let message = capability.web.message.clone().unwrap_or_default();
        let remediation = capability.web.remediation.clone().unwrap_or_default();
        return Err(format!(
            "WordCraft embedded editor is not available ({}). {} {}",
            capability.web.status, message, remediation
        )
        .trim()
        .to_string());
    }

    let artifact_path = capability
        .web
        .artifact_path
        .clone()
        .ok_or_else(|| "WordCraft web capability has no artifact path".to_string())?;
    let artifact = PathBuf::from(&artifact_path);
    if !artifact
        .join(alex_services_core::wordcraft::WORDCRAFT_WEB_ENTRYPOINT)
        .is_file()
    {
        return Err("WordCraft web bundle entrypoint is unavailable".to_string());
    }

    let token = Uuid::new_v4().simple().to_string();
    let session_id = Uuid::new_v4().simple().to_string();
    let frontend_workbench = app
        .state::<Arc<crate::frontend_workbench::FrontendWorkbenchManager>>()
        .inner()
        .clone();
    frontend_workbench.register_wordcraft_surface(session_id.clone(), artifact, token.clone())?;
    start_wordcraft_web_session_reaper();
    wordcraft_web_sessions()
        .lock()
        .map_err(|_| "WordCraft web session registry is poisoned".to_string())?
        .insert(
            session_id.clone(),
            WordcraftWebSession {
                last_activity: Instant::now(),
                frontend_workbench,
            },
        );

    let origin = wordcraft_frontend_origin();
    Ok(WordcraftWebHostResponse {
        // The session identity is part of the browser handshake, not a secret;
        // the token remains the only authorization credential in the URL.
        // The upstream web shell only installs the parent bridge when this
        // explicit mode flag is present. Keep it in the desktop-issued URL so
        // the iframe cannot silently render an unhosted, download-only copy.
        url: wordcraft_web_url(origin, &token, &session_id),
        origin: origin.to_string(),
        session_id,
        version: capability
            .web
            .version
            .ok_or_else(|| "WordCraft web capability has no validated version".to_string())?,
        protocol_version: capability.web.protocol_version.ok_or_else(|| {
            "WordCraft web capability has no validated protocol version".to_string()
        })?,
        command_capabilities: capability.web.command_capabilities,
    })
}

#[tauri::command]
pub async fn wordcraft_web_release_command(
    request: WordcraftWebReleaseRequest,
) -> Result<(), String> {
    if let Some(session) = wordcraft_web_sessions()
        .lock()
        .map_err(|_| "WordCraft web session registry is poisoned".to_string())?
        .remove(&request.session_id)
    {
        session
            .frontend_workbench
            .unregister_wordcraft_surface(&request.session_id);
    }
    Ok(())
}

fn reject_raw_document_input(raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err("Document path must not be empty".to_string());
    }
    if raw.chars().any(|c| c.is_control()) {
        return Err("Document path must not contain control characters".to_string());
    }
    Ok(())
}

fn validate_local_document(document: &Path) -> Result<(), String> {
    if !document.is_file() {
        return Err(format!(
            "Document path does not exist or is not a file: {}",
            document.display()
        ));
    }
    let extension = document
        .extension()
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let extension = extension.to_string_lossy().to_string();
    if !WORDCRAFT_DOC_EXTENSIONS.contains(&extension.as_str()) {
        return Err(format!(
            "WordCraft day one opens .docx documents only (got .{extension})"
        ));
    }
    Ok(())
}

/// Resolves the request path against the desktop host (workspace-relative and
/// runtime-URI forms included) and refuses remote targets loudly. WordCraft is
/// spawned on the local host with the resolved local document path.
async fn resolve_local_document(
    state: &AppState,
    raw: &str,
    workspace_id: Option<&str>,
) -> Result<PathBuf, String> {
    reject_raw_document_input(raw)?;
    let target = resolve_desktop_path_target(state, raw, None, workspace_id).await?;
    let resolved = match target {
        DesktopPathTarget::Local { resolved_path, .. } => resolved_path,
        DesktopPathTarget::Remote { requested_path, .. } => {
            return Err(format!(
                "WordCraft document editing runs on the local workspace only; \
                 remote-workspace document provisioning is not supported yet ({}).",
                requested_path
            ));
        }
    };
    // Legacy local-tree paths may stay workspace-relative here; finalize them
    // against the active workspace root so WordCraft always gets an absolute
    // path.
    let resolved = if resolved.is_absolute() {
        resolved
    } else {
        let workspace = state
            .workspace_service
            .get_current_workspace()
            .await
            .ok_or_else(|| {
                "Cannot open a workspace-relative document without an active workspace".to_string()
            })?;
        workspace.root_path.join(resolved)
    };
    validate_local_document(&resolved)?;
    Ok(resolved)
}

/// Pure loud-refusal decision for launching the visible editor (kept separate
/// so the loud-unavailable contract is unit-testable without spawning). The
/// reason strings embed the shared record verbatim (status, message,
/// remediation); `None` means the visible editor may launch.
fn visible_editor_rejection_reason(capability: &WordcraftCapability) -> Option<String> {
    match capability.status {
        WordcraftStatus::Available => {}
        other => {
            // Loud degradation: the unfetchable state and its remediation come
            // straight from the shared record so every surface reads alike.
            let message = capability.message.clone().unwrap_or_default();
            let remediation = capability.remediation.clone().unwrap_or_default();
            return Some(
                format!("WordCraft is not available ({other}). {message} {remediation}")
                    .trim()
                    .to_string(),
            );
        }
    }

    // The visible editor needs the app binary; CLI-only installs are fine for
    // headless/agent work but cannot show a window.
    if capability.app.is_none() {
        return Some(
            "WordCraft's visible editor requires the wordcraft app binary; only \
             wordcraft-cli was found. Agent editing still works; install the full \
             WordCraft build for the editor window."
                .to_string(),
        );
    }
    None
}

fn free_loopback_control_port() -> Result<u16, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| format!("Failed to reserve a WordCraft control port: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to read the reserved WordCraft control port: {e}"))?
        .port();
    drop(listener);
    Ok(port)
}

fn wordcraft_session_profile_dir(document_path: Option<&str>) -> Result<PathBuf, String> {
    let base = get_path_manager_arc()
        .user_data_dir()
        .join("wordcraft")
        .join("profiles");
    std::fs::create_dir_all(&base)
        .map_err(|e| format!("Failed to create WordCraft profile root: {e}"))?;

    // Unique per session: document stem (when any) plus a short fingerprint of
    // the seed, the pid, and a monotonic per-process session sequence, so two
    // opens in the same process never derive the same profile dir (pid alone is
    // constant here) and repeated opens do not collide with recovery copies of
    // previously lost sessions.
    let unique = {
        static SESSION_SEQ: AtomicU64 = AtomicU64::new(0);
        let mut seed = String::new();
        if let Some(path) = document_path {
            seed.push_str(
                &PathBuf::from(path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "document".to_string()),
            );
        } else {
            seed.push_str("start-screen");
        }
        let seq = SESSION_SEQ.fetch_add(1, Ordering::Relaxed);
        let digest = alex_services_core::wordcraft::session_fingerprint(&[
            seed.as_bytes(),
            std::process::id().to_le_bytes().as_slice(),
            seq.to_le_bytes().as_slice(),
        ]);
        format!("{seed}-{}", &digest[..std::cmp::min(12, digest.len())])
    };
    let dir = base.join(unique);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create the WordCraft session profile dir: {e}"))?;
    Ok(dir)
}

/// Opens a local document in the visible WordCraft editor as its own OS window
/// with a live control channel (the `live-if-open` route's prerequisite).
#[tauri::command]
pub async fn wordcraft_open_document_command(
    state: State<'_, AppState>,
    request: WordcraftOpenDocumentRequest,
) -> Result<WordcraftOpenDocumentResponse, String> {
    // Legacy remote-pointer callers are refused before anything resolves;
    // remote targets are also refused inside resolution below.
    if request.connection_id.is_some() {
        return Err(
            "WordCraft document editing runs on the local workspace only; \
             remote-workspace document provisioning is not supported yet."
                .to_string(),
        );
    }
    let document = match request.path.as_deref() {
        Some(raw) => {
            Some(resolve_local_document(state.inner(), raw, request.workspace_id.as_deref()).await?)
        }
        None => None,
    };

    let capability = wordcraft_probe()?.recheck();
    if let Some(reason) = visible_editor_rejection_reason(&capability) {
        return Err(reason);
    }
    let app = capability
        .app
        .as_ref()
        .expect("available with an app binary");
    let app_path = app
        .resolved_path
        .clone()
        .unwrap_or_else(|| app.command.clone());

    let control_port = free_loopback_control_port()?;
    let mut command = process_manager::create_command(&app_path);
    if let Some(document) = document.as_ref() {
        command.arg(document);
    }
    command.arg("--control").arg(control_port.to_string());

    let profile_dir = wordcraft_session_profile_dir(document.as_ref().and_then(|p| p.to_str()))?
        .to_string_lossy()
        .to_string();
    command.env("WORDCRAFT_PROFILE_DIR", &profile_dir);

    // The editor window is user-facing and outlives Alex; it is spawned
    // detached from the managed child tree and never killed by app teardown.
    command
        .spawn()
        .map_err(|e| format!("Failed to start WordCraft: {e}"))?;

    Ok(WordcraftOpenDocumentResponse {
        app_path,
        control_host: "127.0.0.1".to_string(),
        control_port,
        profile_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alex_core::service::runtime::RuntimeSource;
    use alex_services_core::managed_runtime::ResolvedCommand;

    fn capability(
        status: WordcraftStatus,
        app: Option<ResolvedCommand>,
        message: Option<&str>,
        remediation: Option<&str>,
    ) -> WordcraftCapability {
        WordcraftCapability {
            status,
            cli: None,
            app,
            version: None,
            message: message.map(str::to_string),
            remediation: remediation.map(str::to_string),
            checked_at_epoch_ms: 0,
            web: WordcraftWebCapability::default(),
        }
    }

    #[test]
    fn rejection_reason_is_none_only_for_a_launchable_record() {
        let launchable = capability(
            WordcraftStatus::Available,
            Some(ResolvedCommand {
                command: "wordcraft".to_string(),
                source: RuntimeSource::Managed,
                resolved_path: Some("/managed/wordcraft".to_string()),
            }),
            None,
            None,
        );
        assert!(visible_editor_rejection_reason(&launchable).is_none());
    }

    #[test]
    fn cli_only_installs_refuse_the_window_but_keep_agent_editing() {
        let cli_only = capability(
            WordcraftStatus::Available,
            None,
            Some("CLI not runnable here; headless agent editing only."),
            None,
        );
        let reason = visible_editor_rejection_reason(&cli_only).expect("cli-only refused");
        assert!(reason.contains("visible editor requires the wordcraft app"));
        assert!(
            reason.contains("Agent editing still works"),
            "unadmitting: {reason}"
        );
    }

    #[test]
    fn loud_unavailable_states_surface_the_record_verbatim() {
        let not_installed = capability(
            WordcraftStatus::NotInstalled,
            None,
            Some("Managed component not downloaded."),
            Some("Download from the settings card, then Recheck."),
        );
        let reason = visible_editor_rejection_reason(&not_installed).expect("refused");
        assert!(
            reason.contains("not available (not-installed)"),
            "missing status: {reason}"
        );
        assert!(
            reason.contains("Managed component not downloaded."),
            "missing message: {reason}"
        );
        assert!(
            reason.contains("then Recheck."),
            "missing remediation: {reason}"
        );
    }

    #[test]
    fn raw_document_inputs_are_rejected_strictly() {
        let empty = reject_raw_document_input("   ").expect_err("empty refused");
        assert!(empty.contains("must not be empty"));

        let control = reject_raw_document_input("/work/a\u{7}b.docx").expect_err("control refused");
        assert!(control.contains("control characters"));

        assert!(reject_raw_document_input("notes/report.docx").is_ok());
    }

    #[test]
    fn local_document_paths_are_validated() {
        let extension = validate_local_document(Path::new("definitely-missing-alex-path.docx"))
            .expect_err("missing file refused");
        assert!(
            extension.contains("does not exist"),
            "unexpected: {extension}"
        );

        let dir = std::env::temp_dir().join(format!(
            "alex-wordcraft-api-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let wrong_ext = dir.join("report.rtf");
        std::fs::write(&wrong_ext, b"x").unwrap();
        let err = validate_local_document(&wrong_ext).expect_err("rtf refused");
        assert!(err.contains(".docx"), "unexpected: {err}");

        let ok_path = dir.join("Report.DOCX"); // case-insensitive extension
        std::fs::write(&ok_path, b"x").unwrap();
        assert!(validate_local_document(&ok_path).is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn control_port_scan_is_loopback_only() {
        let port = free_loopback_control_port().unwrap();
        assert!(port > 0);
    }

    #[test]
    fn session_profile_dirs_are_unique_per_document() {
        let a = wordcraft_session_profile_dir(None).unwrap();
        let b = wordcraft_session_profile_dir(None).unwrap();
        assert_ne!(a, b, "start-screen sessions must not collide");
        let base_exists = a.parent().unwrap().exists();
        assert!(base_exists);
    }

    #[test]
    fn hosted_web_url_enables_the_upstream_parent_bridge() {
        let url = wordcraft_web_url("alex-ui://localhost", "token", "session-1");
        assert_eq!(
            url,
            "alex-ui://localhost/wordcraft/session-1/token/index.html?wordcraftToken=token&sessionId=session-1&host=parent"
        );
    }

    #[test]
    fn hosted_web_url_matches_the_windows_workbench_origin() {
        assert_eq!(
            wordcraft_frontend_origin_for(true),
            "http://alex-ui.localhost"
        );
        assert_eq!(
            wordcraft_web_url(
                wordcraft_frontend_origin_for(true),
                "token",
                "session-1"
            ),
            "http://alex-ui.localhost/wordcraft/session-1/token/index.html?wordcraftToken=token&sessionId=session-1&host=parent"
        );
    }
}
