const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");

const {
  REQUIRED_PAYLOAD_FILES,
  getExpectedAppProfile,
  getPayloadSourceExeName,
  resolveAppExePath,
  shouldCopySiblingRuntimeFile,
  validateRequiredPayloadFiles,
} = require("./build-installer.cjs");

const installerRoot = path.resolve(__dirname, "..");

test("installer modes select only their exact Cargo profile", () => {
  assert.equal(getExpectedAppProfile("release"), "release");
  assert.equal(getExpectedAppProfile("fast"), "release-fast");

  const releasePath = resolveAppExePath("release", [], {});
  assert.match(releasePath, /target[\\/]release[\\/]alex-desktop\.exe$/);
  assert.doesNotMatch(releasePath, /release-fast|debug/);
});

test("an explicit desktop executable wins over target directory discovery", () => {
  const explicit = path.join(os.tmpdir(), "custom-target", "alex-desktop.exe");
  assert.equal(
    resolveAppExePath("release", ["--app-exe", explicit], {
      CARGO_TARGET_DIR: "ignored-target",
    }),
    path.normalize(explicit)
  );
});

test("payload manifests record only the canonical executable name", () => {
  const appExe = path.join(
    os.tmpdir(),
    "alex-build-root",
    "target",
    "release",
    "alex-desktop.exe"
  );
  assert.equal(getPayloadSourceExeName(appExe), "alex-desktop.exe");
  assert.equal(path.isAbsolute(getPayloadSourceExeName(appExe)), false);
});

test("payload validation requires every desktop runtime surface", () => {
  const payload = fs.mkdtempSync(path.join(os.tmpdir(), "alex-installer-payload-"));
  const manifest = { files: [] };

  for (const relativePath of REQUIRED_PAYLOAD_FILES) {
    const diskPath = path.join(payload, ...relativePath.split("/"));
    fs.mkdirSync(path.dirname(diskPath), { recursive: true });
    if (relativePath === "interpreter/alex-bundled-tool.json") {
      // The packaged agent is a directory, and the manifest names the
      // executable inside it.
      fs.writeFileSync(
        diskPath,
        JSON.stringify({
          toolId: "interpreter",
          version: "0.0.56",
          hostTag: "windows-x86_64",
          target: "x86_64-pc-windows-msvc",
          entrypoint: "bin/interpreter.exe",
          sourceSha256: "a".repeat(64),
        })
      );
      fs.mkdirSync(path.join(payload, "interpreter", "bin"), { recursive: true });
      fs.writeFileSync(path.join(payload, "interpreter", "bin", "interpreter.exe"), "agent");
    } else {
      fs.writeFileSync(diskPath, relativePath);
    }
    manifest.files.push({ path: relativePath });
  }

  assert.doesNotThrow(() => validateRequiredPayloadFiles(payload, manifest));

  fs.rmSync(path.join(payload, "frontend"), { recursive: true, force: true });
  assert.throws(
    () => validateRequiredPayloadFiles(payload, manifest),
    /frontend\/dist\/index\.html/
  );
});

test("payload validation rejects a bundled agent package without its entrypoint", () => {
  const payload = fs.mkdtempSync(path.join(os.tmpdir(), "alex-installer-payload-"));
  const manifest = { files: [] };

  for (const relativePath of REQUIRED_PAYLOAD_FILES) {
    const diskPath = path.join(payload, ...relativePath.split("/"));
    fs.mkdirSync(path.dirname(diskPath), { recursive: true });
    if (relativePath === "interpreter/alex-bundled-tool.json") {
      fs.writeFileSync(
        diskPath,
        JSON.stringify({
          toolId: "interpreter",
          version: "0.0.56",
          hostTag: "windows-x86_64",
          target: "x86_64-pc-windows-msvc",
          entrypoint: "bin/interpreter.exe",
          sourceSha256: "a".repeat(64),
        })
      );
    } else {
      fs.writeFileSync(diskPath, relativePath);
    }
    manifest.files.push({ path: relativePath });
  }

  // The manifest is present, but the executable it names is not, so the
  // product would ship an agent it cannot start.
  assert.throws(
    () => validateRequiredPayloadFiles(payload, manifest),
    /bundled Open Interpreter entrypoint/
  );
});

