//! WordCraft editor integration capability record.
//!
//! This module is the Rust-side owner of the WordCraft capability projection:
//! one factual record (`available` / `unsupported` / `not-installed` / `error`)
//! derived on demand from the command discovery order
//! **explicit path → system PATH → OpenBitFun managed runtime component**, cached
//! until an explicit `recheck`. Worker crates may add product policy (mode /
//! disable semantics riding `external_integration_policy`) on top; this record
//! stays purely factual so every surface derives the same answer.

use crate::managed_runtime::{ManagedRuntimeResolver, ResolvedCommand};
use crate::process_manager::create_command;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// The headless CLI binary (`wordcraft-cli`) — the agent's primary edit path.
pub const WORDCRAFT_CLI_COMMAND: &str = "wordcraft-cli";
/// The visible application binary (`wordcraft`) — the live editor surface.
pub const WORDCRAFT_APP_COMMAND: &str = "wordcraft";
/// Managed runtime component name (resolved as `<runtime_root>/wordcraft/current`).
pub const MANAGED_WORDCRAFT_COMPONENT: &str = "wordcraft";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WordcraftStatus {
    Available,
    Unsupported,
    NotInstalled,
    Error,
}

/// Lowercase wire-ish names so messages embedding the status read the same on
/// every surface ("WordCraft is not available (not-installed).").
impl std::fmt::Display for WordcraftStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            WordcraftStatus::Available => "available",
            WordcraftStatus::Unsupported => "unsupported",
            WordcraftStatus::NotInstalled => "not-installed",
            WordcraftStatus::Error => "error",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordcraftCapability {
    pub status: WordcraftStatus,
    /// Resolution of the headless CLI; `None` only when not installed.
    pub cli: Option<ResolvedCommand>,
    /// Resolution of the visible app; optional (CLI-only installs are valid).
    pub app: Option<ResolvedCommand>,
    /// Reported by `wordcraft-cli --version` during a recheck, when a probe ran.
    pub version: Option<String>,
    /// Status detail (English-only), present for every non-available status.
    pub message: Option<String>,
    /// User-facing remediation guidance (English-only).
    pub remediation: Option<String>,
    pub checked_at_epoch_ms: u64,
}

/// On-demand cached WordCraft capability probe.
pub struct WordcraftProbe {
    runtime_root: PathBuf,
    cache: RwLock<Option<Arc<WordcraftCapability>>>,
}

impl WordcraftProbe {
    pub fn new(runtime_root: impl Into<PathBuf>) -> Self {
        Self {
            runtime_root: runtime_root.into(),
            cache: RwLock::new(None),
        }
    }

    pub fn runtime_root(&self) -> &std::path::Path {
        &self.runtime_root
    }

    /// Cached record; probes on first use. Never invents an availability answer
    /// while a probe has not resolved yet — callers that need freshness use
    /// [`WordcraftProbe::recheck`].
    pub fn current(&self) -> Arc<WordcraftCapability> {
        if let Some(cached) = self.cached() {
            return cached;
        }
        let fresh = Arc::new(self.probe());
        if let Ok(mut cache) = self.cache.write() {
            *cache = Some(Arc::clone(&fresh));
        }
        fresh
    }

    /// Fresh detection with a `--version` probe, replacing the cached record.
    pub fn recheck(&self) -> WordcraftCapability {
        let fresh = self.probe();
        if let Ok(mut cache) = self.cache.write() {
            *cache = Some(Arc::new(fresh.clone()));
        }
        fresh
    }

    /// Drops the cached record so the next `current()` re-derives.
    pub fn invalidate(&self) {
        if let Ok(mut cache) = self.cache.write() {
            *cache = None;
        }
    }

    fn cached(&self) -> Option<Arc<WordcraftCapability>> {
        self.cache.read().ok().and_then(|c| c.clone())
    }

