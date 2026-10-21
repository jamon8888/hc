#!/usr/bin/env node
/** Runs `tauri build` from src/apps/desktop with CI=true. */
import { spawnSync } from 'child_process';
import { fileURLToPath } from 'url';
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from 'path';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'fs';
import { ensureFlashgrepBinary } from './prepare-flashgrep-resource.mjs';
import { ensureBasemindBinary } from './prepare-basemind-resource.mjs';
import { ensureInterpreterPackage } from './prepare-interpreter-resource.mjs';
import { extractProductConfigArg } from './product-customization/cli.mjs';
import { productBuildEnvironment } from './product-customization/projections.mjs';
import { resolveProductDefinition } from './product-customization/resolver.mjs';
import { resolveReleaseChannel } from './release-channel.mjs';
import {
  WEB_FONT_PROFILE_ENV,
  fontProfileForDesktopTarget,
} from './web-font-profile.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');

function tauriBuildArgsFromArgv() {
  const args = process.argv.slice(2);
  // `node script.mjs -- --foo` leaves a leading `--`; strip so `tauri build` sees the same argv as before.
  let i = 0;
  while (i < args.length && args[i] === '--') {
    i += 1;
  }
  return args.slice(i);
}

async function main() {
  const { productConfig, forwardArgs: forward } = extractProductConfigArg(tauriBuildArgsFromArgv());
  const bundleOnly = forward.includes('--bundle-only');
  if (bundleOnly) forward.splice(forward.indexOf('--bundle-only'), 1);
  const resolution = resolveProductDefinition({ rootDir: ROOT, productConfig, member: 'desktop' });
  Object.assign(process.env, productBuildEnvironment(resolution));
  console.log(`[product] ${resolution.assembly.member} ${resolution.assembly.assemblyDigest}`);
  const fontProfile = configureDesktopWebFontProfile(forward);
  console.log(`[font-profile] ${fontProfile}`);
  const releaseChannel = resolveReleaseChannel(process.env.OPENBITFUN_RELEASE_CHANNEL);
  console.log(`[release] channel=${releaseChannel.channel}`);

  const desktopDir = join(ROOT, 'src', 'apps', 'desktop');
  if (!bundleOnly) preparePluginHost();
  const flashgrepBinary = prepareMacOSFlashgrepForSigning(
    ensureFlashgrepBinary({ target: optionValue(forward, '--target') || rustHostTargetTriple() }),
    desktopDir,
  );
  process.env.FLASHGREP_DAEMON_BIN = flashgrepBinary;
  // The Basemind sidecar is packaged with the product: the PII redaction
  // surface is part of the desktop, and a user must not have to install the
  // sidecar before a document can be redacted. The binary is downloaded from
  // the pinned fork release and verified against its published checksum.
  const basemindBinary = ensureBasemindBinary({
    target: optionValue(forward, '--target') || rustHostTargetTriple(),
  });
  // The Open Interpreter agent is packaged with the product, so a user does
  // not have to install it before an ACP session can use it. The package is
  // a few hundred megabytes, so a debug build does not pull it: a developer
  // build is not a release artifact, and the product falls back to the user's
  // own installation without it.
  const interpreterPackage = prepareMacOSInterpreterForSigning(
    shouldPackageInterpreter(forward)
      ? ensureInterpreterPackage({ target: optionValue(forward, '--target') || rustHostTargetTriple() })
      : undefined,
    desktopDir,
  );
  // Tauri CLI reads CI and rejects numeric "1" (common in CI providers).
  process.env.CI = 'true';
  if (process.platform === 'darwin' && requestsDmgBundle(forward)) {
    // Tauri otherwise passes --skip-jenkins under CI, which drops the branded
    // Finder background and icon positions from the generated DMG.
    process.env.TAURI_BUNDLER_DMG_IGNORE_CI = 'true';
  }

  const tauriConfig = prepareTauriConfig(join(desktopDir, 'tauri.conf.json'), {
    desktopDir,
    flashgrepBinary,
    basemindBinary,
    interpreterPackage,
    resolution,
    releaseChannel,
  });
  const tauriBin = join(ROOT, 'node_modules', '.bin', 'tauri');
  const tauriArgs = [bundleOnly ? 'bundle' : 'build', '--config', tauriConfig, ...forward];
  let attemptStartedAtMs = Date.now();
  let r = runTauriBuild(tauriBin, tauriArgs, desktopDir);

  const maxMacDmgBuildAttempts = 3;
  for (
    let attempt = 1;
    attempt < maxMacDmgBuildAttempts
      && !r.error
      && shouldRetryMacDmgBuild(r, forward, desktopDir, attemptStartedAtMs);
    attempt += 1
  ) {
    const retryDelaySeconds = attempt * 10;
    console.warn(
      `[tauri-build] DMG bundling failed after the macOS app bundle was refreshed; retrying build attempt ${attempt + 1}/${maxMacDmgBuildAttempts} in ${retryDelaySeconds} seconds.`
    );
    await new Promise((resolveRetry) => setTimeout(resolveRetry, retryDelaySeconds * 1_000));
    attemptStartedAtMs = Date.now();
    r = runTauriBuild(tauriBin, tauriArgs, desktopDir);
  }

  if (r.error) {
    console.error(r.error);
    process.exit(1);
  }

  // Keep only the latest useful Cargo caches for this build profile after tauri build ends.
  try {
    const { profileFromTauriBuildArgs, runGcBestEffort, targetFromTauriBuildArgs } = await import(
      './cargo-target-gc.mjs'
    );
    runGcBestEffort({
      rootDir: ROOT,
      profile: profileFromTauriBuildArgs(forward),
      triple: targetFromTauriBuildArgs(forward),
    });
  } catch (error) {
    console.warn(`[target-gc] skipped: ${error.message || String(error)}`);
  }

  if (r.status === 0 && forward.includes('--no-bundle')) {
    console.warn(
      '[tauri-build] No bundle was produced. The raw desktop executable depends on its adjacent frontend, flashgrep, mobile-web, and resources directories and must not be distributed by itself.'
    );
  }

  process.exit(r.status ?? 1);
}

