//! Discovery of tool packages that ship inside the OpenBitFun product bundle.
//!
//! Some third-party tools are packaged with the product so a user does not
//! have to install them before they work (see `resources/interpreter/README.md`).
//! Their location depends on how the product was packaged, and the layout
//! differs per platform, so the lookup lives here instead of in the surface
//! that happens to know the path.
//!
//! A package is a directory, not a lone executable: a tool may need helper
//! files next to it. The build step writes a manifest into the package, so
//! this module never has to know how a third party lays its own files out.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

/// The manifest every bundled tool package carries next to its files.
const BUNDLED_TOOL_MANIFEST: &str = "openbitfun-bundled-tool.json";

/// Overrides the directory a bundled tool package is read from.
///
/// The desktop app resolves its own resource directory at startup; this
/// override exists for the cases where that is not enough, such as a build
/// that runs the product from a staging directory.
const BUNDLED_TOOL_DIR_ENV: &str = "OPENBITFUN_BUNDLED_TOOL_DIR";

/// A tool package that ships inside the OpenBitFun product bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundledToolPackage {
    /// The tool's stable id, e.g. `interpreter`.
    pub tool_id: String,
    /// The tool's own version.
    pub version: String,
    /// The package directory that holds the tool's files.
    pub root: PathBuf,
    /// The executable to launch.
    pub entrypoint: PathBuf,
    /// The host the package was built for, e.g. `macos-aarch64`.
    pub host_tag: String,
    /// SHA-256 of the release asset the package was extracted from.
    pub source_sha256: String,
}

/// The on-disk manifest a bundled tool package carries.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundledToolManifest {
    tool_id: String,
    version: String,
    host_tag: String,
    entrypoint: String,
    #[serde(default)]
    source_sha256: String,
}

/// The host tag of the machine this process runs on, e.g. `macos-aarch64`.
///
/// The build step derives the same tag from the target it packaged, so a
/// package built for another host is never launched by accident.
pub fn bundled_tool_host_tag() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// The bundled package for `tool_id`, if this build ships one for this host.
pub fn bundled_tool_package(tool_id: &str) -> Option<BundledToolPackage> {
    let host_tag = bundled_tool_host_tag();
    bundled_tool_containers(tool_id)
        .iter()
        .find_map(|container| read_bundled_tool_package(container, tool_id, &host_tag))
}

/// The directories that may hold the bundled package for `tool_id`.
///
/// A packaged product holds the package directly. The development tree keeps
/// one directory per packaged target, so a container may also hold the
/// package one level down.
fn bundled_tool_containers(tool_id: &str) -> Vec<PathBuf> {
    let mut containers = Vec::new();
    if let Some(directory) = std::env::var_os(BUNDLED_TOOL_DIR_ENV) {
        containers.push(PathBuf::from(directory));
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            containers.extend(
                [
                    directory.join(tool_id),
                    directory.join("resources").join(tool_id),
                    // macOS application bundle.
                    directory.join("../Resources").join(tool_id),
                    directory.join("../Resources/resources").join(tool_id),
                    // Linux package layouts.
                    directory.join("../lib/openbitfun/resources").join(tool_id),
                    directory.join("../share/openbitfun/resources").join(tool_id),
                ]
                .into_iter()
                .map(PathBuf::from),
            );
        }
    }
    // The development tree keeps one directory per packaged target. It is
    // found at runtime rather than baked in at compile time, so a release
    // binary does not carry the build machine's directory.
    containers.extend(development_tree_candidates(tool_id));
    containers
}

/// The development tree, found by walking up from the running executable.
///
/// A build lands in `target/<profile>/`, and its test binaries one level
/// deeper, so the repository root is a few levels up from the executable.
fn development_tree_candidates(tool_id: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let Ok(executable) = std::env::current_exe() else {
        return candidates;
    };
    let Some(mut directory) = executable.parent().map(Path::to_path_buf) else {
        return candidates;
    };
    for _ in 0..4 {
        candidates.push(directory.join("resources").join(tool_id));
        let Some(parent) = directory.parent() else {
            break;
        };
        if parent == directory {
            break;
        }
        directory = parent.to_path_buf();
    }
    candidates
}

