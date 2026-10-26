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
use alex_core::util::process_manager;
use alex_services_core::wordcraft::{WordcraftCapability, WordcraftProbe, WordcraftStatus};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use tauri::State;

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
        let extension =
            validate_local_document(Path::new("definitely-missing-alex-path.docx"))
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
}