function rustHostTargetTriple() {
  const result = spawnSync('rustc', ['-vV'], {
    cwd: ROOT,
    encoding: 'utf8',
    shell: false,
    windowsHide: true,
  });
  if (result.error || result.status !== 0) {
    const detail = result.error?.message || result.stderr || `exit status ${result.status}`;
    throw new Error(`Could not determine the Rust host target: ${detail}`);
  }
  const host = String(result.stdout).match(/^host:\s*(\S+)$/m)?.[1];
  if (!host) throw new Error('rustc -vV did not report a host target triple.');
  return host;
}

function preparePluginHost() {
  const result = spawnSync('pnpm', ['run', 'plugin-host:prepare'], {
    cwd: ROOT,
    env: process.env,
    stdio: 'inherit',
    shell: true,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`OpenCode extension Host preparation failed with exit code ${result.status}`);
  }
}

function runTauriBuild(tauriBin, args, desktopDir) {
  return spawnSync(tauriBin, args, {
    cwd: desktopDir,
    env: process.env,
    stdio: 'inherit',
    shell: true,
  });
}

export function shouldRetryMacDmgBuild(
  result,
  args,
  desktopDir,
  buildStartedAtMs,
  runtime = {}
) {
  const platform = runtime.platform ?? process.platform;
  const githubActions = runtime.githubActions ?? process.env.GITHUB_ACTIONS;
  if (
    result.status === 0 ||
    platform !== 'darwin' ||
    githubActions !== 'true' ||
    args.includes('--no-bundle') ||
    !requestsDmgBundle(args)
  ) {
    return false;
  }

  const configuredTargetDir = runtime.cargoTargetDir ?? process.env.CARGO_TARGET_DIR;
  const targetDir = configuredTargetDir
    ? isAbsolute(configuredTargetDir)
      ? configuredTargetDir
      : resolve(desktopDir, configuredTargetDir)
    : join(runtime.root ?? ROOT, 'target');
  const target = optionValue(args, '--target');
  const profile = args.includes('--debug') ? 'debug' : optionValue(args, '--profile') || 'release';
  const bundleDir = join(
    targetDir,
    ...(target ? [target] : []),
    profile,
    'bundle',
    'macos'
  );

  const freshAfterMs = buildStartedAtMs - 1_000;
  try {
    return readdirSync(bundleDir, { withFileTypes: true }).some((entry) => {
      if (!entry.isDirectory() || !entry.name.endsWith('.app')) {
        return false;
      }

      const appDir = join(bundleDir, entry.name);
      if (statSync(appDir).mtimeMs >= freshAfterMs) {
        return true;
      }

      // The Rust cache can restore an existing app directory without changing
      // its own mtime. Tauri still refreshes the executable inside it before
      // codesigning, so use that file as the reliable bundling boundary.
      const executableDir = join(appDir, 'Contents', 'MacOS');
      return readdirSync(executableDir, { withFileTypes: true }).some(
        (executable) =>
          executable.isFile()
          && statSync(join(executableDir, executable.name)).mtimeMs >= freshAfterMs
      );
    });
  } catch {
    return false;
  }
}

