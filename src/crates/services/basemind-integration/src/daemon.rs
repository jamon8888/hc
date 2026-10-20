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
#[cfg(feature = "basemind")]
pub const BASEMIND_VERSION: &str = "0.34.0";

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
         Restart the daemon with the pinned version: `basemind comms stop && basemind comms start`."
    )
}