/// The manifest names the executable to launch. A path that climbs out of the
/// package would let a tampered manifest point at anything on the machine, so
/// the entrypoint has to stay inside the package it belongs to.
fn entrypoint_stays_inside_package(entrypoint: &str) -> bool {
    !Path::new(entrypoint).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

fn read_bundled_tool_package(
    container: &Path,
    tool_id: &str,
    host_tag: &str,
) -> Option<BundledToolPackage> {
    if let Some(package) = read_bundled_tool_manifest(container, tool_id, host_tag) {
        return Some(package);
    }
    let entries = fs::read_dir(container).ok()?;
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .find_map(|path| read_bundled_tool_manifest(&path, tool_id, host_tag))
}

fn read_bundled_tool_manifest(
    root: &Path,
    tool_id: &str,
    host_tag: &str,
) -> Option<BundledToolPackage> {
    let manifest_path = root.join(BUNDLED_TOOL_MANIFEST);
    let raw = fs::read_to_string(&manifest_path).ok()?;
    let manifest: BundledToolManifest = serde_json::from_str(&raw).ok()?;
    if manifest.tool_id != tool_id {
        return None;
    }
    // A package built for another host is not a usable fallback. Without this
    // check a development tree holding several targets would launch whichever
    // directory happened to be read first.
    if manifest.host_tag != host_tag {
        return None;
    }
    let entrypoint = root.join(&manifest.entrypoint);
    if !entrypoint_stays_inside_package(&manifest.entrypoint) {
        return None;
    }
    if !entrypoint.is_file() {
        return None;
    }
    Some(BundledToolPackage {
        tool_id: manifest.tool_id,
        version: manifest.version,
        root: root.to_path_buf(),
        entrypoint,
        host_tag: manifest.host_tag,
        source_sha256: manifest.source_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(root: &Path, body: &str) {
        fs::create_dir_all(root).expect("package directory");
        fs::write(root.join(BUNDLED_TOOL_MANIFEST), body).expect("manifest");
    }

    fn write_file(root: &Path, name: &str) {
        fs::create_dir_all(root).expect("directory");
        fs::write(root.join(name), b"content").expect("file");
    }

    fn temp_dir(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "openbitfun-bundled-tools-{name}-{nanos}-{}",
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("temp dir");
        path
    }

    #[test]
    fn host_tag_names_the_running_host() {
        let tag = bundled_tool_host_tag();
        assert_eq!(
            tag,
            format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
        );
    }

    #[test]
    fn reads_a_package_that_carries_its_manifest() {
        let root = temp_dir("package");
        let entrypoint = root.join("bin/interpreter");
        fs::create_dir_all(entrypoint.parent().expect("bin dir")).expect("bin dir");
        fs::write(&entrypoint, b"#!/bin/sh\n").expect("entrypoint");
        write_manifest(
            &root,
            &format!(
                r#"{{"toolId":"interpreter","version":"0.0.56","hostTag":"{}","entrypoint":"bin/interpreter","sourceSha256":"abc"}}"#,
                bundled_tool_host_tag()
            ),
        );

        let package = read_bundled_tool_manifest(&root, "interpreter", &bundled_tool_host_tag())
            .expect("package");

        assert_eq!(package.tool_id, "interpreter");
        assert_eq!(package.version, "0.0.56");
        assert_eq!(package.root, root);
        assert_eq!(package.entrypoint, entrypoint);
        assert_eq!(package.source_sha256, "abc");
    }

    #[test]
    fn refuses_a_package_built_for_another_host() {
        let root = temp_dir("other-host");
        let entrypoint = root.join("bin/interpreter");
        fs::create_dir_all(entrypoint.parent().expect("bin dir")).expect("bin dir");
        fs::write(&entrypoint, b"#!/bin/sh\n").expect("entrypoint");
        write_manifest(
            &root,
            r#"{"toolId":"interpreter","version":"0.0.56","hostTag":"not-this-host","entrypoint":"bin/interpreter"}"#,
        );

        assert!(read_bundled_tool_manifest(&root, "interpreter", &bundled_tool_host_tag()).is_none());
    }

    #[test]
    fn refuses_a_package_for_another_tool() {
        let root = temp_dir("other-tool");
        write_manifest(
            &root,
            &format!(
                r#"{{"toolId":"something-else","version":"1.0.0","hostTag":"{}","entrypoint":"bin/interpreter"}}"#,
                bundled_tool_host_tag()
            ),
        );

        assert!(read_bundled_tool_manifest(&root, "interpreter", &bundled_tool_host_tag()).is_none());
    }

    #[test]
    fn refuses_a_package_whose_entrypoint_is_missing() {
        let root = temp_dir("missing-entrypoint");
        write_manifest(
            &root,
            &format!(
                r#"{{"toolId":"interpreter","version":"0.0.56","hostTag":"{}","entrypoint":"bin/interpreter"}}"#,
                bundled_tool_host_tag()
            ),
        );

        assert!(read_bundled_tool_manifest(&root, "interpreter", &bundled_tool_host_tag()).is_none());
    }

    #[test]
    fn refuses_an_entrypoint_that_climbs_out_of_the_package() {
        let root = temp_dir("escaping-entrypoint");
        // The manifest is data. An entrypoint that climbs out of the package
        // would point at any file on the machine.
        let outside = temp_dir("escaping-target");
        write_file(&outside, "not the agent");
        write_manifest(
            &root,
            &format!(
                r#"{{"toolId":"interpreter","version":"0.0.56","hostTag":"{}","entrypoint":"../{}/not the agent"}}"#,
                bundled_tool_host_tag(),
                outside.file_name().expect("target name").to_string_lossy()
            ),
        );

        assert!(read_bundled_tool_manifest(&root, "interpreter", &bundled_tool_host_tag()).is_none());
    }

    #[test]
    fn an_entrypoint_inside_the_package_is_accepted() {
        assert!(entrypoint_stays_inside_package("bin/interpreter"));
        assert!(entrypoint_stays_inside_package("codex-resources/zsh/bin/zsh"));
        assert!(!entrypoint_stays_inside_package("../outside"));
        assert!(!entrypoint_stays_inside_package("bin/../../outside"));
        assert!(!entrypoint_stays_inside_package("/etc/passwd"));
    }

    #[test]
    fn the_development_tree_is_found_without_a_compiled_in_build_path() {
        // A release binary must not carry the build machine's directory, so
        // the development tree is discovered from the running executable.
        let candidates = development_tree_candidates("interpreter");

        assert!(!candidates.is_empty());
        for candidate in &candidates {
            assert!(
                candidate.ends_with("resources/interpreter"),
                "unexpected candidate: {candidate:?}"
            );
        }
    }

    #[test]
    fn finds_a_package_one_level_below_the_container() {
        let container = temp_dir("container");
        let root = container.join("aarch64-apple-darwin");
        let entrypoint = root.join("bin/interpreter");
        fs::create_dir_all(entrypoint.parent().expect("bin dir")).expect("bin dir");
        fs::write(&entrypoint, b"#!/bin/sh\n").expect("entrypoint");
        write_manifest(
            &root,
            &format!(
                r#"{{"toolId":"interpreter","version":"0.0.56","hostTag":"{}","entrypoint":"bin/interpreter"}}"#,
                bundled_tool_host_tag()
            ),
        );

        let package = read_bundled_tool_package(&container, "interpreter", &bundled_tool_host_tag())
            .expect("package");

        assert_eq!(package.root, root);
        assert_eq!(package.entrypoint, entrypoint);
    }

    #[test]
    fn a_package_prepared_for_this_host_is_discoverable() {
        // The preparation step writes one directory per packaged target under
        // `resources/<tool>`. When one of them was prepared for this host, it
        // is found; when none was, the lookup finds nothing and the product
        // falls back to the user's own installation.
        let package = bundled_tool_package("interpreter");
        let Some(package) = package else {
            assert!(
                !bundled_tool_containers("interpreter")
                    .iter()
                    .any(|container| container.join(BUNDLED_TOOL_MANIFEST).exists()),
                "a package for this host should have been found",
            );
            return;
        };
        assert_eq!(package.tool_id, "interpreter");
        assert_eq!(package.host_tag, bundled_tool_host_tag());
        assert!(package.entrypoint.is_file());
        assert!(!package.version.is_empty());
    }
}