function requestsDmgBundle(args) {
  const bundles = optionValue(args, '--bundles');
  return bundles === undefined || bundles.split(',').some((bundle) => bundle.trim() === 'dmg');
}

function optionValue(args, option) {
  const inlinePrefix = `${option}=`;
  for (let i = 0; i < args.length; i += 1) {
    if (args[i] === option) {
      return args[i + 1];
    }
    if (args[i].startsWith(inlinePrefix)) {
      return args[i].slice(inlinePrefix.length);
    }
  }
  return undefined;
}

export function configureDesktopWebFontProfile(
  args,
  { env = process.env, platform = process.platform } = {},
) {
  const profile = fontProfileForDesktopTarget({
    target: optionValue(args, '--target'),
    platform,
  });
  env[WEB_FONT_PROFILE_ENV] = profile;
  return profile;
}

export function shouldPackageInterpreter(args = [], env = process.env) {
  // A debug build is a developer build, not a release artifact, so it does not
  // pay for the packaged agent. `OPENBITFUN_SKIP_INTERPRETER_PACKAGE=1`
  // opts a release build out as well.
  if (['1', 'true', 'yes'].includes(String(env.OPENBITFUN_SKIP_INTERPRETER_PACKAGE ?? '').toLowerCase())) {
    return false;
  }
  return !args.includes('--debug');
}

export function prepareMacOSFlashgrepForSigning(
  flashgrepBinary,
  desktopDir,
  runtime = {},
) {
  const platform = runtime.platform ?? process.platform;
  const signingIdentity = runtime.signingIdentity ?? process.env.APPLE_SIGNING_IDENTITY;
  if (platform !== 'darwin' || !signingIdentity) {
    return flashgrepBinary;
  }

  const signedDir = join(desktopDir, 'gen', 'signed-resources', 'flashgrep');
  const signedBinary = join(signedDir, basename(flashgrepBinary));
  mkdirSync(signedDir, { recursive: true });
  copyFileSync(flashgrepBinary, signedBinary);
  chmodSync(signedBinary, statSync(signedBinary).mode | 0o111);

  const run = runtime.spawnSync ?? spawnSync;
  const result = run(
    'codesign',
    [
      '--force',
      '--sign',
      signingIdentity,
      '--options',
      'runtime',
      '--timestamp',
      signedBinary,
    ],
    { encoding: 'utf8', shell: false, windowsHide: true },
  );
  if (result.error || result.status !== 0) {
    const detail = result.error?.message || result.stderr || `exit status ${result.status}`;
    throw new Error(`Failed to sign bundled flashgrep binary: ${detail}`);
  }

  console.log(`[tauri-build] Signed bundled flashgrep binary: ${signedBinary}`);
  return signedBinary;
}

export function prepareMacOSInterpreterForSigning(
  interpreterPackage,
  desktopDir,
  runtime = {},
) {
  const platform = runtime.platform ?? process.platform;
  const signingIdentity = runtime.signingIdentity ?? process.env.APPLE_SIGNING_IDENTITY;
  if (platform !== 'darwin' || !signingIdentity || !interpreterPackage) {
    return interpreterPackage;
  }

  // The package is a directory of executables that have to stay together, so
  // the whole directory is copied and every executable in it is signed.
  const signedDir = join(desktopDir, 'gen', 'signed-resources', 'interpreter');
  rmSync(signedDir, { recursive: true, force: true });
  mkdirSync(signedDir, { recursive: true });
  copyDirectory(interpreterPackage, signedDir);

  const run = runtime.spawnSync ?? spawnSync;
  for (const relativePath of interpreterExecutables(signedDir)) {
    const result = run(
      'codesign',
      [
        '--force',
        '--sign',
        signingIdentity,
        '--options',
        'runtime',
        '--timestamp',
        join(signedDir, relativePath),
      ],
      { encoding: 'utf8', shell: false, windowsHide: true },
    );
    if (result.error || result.status !== 0) {
      const detail = result.error?.message || result.stderr || `exit status ${result.status}`;
      throw new Error(`Failed to sign bundled Open Interpreter binary: ${detail}`);
    }
  }

  console.log(`[tauri-build] Signed bundled Open Interpreter package: ${signedDir}`);
  return signedDir;
}