    /// One-shot detection without touching the cache.
    pub fn probe(&self) -> WordcraftCapability {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let resolver = ManagedRuntimeResolver::new(self.runtime_root.clone());

        let cli = resolver
            .resolve_command(WORDCRAFT_CLI_COMMAND)
            .map(|mut r| {
                r.command = r.resolved_path.clone().unwrap_or(r.command);
                r
            });

        let Some(cli) = cli else {
            return WordcraftCapability {
                status: WordcraftStatus::NotInstalled,
                cli: None,
                app: None,
                version: None,
                message: Some(format!(
                    "wordcraft-cli not found (runtime root: {})",
                    resolver.runtime_root_display()
                )),
                remediation: Some(format!(
                    "Install WordCraft {} or later (upstream release archives land as the \
                     managed component at <runtime-root>/{}/current), or point the \
                     configured command at the upstream wordcraft-cli binary, then Recheck.",
                    pinned_version(),
                    MANAGED_WORDCRAFT_COMPONENT
                )),
                checked_at_epoch_ms: now_ms,
            };
        };

        // App is optional: CLI-only installs still serve headless agent editing.
        let app = resolver.resolve_command(WORDCRAFT_APP_COMMAND);
        let has_app = app.is_some();

        match run_version_probe(cli.command.as_str()) {
            VersionProbe::Ok { output } => WordcraftCapability {
                status: WordcraftStatus::Available,
                cli: Some(cli),
                app,
                version: Some(output),
                message: None,
                remediation: None,
                checked_at_epoch_ms: now_ms,
            },
            VersionProbe::UnsupportedBinary { detail } => WordcraftCapability {
                status: WordcraftStatus::Unsupported,
                cli: Some(cli),
                app,
                version: None,
                message: Some(detail),
                remediation: Some(format!(
                    "Download the upstream WordCraft build for this platform, verify it \
                     against the release SHA256SUMS.txt, then Recheck."
                )),
                checked_at_epoch_ms: now_ms,
            },
            VersionProbe::Failed { detail } => {
                // With no visible app on this host, headless editing still works.
                if has_app {
                    WordcraftCapability {
                        status: WordcraftStatus::Error,
                        cli: Some(cli),
                        app,
                        version: None,
                        message: Some(detail),
                        remediation: Some(
                            "The WordCraft CLI exists but did not start. Reinstall the \
                             pinned component, then Recheck."
                                .to_string(),
                        ),
                        checked_at_epoch_ms: now_ms,
                    }
                } else {
                    WordcraftCapability {
                        status: WordcraftStatus::Available,
                        cli: Some(cli),
                        app: None,
                        version: None,
                        message: Some(format!(
                            "CLI not runnable here ({}); headless agent editing only.",
                            detail
                        )),
                        remediation: None,
                        checked_at_epoch_ms: now_ms,
                    }
                }
            }
        }
    }
}

enum VersionProbe {
    Ok { output: String },
    UnsupportedBinary { detail: String },
    Failed { detail: String },
}

fn run_version_probe(command: &str) -> VersionProbe {
    let output = create_command(command).arg("--version").output();
    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if text.is_empty() {
                VersionProbe::Failed {
                    detail: "empty --version output".to_string(),
                }
            } else {
                VersionProbe::Ok { output: text }
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = {
                let tail: String = stderr
                    .lines()
                    .last()
                    .unwrap_or("exit code != 0")
                    .trim()
                    .chars()
                    .take(160)
                    .collect();
                if tail.is_empty() {
                    format!("exit code {:?}", output.status.code())
                } else {
                    tail
                }
            };
            VersionProbe::Failed { detail }
        }
        Err(err) => {
            if err.raw_os_error() == Some(8) {
                // ENOEXEC: right name, wrong platform/architecture binary.
                VersionProbe::UnsupportedBinary {
                    detail: err.to_string(),
                }
            } else {
                VersionProbe::Failed {
                    detail: err.to_string(),
                }
            }
        }
    }
}

/// Pinned upstream version the managed component is expected to serve. Kept as a
/// const so a product release chooses the bump deliberately (ticket 01 policy).
pub const PINNED_WORDCRAFT_VERSION: &str = "0.7.0";

#[cfg(feature = "wordcraft")]
pub use fingerprint::session_fingerprint;

fn pinned_version() -> &'static str {
    PINNED_WORDCRAFT_VERSION
}

#[cfg(feature = "wordcraft")]
pub mod fingerprint {
    //! Hex fingerprint helper for session-local identifiers (profile dirs).

    use sha2::{Digest, Sha256};