test("payload validation rejects a bundled agent entrypoint that escapes the package", () => {
  const payload = fs.mkdtempSync(path.join(os.tmpdir(), "alex-installer-payload-"));
  const manifest = { files: [] };

  for (const relativePath of REQUIRED_PAYLOAD_FILES) {
    const diskPath = path.join(payload, ...relativePath.split("/"));
    fs.mkdirSync(path.dirname(diskPath), { recursive: true });
    if (relativePath === "interpreter/alex-bundled-tool.json") {
      fs.writeFileSync(
        diskPath,
        JSON.stringify({
          toolId: "interpreter",
          version: "0.0.56",
          hostTag: "windows-x86_64",
          target: "x86_64-pc-windows-msvc",
          // The manifest is data. An entrypoint that climbs out of the package
          // would let a tampered payload point at any file on the machine.
          entrypoint: "../resources/worker_host.js",
          sourceSha256: "a".repeat(64),
        })
      );
      // The file the entrypoint names exists, and it is outside the package.
      fs.mkdirSync(path.join(payload, "resources"), { recursive: true });
      fs.writeFileSync(path.join(payload, "resources", "worker_host.js"), "not the agent");
    } else {
      fs.writeFileSync(diskPath, relativePath);
    }
    manifest.files.push({ path: relativePath });
  }

  assert.throws(
    () => validateRequiredPayloadFiles(payload, manifest),
    /bundled Open Interpreter entrypoint/
  );
});

test("Cargo implementation locks are not copied into the installer", () => {
  assert.equal(shouldCopySiblingRuntimeFile(".cargo-lock", "alex-desktop.exe"), false);
  assert.equal(
    shouldCopySiblingRuntimeFile(".cargo-artifact-lock", "alex-desktop.exe"),
    false
  );
  assert.equal(
    shouldCopySiblingRuntimeFile(".cargo-build-lock", "alex-desktop.exe"),
    false
  );
});

test("Rust and JavaScript Tauri package lines stay pinned and aligned", () => {
  const packageJson = JSON.parse(
    fs.readFileSync(path.join(installerRoot, "package.json"), "utf8")
  );
  const cargoToml = fs.readFileSync(
    path.join(installerRoot, "src-tauri", "Cargo.toml"),
    "utf8"
  );
  const rustTauri = cargoToml.match(/tauri = \{ version = "=([^"]+)"/)?.[1];
  const rustDialog = cargoToml.match(/tauri-plugin-dialog = "=([^"]+)"/)?.[1];
  const jsTauri = packageJson.dependencies["@tauri-apps/api"];
  const jsDialog = packageJson.dependencies["@tauri-apps/plugin-dialog"];

  assert.match(jsTauri, /^\d+\.\d+\.\d+$/);
  assert.match(jsDialog, /^\d+\.\d+\.\d+$/);
  assert.equal(minorLine(rustTauri), minorLine(jsTauri));
  assert.equal(minorLine(rustDialog), minorLine(jsDialog));
});

test("all Installer validators share the required runtime file contract", () => {
  const buildRs = fs.readFileSync(path.join(installerRoot, "src-tauri", "build.rs"), "utf8");
  const commandsRs = fs.readFileSync(
    path.join(installerRoot, "src-tauri", "src", "installer", "commands.rs"),
    "utf8"
  );
  for (const source of [buildRs, commandsRs]) {
    const declaration = source.match(/const REQUIRED_PAYLOAD_FILES: \[&str; (\d+)\] = \[([\s\S]*?)\];/);
    assert.equal(Number(declaration[1]), REQUIRED_PAYLOAD_FILES.length);
    const files = [...declaration[2].matchAll(/"([^"]+)"|MAIN_APP_EXE/g)]
      .map((match) => match[1] || "alex-desktop.exe");
    assert.deepEqual(files, REQUIRED_PAYLOAD_FILES);
  }
  for (const relativePath of REQUIRED_PAYLOAD_FILES) {
    assert.match(buildRs, new RegExp(escapeRegExp(relativePath)));
    if (relativePath === "alex-desktop.exe") {
      assert.match(commandsRs, /MAIN_APP_EXE/);
    } else {
      assert.match(commandsRs, new RegExp(escapeRegExp(relativePath)));
    }
  }
});

test("standalone Data Migrator is not required by the installer", () => {
  assert.ok(!REQUIRED_PAYLOAD_FILES.includes("alex-data-migrator.exe"));
  const commandsRs = fs.readFileSync(path.join(installerRoot, "src-tauri/src/installer/commands.rs"), "utf8");
  assert.doesNotMatch(commandsRs, /DATA_MIGRATOR_EXE|HandoffStore|launch_trusted_executable/);
  assert.match(commandsRs, /Data Migrator is distributed separately/);
  const themeSetup = fs.readFileSync(path.join(installerRoot, "src/pages/ThemeSetup.tsx"), "utf8");
  assert.doesNotMatch(themeSetup, /migrateLegacyData|onLaunchMigration/);
});

function minorLine(version) {
  assert.match(version, /^\d+\.\d+\.\d+$/);
  return version.split(".").slice(0, 2).join(".");
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