// The cloud private key remains in SimplySign; only its certificate selector is
// passed to Tauri. Authenticode runs before Tauri creates updater signatures.
export function configureWindowsSigning(config, env = process.env, platform = process.platform) {
  if (platform !== 'win32' || !env.WINDOWS_CERTIFICATE_THUMBPRINT) return;
  const thumbprint = env.WINDOWS_CERTIFICATE_THUMBPRINT.replace(/\s/g, '').toUpperCase();
  if (!/^[0-9A-F]{40}$/.test(thumbprint)) {
    throw new Error('WINDOWS_CERTIFICATE_THUMBPRINT must be a SHA-1 certificate fingerprint.');
  }
  config.bundle ??= {};
  config.bundle.windows = {
    ...config.bundle.windows,
    certificateThumbprint: thumbprint,
    digestAlgorithm: 'sha256',
    timestampUrl: 'http://time.certum.pl',
    tsp: true,
  };
}

export function prepareTauriConfig(
  baseConfigPath,
  { desktopDir, flashgrepBinary, basemindBinary, interpreterPackage, resolution, releaseChannel }
) {
  const config = JSON.parse(readFileSync(baseConfigPath, 'utf8'));
  if (resolution) {
    const productName =
      resolution.productNames[resolution.assembly.fallbackLocale]
      ?? resolution.productNames[resolution.assembly.defaultLocale];
    config.productName = productName;
    config.mainBinaryName = resolution.assembly.binaryName;
    config.identifier = resolution.assembly.bundleId;
  }
  configureWindowsSigning(config);
  injectTargetFlashgrepResource(config, desktopDir, flashgrepBinary);
  injectTargetBasemindResource(config, desktopDir, basemindBinary);
  injectTargetInterpreterResource(config, desktopDir, interpreterPackage);
  // The DeepSeek bridge is not a compile-time resource: cargo check and
  // desktop:dev must not require packages/dsh-acp/dist-profile. Official
  // packaging injects it here; frontend:build-all (beforeBuildCommand)
  // compiles the profile before Tauri copies resources.
  injectDshProfileResource(config);
  injectExternalFrontendResource(config);

  const release = releaseChannel
    ?? resolveReleaseChannel(process.env.OPENBITFUN_RELEASE_CHANNEL);
  const primaryEndpoint =
    process.env.TAURI_UPDATER_ENDPOINT || release.primaryUpdaterEndpoint;
  const fallbackEndpoint =
    process.env.TAURI_UPDATER_FALLBACK_ENDPOINT || release.fallbackUpdaterEndpoint;
  process.env.OPENBITFUN_RELEASE_CHANNEL = release.channel;
  process.env.OPENBITFUN_UPDATER_PRIMARY_ENDPOINT = primaryEndpoint;
  process.env.OPENBITFUN_UPDATER_FALLBACK_ENDPOINT = fallbackEndpoint;

  const enabled = ['1', 'true', 'yes'].includes(
    String(process.env.OPENBITFUN_ENABLE_UPDATER_ARTIFACTS || '').toLowerCase()
  );

  if (enabled) {
    const pubkey = process.env.TAURI_UPDATER_PUBKEY;
    if (!pubkey) {
      console.error('OPENBITFUN_ENABLE_UPDATER_ARTIFACTS is set, but TAURI_UPDATER_PUBKEY is missing.');
      process.exit(1);
    }
    if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
      console.error('OPENBITFUN_ENABLE_UPDATER_ARTIFACTS is set, but TAURI_SIGNING_PRIVATE_KEY is missing.');
      process.exit(1);
    }

    // Fallback endpoint used when GitHub is unreachable (not when no update is found).
    // Tauri updater iterates endpoints and only falls through on network/HTTP errors;
    // a 204 (no update) or a successfully parsed manifest stops the loop.
    config.bundle = {
      ...(config.bundle || {}),
      createUpdaterArtifacts: true,
    };
    config.plugins = {
      ...(config.plugins || {}),
      updater: {
        endpoints: [primaryEndpoint, fallbackEndpoint],
        pubkey,
        windows: {
          installMode: 'quiet',
        },
      },
    };
    console.log(
      `[tauri-build] Updater artifacts enabled for ${release.channel}: ${primaryEndpoint} (fallback: ${fallbackEndpoint})`
    );
  }

  const generatedDir = join(desktopDir, 'gen');
  mkdirSync(generatedDir, { recursive: true });
  const generatedConfig = join(
    generatedDir,
    resolution
      ? `tauri.${resolution.assembly.assemblyDigest}.generated.conf.json`
      : 'tauri.generated.conf.json',
  );
  writeFileSync(generatedConfig, `${JSON.stringify(config, null, 2)}\n`, 'utf8');
  return generatedConfig;
}