    /// Hashes the concatenated parts and returns the lowercase hex SHA-256.
    /// Callers truncate; this stays untruncated so collision risk lives with
    /// the caller-visible prefix length only.
    pub fn session_fingerprint(parts: &[&[u8]]) -> String {
        let mut hasher = Sha256::new();
        for part in parts {
            hasher.update(part);
        }
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

#[cfg(feature = "wordcraft")]
pub mod checksum {
    //! Release-archive verification against upstream `SHA256SUMS.txt`.

    use sha2::{Digest, Sha256};
    use std::path::Path;

    #[derive(Debug)]
    pub enum ChecksumError {
        Io(std::io::Error),
        Mismatch {
            file: String,
            expected: String,
            actual: String,
        },
        Missing {
            file: String,
        },
        Unlisted {
            file: String,
        },
    }

    impl std::fmt::Display for ChecksumError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ChecksumError::Io(e) => write!(f, "checksum io error: {}", e),
                ChecksumError::Mismatch {
                    file,
                    expected,
                    actual,
                } => {
                    write!(
                        f,
                        "checksum mismatch for {}: expected {}, got {}",
                        file, expected, actual
                    )
                }
                ChecksumError::Missing { file } => write!(f, "listed file missing: {}", file),
                ChecksumError::Unlisted { file } => {
                    write!(f, "file is not listed in SHA256SUMS: {}", file)
                }
            }
        }
    }

    /// Verify every file listed in `sums_text` under `dir`, and refuse files
    /// present in `dir` that the sums file does not list.
    pub fn verify_directory_against_sha256sums(
        dir: &Path,
        sums_text: &str,
    ) -> Result<(), ChecksumError> {
        let mut listed: Vec<(String, String)> = Vec::new();
        for line in sums_text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((hash, rest)) = line.split_once(' ') else {
                continue;
            };
            let name = rest.trim_start().trim_start_matches('*').to_string();
            if name.is_empty() || hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            listed.push((hash.to_ascii_lowercase(), name));
        }
        if listed.is_empty() {
            return Err(ChecksumError::Missing {
                file: "(SHA256SUMS)".to_string(),
            });
        }

        let mut seen: Vec<String> = Vec::new();
        for (hash, name) in &listed {
            let path = dir.join(name);
            let bytes = std::fs::read(&path).map_err(ChecksumError::Io)?;
            let actual = hex(Sha256::digest(&bytes).as_slice());
            if &actual != hash {
                return Err(ChecksumError::Mismatch {
                    file: name.clone(),
                    expected: hash.clone(),
                    actual,
                });
            }
            seen.push(name.clone());
        }

        for entry in dir.read_dir().map_err(ChecksumError::Io)? {
            let entry = entry.map_err(ChecksumError::Io)?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "SHA256SUMS.txt" || name == "SHA256SUMS" {
                continue;
            }
            if !entry.path().is_file() {
                continue;
            }
            if !seen.contains(&name) {
                return Err(ChecksumError::Unlisted { file: name });
            }
        }
        Ok(())
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fs;
        use std::path::PathBuf;

        fn temp_dir(label: &str) -> PathBuf {
            let p = std::env::temp_dir().join(format!(
                "openbitfun-wordcraft-checksum-{}-{}-{}",
                label,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&p).unwrap();
            p
        }

        #[test]
        fn verify_directory_against_sha256sums_accepts_matching_files() {
            let dir = temp_dir("ok");
            fs::write(dir.join("wordcraft-cli"), b"payload").unwrap();
            fs::write(dir.join("LICENSE-MIT"), b"MIT").unwrap();
            let mut sums = String::new();
            for name in ["wordcraft-cli", "LICENSE-MIT"] {
                let digest = sha256_file(&dir.join(name));
                sums.push_str(&format!("{}  {}\n", digest, name));
            }
            assert!(
                verify_directory_against_sha256sums(&dir, &sums).is_ok(),
                "expected ok, got {:?}",
                verify_directory_against_sha256sums(&dir, &sums)
            );
            let _ = fs::remove_dir_all(dir);
        }

        #[test]
        fn verify_directory_against_sha256sums_detects_mismatch_and_unlisted() {
            let dir = temp_dir("bad");
            fs::write(dir.join("wordcraft-cli"), b"payload").unwrap();
            let sums = format!("{}  wordcraft-cli\n", "0".repeat(64));
            let err = verify_directory_against_sha256sums(&dir, &sums).unwrap_err();
            assert!(matches!(err, ChecksumError::Mismatch { .. }));

            // With a correct sums entry that omits extra.txt, the extra file
            // must be refused as unlisted.
            fs::write(dir.join("extra.txt"), b"extra").unwrap();
            let sums = format!(
                "{}  wordcraft-cli\n",
                sha256_file(&dir.join("wordcraft-cli"))
            );
            let err = verify_directory_against_sha256sums(&dir, &sums).unwrap_err();
            assert!(matches!(err, ChecksumError::Unlisted { file } if file == "extra.txt"));
            let _ = fs::remove_dir_all(dir);
        }

        fn sha256_file(path: &Path) -> String {
            let bytes = fs::read(path).unwrap();
            hex(Sha256::digest(&bytes).as_slice())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_runtime_root(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "openbitfun-wordcraft-cap-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn empty_runtime_root_reports_not_installed_with_remediation() {
        let root = temp_runtime_root("missing");
        let probe = WordcraftProbe::new(root.clone());
        let cap = probe.probe();
        assert_eq!(cap.status, WordcraftStatus::NotInstalled);
        assert!(cap.cli.is_none());
        assert!(cap
            .remediation
            .as_deref()
            .unwrap_or_default()
            .contains("Recheck"));
        assert!(cap.app.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn explicit_path_command_records_available_without_app() {
        let root = temp_runtime_root("explicit");
        let managed = root.join(MANAGED_WORDCRAFT_COMPONENT).join("current");
        fs::create_dir_all(&managed).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let cli_bin = managed.join("wordcraft-cli");
            fs::write(&cli_bin, b"#!/bin/sh\necho wordcraft-cli 0.7.0\n").unwrap();
            fs::set_permissions(&cli_bin, fs::Permissions::from_mode(0o755)).unwrap();

            let probe = WordcraftProbe::new(root.clone());
            let cap = probe.recheck();
            assert_eq!(
                cap.status,
                WordcraftStatus::Available,
                "capability: {:?}",
                cap
            );
            assert!(
                cap.cli.as_ref().unwrap().resolved_path.as_deref()
                    == Some(cli_bin.to_string_lossy().as_ref()),
                "cli: {:?}",
                cap.cli
            );
            assert_eq!(cap.app, None);
            assert!(cap
                .version
                .as_deref()
                .unwrap_or_default()
                .starts_with("wordcraft-cli"));
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cached_current_is_replaced_by_recheck() {
        let root = temp_runtime_root("cache");
        let probe = WordcraftProbe::new(root.clone());
        let first = probe.current();
        assert_eq!(first.status, WordcraftStatus::NotInstalled);

        let managed = root.join(MANAGED_WORDCRAFT_COMPONENT).join("current");
        fs::create_dir_all(&managed).unwrap();
        fs::write(managed.join("wordcraft-cli"), b"binary").unwrap();

        // Still cached not-installed until recheck.
        assert_eq!(probe.current().status, WordcraftStatus::NotInstalled);
        assert_eq!(probe.recheck().status, WordcraftStatus::Available);
        // current() reflects the refreshed cache with no re-probe overhead.
        assert_eq!(probe.current().status, WordcraftStatus::Available);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalidate_forces_fresh_probe_on_next_current() {
        let root = temp_runtime_root("invalidate");
        let probe = WordcraftProbe::new(root.clone());
        assert_eq!(probe.current().status, WordcraftStatus::NotInstalled);

        let managed = root.join(MANAGED_WORDCRAFT_COMPONENT).join("current");
        fs::create_dir_all(&managed).unwrap();
        let cli_bin = managed.join("wordcraft-cli");
        fs::write(&cli_bin, b"binary").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&cli_bin, fs::Permissions::from_mode(0o755)).unwrap();
            probe.invalidate();
            // No recheck(): invalidate alone must produce a fresh, available answer.
            assert_eq!(probe.current().status, WordcraftStatus::Available);
        }
        let _ = fs::remove_dir_all(root);
    }
}
