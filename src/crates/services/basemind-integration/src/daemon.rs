//! Daemon discovery — find the running basemind daemon's HTTP endpoint.
//!
//! The fork runs its comms broker as a separate process and publishes two
//! things a client needs in a portfile:
//!
//! - line 1: the bound `host:port` the HTTP front-end listens on;
//! - line 2: the process's bearer token, which the HTTP front-end requires
//!   on every request.
//!
//! The portfile lives at `<comms_dir>/http.addr`, where `comms_dir` is
//! `ProjectDirs::from("", "", "basemind").data_dir()/comms`, overridable with
//! the `BASEMIND_COMMS_DIR` environment variable.
//!
//! The token is the identity: the listener is bound to loopback, but
//! loopback alone is not an authentication boundary, so a client that cannot
//! read the portfile cannot talk to the daemon at all.

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use std::path::{Path, PathBuf};

/// The basemind fork version this integration is written against.
///
/// The fork is a separate process, not a Cargo dependency, so the version is
/// pinned here rather than read from a manifest. The pin is the contract the
/// product code was written against: the MCP tool names, the parameter
/// names, and the response shape all belong to this version.
///
/// v0.34.0 is the upstream sync of `Goldziher/basemind` v0.29.3 into the
/// `jamon8888/basemind` fork. The sync keeps the fork's PII, redaction and
/// vault features end to end, and adds the MCP resources and the
/// `redact_text` `file_path` extraction this integration uses.
///
/// v0.35.0 adds the privacy gateway boundary (fork ADR-0015): a workspace
/// carrying a `safe/` mirror is protected, and `redact_text` / `vault` are
/// withheld there from any daemon that does not carry the
/// `BASEMIND_GATEWAY_TOKEN` marker this product sets on its own daemon. It
/// also ships the NER windowing and its coverage report (first released as
/// v0.34.1), which `ner_coverage_refusal` enforces.
#[cfg(feature = "basemind")]
pub const BASEMIND_VERSION: &str = "0.35.0";

/// Environment variable the fork uses to override the comms directory.
#[cfg(feature = "basemind")]
pub const COMMS_DIR_ENV: &str = "BASEMIND_COMMS_DIR";

/// The portfile name inside the comms directory.
#[cfg(feature = "basemind")]
const PORTFILE_NAME: &str = "http.addr";

/// A discovered daemon endpoint.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone)]
pub struct DaemonEndpoint {
    /// The bound `host:port` the HTTP front-end serves on.
    pub addr: String,
    /// The bearer token the front-end requires.
    pub token: String,
}

#[cfg(feature = "basemind")]
impl DaemonEndpoint {
    /// The base URL to post MCP requests to.
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

/// The comms directory the daemon publishes its portfile in.
#[cfg(feature = "basemind")]
pub fn comms_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(COMMS_DIR_ENV) {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    default_comms_dir()
}

/// The platform's default comms directory, matching the fork's own
/// `ProjectDirs::from("", "", "basemind").data_dir()/comms`.
#[cfg(feature = "basemind")]
fn default_comms_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        if let Some(home) = home {
            return home
                .join("Library")
                .join("Application Support")
                .join("basemind")
                .join("comms");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("basemind").join("comms");
        }
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(xdg).join("basemind").join("comms");
        }
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            return home
                .join(".local")
                .join("share")
                .join("basemind")
                .join("comms");
        }
    }
    PathBuf::from("comms")
}

/// The portfile path for a comms directory.
#[cfg(feature = "basemind")]
fn portfile_path(comms_dir: &Path) -> PathBuf {
    comms_dir.join(PORTFILE_NAME)
}

/// Read the daemon's published endpoint.
///
/// Returns an error when no daemon is running, or when the portfile is
/// unreadable or malformed. The caller surfaces that to the user rather than
/// silently degrading: a PII guarantee that quietly stops redacting is not a
/// guarantee.
#[cfg(feature = "basemind")]
pub fn discover() -> Result<DaemonEndpoint> {
    read_endpoint(&portfile_path(&comms_dir()))
}

/// Read the endpoint from a specific portfile.
#[cfg(feature = "basemind")]
pub fn read_endpoint(portfile: &Path) -> Result<DaemonEndpoint> {
    let raw = std::fs::read_to_string(portfile).map_err(|error| {
        anyhow::anyhow!(
            "basemind daemon not reachable: cannot read {} ({error}). Start it with `basemind comms start`.",
            portfile.display()
        )
    })?;
    let mut lines = raw.lines();
    let addr = lines
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .ok_or_else(|| anyhow::anyhow!("basemind portfile {} is empty", portfile.display()))?
        .to_string();
    let token = lines
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "basemind portfile {} carries no bearer token",
                portfile.display()
            )
        })?
        .to_string();
    Ok(DaemonEndpoint { addr, token })
}

/// The version of the running daemon, when it publishes one.
///
/// The daemon writes its pid record next to the portfile, including the
/// fork version it was built from.
#[cfg(feature = "basemind")]
pub fn daemon_version() -> Option<String> {
    let record = std::fs::read_to_string(comms_dir().join("daemon.pid")).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&record).ok()?;
    parsed
        .get("version")
        .and_then(|version| version.as_str())
        .map(str::to_string)
}

/// Check that the running daemon is the version this integration targets.
///
/// A daemon of a different version may answer the same requests with a
/// different shape, so the mismatch is reported rather than discovered later
/// as a confusing parse failure.
#[cfg(feature = "basemind")]
pub fn verify_version() -> Result<()> {
    let Some(version) = daemon_version() else {
        // The daemon does not always publish a version; that is not a
        // mismatch, so it must not fail the call.
        return Ok(());
    };
    if version == BASEMIND_VERSION {
        return Ok(());
    }
    anyhow::bail!(
        "basemind daemon runs {version}, but this build targets {BASEMIND_VERSION}. \
         The client restarts the daemon with the pinned version when it knows the \
         sidecar binary; otherwise run: `basemind comms stop && basemind comms start`."
    )
}