const DSH_PROFILE_RESOURCE_SOURCE = '../../../packages/dsh-acp/dist-profile';
const DSH_PROFILE_RESOURCE_TARGET = 'resources/dsh-profile';
const EXTERNAL_FRONTEND_RESOURCE_SOURCE = '../../../dist';
const EXTERNAL_FRONTEND_RESOURCE_TARGET = 'frontend/dist';

function injectDshProfileResource(config) {
  const resources = { ...(config.bundle?.resources || {}) };
  resources[DSH_PROFILE_RESOURCE_SOURCE] = DSH_PROFILE_RESOURCE_TARGET;
  config.bundle = {
    ...(config.bundle || {}),
    resources,
  };
}

function injectExternalFrontendResource(config) {
  const resources = { ...(config.bundle?.resources || {}) };
  resources[EXTERNAL_FRONTEND_RESOURCE_SOURCE] = EXTERNAL_FRONTEND_RESOURCE_TARGET;
  config.bundle = {
    ...(config.bundle || {}),
    resources,
  };
}

function injectTargetFlashgrepResource(config, desktopDir, flashgrepBinary) {
  const resources = { ...(config.bundle?.resources || {}) };
  delete resources['../../../resources/flashgrep'];

  for (const binaryPath of bundledFlashgrepResources(flashgrepBinary)) {
    const source = toTauriPath(relative(desktopDir, binaryPath));
    resources[source] = `flashgrep/${basename(binaryPath)}`;
  }
  config.bundle = {
    ...(config.bundle || {}),
    resources,
  };
}

function injectTargetBasemindResource(config, desktopDir, basemindBinary) {
  if (!basemindBinary) {
    return;
  }
  const resources = { ...(config.bundle?.resources || {}) };
  delete resources['../../../resources/basemind'];

  // The sidecar loads its shared libraries relative to its own directory, so
  // the whole extracted tree travels together, not just the binary.
  const source = toTauriPath(relative(desktopDir, dirname(basemindBinary)));
  resources[source] = 'basemind';
  config.bundle = {
    ...(config.bundle || {}),
    resources,
  };
}

function injectTargetInterpreterResource(config, desktopDir, interpreterPackage) {
  if (!interpreterPackage) {
    return;
  }
  const resources = { ...(config.bundle?.resources || {}) };
  delete resources['../../../resources/interpreter'];

  // The package is a directory: the agent reads its helper files relative to
  // the executable, so the whole directory has to travel together.
  const source = toTauriPath(relative(desktopDir, interpreterPackage));
  resources[source] = 'interpreter';
  config.bundle = {
    ...(config.bundle || {}),
    resources,
  };
}

function copyDirectory(source, destination) {
  mkdirSync(destination, { recursive: true });
  for (const entry of readdirSync(source, { withFileTypes: true })) {
    const from = join(source, entry.name);
    const to = join(destination, entry.name);
    if (entry.isDirectory()) {
      copyDirectory(from, to);
    } else if (entry.isFile()) {
      copyFileSync(from, to);
      // The mode travels with the file: the package's data files are not
      // executable, and the signing step covers only the files that are.
      chmodSync(to, statSync(from).mode & 0o777);
    }
  }
}

function interpreterExecutables(root) {
  // The preparation step records the executables it made runnable, so the
  // signing step covers exactly the files that are code. The package's data
  // files must not be signed.
  const manifestPath = join(root, 'openbitfun-bundled-tool.json');
  if (!existsSync(manifestPath)) {
    throw new Error('Bundled Open Interpreter package is missing its manifest');
  }
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  if (!Array.isArray(manifest.executables) || manifest.executables.length === 0) {
    throw new Error('Bundled Open Interpreter manifest lists no executables');
  }
  for (const relativePath of manifest.executables) {
    // The manifest is data. A path that climbs out of the package would make
    // the signing step touch a file outside it.
    const segments = relativePath.split('/');
    if (segments.includes('..') || isAbsolute(relativePath)) {
      throw new Error(
        `Bundled Open Interpreter manifest lists a path outside the package: ${relativePath}`,
      );
    }
  }
  return manifest.executables;
}

function bundledFlashgrepResources(primaryBinary) {
  return primaryBinary ? [primaryBinary] : [];
}

function toTauriPath(value) {
  return value.split(sep).join('/');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((e) => {
    console.error(e);
    process.exit(1);
  });
}