/// Start the daemon if (and only if) none holds the comms directory.
///
/// `comms start` is the fork's own ensure verb: when a daemon already holds
/// the comms directory the spawn is a no-op, and a spawn races are resolved
/// by the fork's comms-directory lock. The HTTP front-end must be opted into
/// with `BASEMIND_ALLOW_HTTP=1`, or the daemon serves nothing `/mcp` can
/// reach and never writes the portfile at all.
#[cfg(feature = "basemind")]
fn ensure(daemon_binary: &Path) -> Result<()> {
    use openbitfun_services_core::process_manager::create_command;

    // A restart is a launch: the daemon the product brings back is the
    // gateway's own, marked like the startup command marks it, so the
    // withheld tools reopen for the restarted process too. A token that
    // cannot be minted fails the ensure — silently spawning an unmarked
    // daemon would refuse the protected-workspace tools once the fork pins
    // the gateway boundary, which is a degradation the caller must see.
    let gateway_token = crate::gateway::generate_gateway_token().map_err(|error| {
        anyhow::anyhow!("basemind daemon ensure could not mint a gateway token: {error}")
    })?;

    let mut command = create_command(daemon_binary);
    command
        .arg("comms")
        .arg("start")
        .env(crate::gateway::GATEWAY_TOKEN_ENV, gateway_token)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    command.env("BASEMIND_ALLOW_HTTP", "1");
    // The fork's comms directory default matches the product's discovery
    // resolution, but an explicit call-side override must win for the
    // spawned daemon as well, or the ensure would start a daemon the
    // caller cannot find.
    if let Some(comms_dir) = std::env::var_os(COMMS_DIR_ENV) {
        command.env(COMMS_DIR_ENV, comms_dir);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| anyhow::anyhow!("basemind daemon ensure failed to spawn: {error}"))
}

/// A daemon endpoint that is true right now.
///
/// The daemon may have self-terminated after its bootstrap window (the
/// fork's design: an abandoned daemon does not linger), and it binds a fresh
/// port on every restart, so a caller must never trust an endpoint it
/// resolved for a previous call. When no endpoint is published and the
/// sidecar binary is known, this brings the daemon back first: the product's
/// PII surface stays available across daemon restarts instead of failing
/// on an endpoint that only existed in the past.
///
/// A daemon of a foreign version is repaired rather than reported: the
/// comms directory is held until the daemon is stopped, so an in-place
/// product upgrade finds the previous build's daemon here, and the version
/// error would otherwise ask the user to restart it by hand. The product
/// owns its sidecar — it stops the stale daemon, brings back a marked one
/// at the pinned version, and only a daemon that survives that (or a
/// foreign daemon with no sidecar binary to restart from) surfaces the
/// version error. Whether a running daemon carries the gateway marker is
/// not observable, so the hand-started case stays the residual risk §18 of
/// the spec records.
#[cfg(feature = "basemind")]
pub async fn discover_or_ensure(
    daemon_binary: Option<std::path::PathBuf>,
    wait_seconds: u64,
) -> Result<DaemonEndpoint> {
    if let Ok(endpoint) = discover() {
        if verify_version().is_ok() {
            return Ok(endpoint);
        }
        let Some(binary) = daemon_binary.as_deref() else {
            // Without the sidecar binary the restart cannot be performed,
            // so the version error is the honest state, not a silent reuse
            // of a daemon this build cannot call.
            verify_version()?;
            return Ok(endpoint);
        };
        stop_daemon(binary)?;
        ensure(binary)?;
        return await_endpoint(wait_seconds).await;
    }
    let Some(binary) = daemon_binary else {
        return discover();
    };
    ensure(&binary)?;
    await_endpoint(wait_seconds).await
}

/// Poll the portfile until the (re)started daemon publishes its endpoint.
#[cfg(feature = "basemind")]
async fn await_endpoint(wait_seconds: u64) -> Result<DaemonEndpoint> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(wait_seconds);
    loop {
        if let Ok(endpoint) = discover() {
            return Ok(endpoint);
        }
        if std::time::Instant::now() >= deadline {
            return Err(discover().err().unwrap_or_else(|| {
                anyhow::anyhow!(
                    "basemind daemon did not publish its endpoint within {wait_seconds}s"
                )
            }));
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

/// Stop the daemon that holds the comms directory, and clear the endpoint
/// it published.
///
/// The fork's `comms stop` verb stops whatever holds the comms directory.
/// The portfile is removed best-effort afterwards so the poll cannot hand a
/// caller the stopped daemon's endpoint before the replacement publishes
/// its own.
#[cfg(feature = "basemind")]
fn stop_daemon(daemon_binary: &Path) -> Result<()> {
    use openbitfun_services_core::process_manager::create_command;

    let mut command = create_command(daemon_binary);
    command
        .arg("comms")
        .arg("stop")
        .stdin(std::process::Stdio::null());
    if let Some(comms_dir) = std::env::var_os(COMMS_DIR_ENV) {
        command.env(COMMS_DIR_ENV, comms_dir);
    }
    let status = command
        .status()
        .map_err(|error| anyhow::anyhow!("basemind comms stop failed to launch: {error}"))?;
    if !status.success() {
        anyhow::bail!("basemind comms stop exited with {status}");
    }
    std::fs::remove_file(portfile_path(&comms_dir())).ok();
    Ok(())
}
