import { createHash, randomUUID } from 'node:crypto';
import { chmodSync, copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const RESOURCE_DIR = join(ROOT, 'resources', 'interpreter');
const MANIFEST_PATH = join(RESOURCE_DIR, 'VERSION.json');

// The manifest the product reads at runtime. It is written next to the
// packaged files so the product never has to read the upstream package
// descriptor, and so a change to the upstream layout cannot break the lookup.
const PACKAGE_MANIFEST = 'openbitfun-bundled-tool.json';

const HOST_TARGETS = {
  'darwin/arm64': 'aarch64-apple-darwin',
  'darwin/x64': 'x86_64-apple-darwin',
  'win32/arm64': 'aarch64-pc-windows-msvc',
  'win32/x64': 'x86_64-pc-windows-msvc',
  'linux/arm64': 'aarch64-unknown-linux-musl',
  'linux/x64': 'x86_64-unknown-linux-musl',
};

// The product derives its host tag from the platform it runs on and only
// launches a package built for that host. The tag follows the TARGET, not the
// build host, so a cross-compiled package is labelled for the machine it will
// run on.
const TARGET_HOST_TAGS = {
  'x86_64-apple-darwin': 'macos-x86_64',
  'aarch64-apple-darwin': 'macos-aarch64',
  'x86_64-pc-windows-msvc': 'windows-x86_64',
  'aarch64-pc-windows-msvc': 'windows-aarch64',
  'x86_64-unknown-linux-musl': 'linux-x86_64',
  'aarch64-unknown-linux-musl': 'linux-aarch64',
};

export function interpreterTarget(options = {}) {
  const { target, platform = process.platform, arch = process.arch } = options;
  const hostKey = `${platform}/${arch}`;
  const triple = (target || HOST_TARGETS[hostKey] || '')
    .replace(/-unknown-linux-gnu$/, '-unknown-linux-musl');
  if (!Object.values(HOST_TARGETS).includes(triple)) {
    throw new Error(`Unsupported Open Interpreter target: ${target || hostKey}`);
  }
  return triple;
}

export function interpreterHostTag(options = {}) {
  const hostTag = TARGET_HOST_TAGS[interpreterTarget(options)];
  if (!hostTag) {
    throw new Error(`Unsupported Open Interpreter target: ${options.target || `${options.platform}/${options.arch}`}`);
  }
  return hostTag;
}

export function interpreterAssetName(options = {}) {
  const manifest = readPinnedManifest(options);
  return `${manifest.assetStem}-${interpreterTarget(options)}.tar.gz`;
}

export function interpreterPackageDirectory(options = {}) {
  return join(options.resourceDir || RESOURCE_DIR, interpreterTarget(options));
}

export function interpreterArchiveDirectory(options = {}) {
  return join(options.resourceDir || RESOURCE_DIR, '.cache', interpreterTarget(options));
}

export function readPinnedManifest(options = {}) {
  if (options.manifest) {
    return options.manifest;
  }
  const path = options.manifestPath || MANIFEST_PATH;
  return JSON.parse(readFileSync(path, 'utf8'));
}

export function downloadInterpreter(url, destination) {
  // curl supports system proxy settings on all desktop build hosts, including Windows.
  const result = spawnSync(process.platform === 'win32' ? 'curl.exe' : 'curl', [
    '--fail', '--location', '--silent', '--show-error',
    '--proto', '=https', '--proto-redir', '=https',
    '--retry', '3', '--connect-timeout', '20', '--max-time', '1800',
    '--output', destination, url,
  ], { encoding: 'utf8', windowsHide: true, shell: false });
  if (result.error || result.status !== 0) {
    throw new Error(
      `Failed to download Open Interpreter: ${result.error?.message || result.stderr || result.status}`,
    );
  }
}

function extractArchive(archivePath, destination) {
  // tar ships with macOS, Linux, and Windows 10+, so the build host does not
  // need a package manager for it.
  const result = spawnSync('tar', ['-xzf', archivePath, '-C', destination], {
    encoding: 'utf8',
    windowsHide: true,
    shell: false,
  });
  if (result.error || result.status !== 0) {
    throw new Error(
      `Failed to extract Open Interpreter: ${result.error?.message || result.stderr || result.status}`,
    );
  }
}

// Apache-2.0 requires the license and the upstream NOTICE to travel with the
// redistributed code, so they are copied into the package itself rather than
// shipped beside it.
// An interrupted extraction leaves its staging directory behind, and the
// package is a few hundred megabytes. Without this sweep every interrupted
// run would keep one more copy on the build machine.
function sweepStaleStagingDirectories(parent, target) {
  if (!existsSync(parent)) {
    return;
  }
  const prefix = `.staging-${target}-`;
  for (const entry of readdirSync(parent, { withFileTypes: true })) {
    if (entry.isDirectory() && entry.name.startsWith(prefix)) {
      rmSync(join(parent, entry.name), { recursive: true, force: true });
    }
  }
}

function copyLicenseFiles(root, resourceDir) {
  for (const name of ['LICENSE', 'NOTICE']) {
    const source = join(resourceDir, name);
    if (!existsSync(source)) {
      throw new Error(`Open Interpreter ${name} is missing from ${resourceDir}`);
    }
    copyFileSync(source, join(root, name));
  }
}

// The files in the package that are code, derived from the upstream package
// descriptor rather than a fixed list. The agent's own binaries live in the
// entrypoint's directory, and the upstream names the directories holding its
// helper binaries. A hardcoded list misses anything upstream adds — the macOS
// package carries a nested `codex-resources/zsh/bin/zsh` that has to be signed
// with the rest, or a notarized bundle is rejected.
function collectExecutables(root, descriptor) {
  const executables = new Set();
  const collect = (relativeDirectory) => {
    const directory = join(root, relativeDirectory);
    if (!existsSync(directory)) {
      return;
    }
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const relative = `${relativeDirectory}/${entry.name}`;
      if (entry.isDirectory()) {
        collect(relative);
      } else if (entry.isFile()) {
        executables.add(relative);
      }
    }
  };

  const entrypoint = descriptor.entrypoint;
  executables.add(entrypoint);
  const entrypointDirectory = entrypoint.includes('/')
    ? entrypoint.slice(0, entrypoint.lastIndexOf('/'))
    : '';
  if (entrypointDirectory) {
    collect(entrypointDirectory);
  }
  for (const directory of [descriptor.pathDir, descriptor.resourcesDir]) {
    if (directory) {
      collect(directory);
    }
  }
  return [...executables].sort();
}

function makeExecutable(root, executables) {
  for (const relative of executables) {
    chmodSync(join(root, relative), 0o755);
  }
}

function writePackageManifest(root, manifest) {
  writeFileSync(
    join(root, PACKAGE_MANIFEST),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
}

function readUpstreamDescriptor(root) {
  // The upstream package descriptor names the executable and the directories
  // holding the helper binaries. Reading it here keeps the product from
  // depending on the upstream layout.
  const descriptor = join(root, 'codex-package.json');
  if (!existsSync(descriptor)) {
    throw new Error('Open Interpreter package is missing codex-package.json');
  }
  const parsed = JSON.parse(readFileSync(descriptor, 'utf8'));
  if (typeof parsed.entrypoint !== 'string' || !parsed.entrypoint) {
    throw new Error('Open Interpreter package descriptor does not name an entrypoint');
  }
  return parsed;
}

function installedPackageEntryPoint(root) {
  const manifestPath = join(root, PACKAGE_MANIFEST);
  if (!existsSync(manifestPath)) {
    return null;
  }
  try {
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
    if (typeof manifest.entrypoint !== 'string' || !manifest.entrypoint) {
      return null;
    }
    return existsSync(join(root, manifest.entrypoint)) ? manifest.entrypoint : null;
  } catch {
    return null;
  }
}

export function ensureInterpreterPackage(options = {}) {
  const manifest = options.manifest || readPinnedManifest(options);
  const target = interpreterTarget(options);
  const hostTag = interpreterHostTag(options);
  const asset = manifest.assets?.[target];
  if (!asset || !/^[a-f0-9]{64}$/.test(asset.sha256) || !Number.isSafeInteger(asset.size) || asset.size <= 0) {
    throw new Error(`Missing or invalid pinned Open Interpreter asset: ${target}`);
  }
  if (!/^[\w.-]+\/[\w.-]+$/.test(manifest.repo) || !/^[\w.-]+$/.test(manifest.tag)) {
    throw new Error('Invalid pinned Open Interpreter release');
  }
  if (typeof manifest.assetStem !== 'string' || !/^[\w.-]+$/.test(manifest.assetStem)) {
    throw new Error('Invalid pinned Open Interpreter asset stem');
  }

  const packageDirectory = interpreterPackageDirectory({ ...options, target });
  if (options.force !== true && installedPackageEntryPoint(packageDirectory)) {
    return packageDirectory;
  }

  // Sweep before the download. The package is a few hundred megabytes, so
  // freeing the copies an interrupted run left behind matters on a build
  // machine, and it has to happen even if the download itself fails.
  sweepStaleStagingDirectories(dirname(packageDirectory), target);

  const assetName = `${manifest.assetStem}-${target}.tar.gz`;
  const url = `https://github.com/${manifest.repo}/releases/download/${manifest.tag}/${assetName}`;
  const archiveDirectory = interpreterArchiveDirectory({ ...options, target });
  const archivePath = join(archiveDirectory, assetName);
  mkdirSync(archiveDirectory, { recursive: true });

  const valid = (path) => {
    if (!existsSync(path)) return false;
    const bytes = readFileSync(path);
    return bytes.length === asset.size && createHash('sha256').update(bytes).digest('hex') === asset.sha256;
  };

  if (!valid(archivePath)) {
    const temporary = `${archivePath}.${randomUUID()}`;
    try {
      (options.download || downloadInterpreter)(url, temporary);
      if (!valid(temporary)) {
        throw new Error(`Open Interpreter SHA-256 or size mismatch: ${assetName}`);
      }
      renameSync(temporary, archivePath);
    } finally {
      rmSync(temporary, { force: true });
    }
  }

  // Stage beside the package, not inside it: the finished package replaces the
  // target directory, which would take the staging copy with it.
  const stagingDirectory = join(dirname(packageDirectory), `.staging-${interpreterTarget(options)}-${randomUUID()}`);
  rmSync(stagingDirectory, { recursive: true, force: true });
  mkdirSync(stagingDirectory, { recursive: true });
  try {
    extractArchive(archivePath, stagingDirectory);
    const descriptor = readUpstreamDescriptor(stagingDirectory);
    const entrypoint = descriptor.entrypoint;
    if (!existsSync(join(stagingDirectory, entrypoint))) {
      throw new Error(`Open Interpreter package does not contain its entrypoint: ${entrypoint}`);
    }
    const executables = collectExecutables(stagingDirectory, descriptor);
    makeExecutable(stagingDirectory, executables);
    copyLicenseFiles(stagingDirectory, options.resourceDir || RESOURCE_DIR);
    writePackageManifest(stagingDirectory, {
      toolId: options.toolId || 'interpreter',
      version: manifest.version,
      hostTag,
      target,
      entrypoint,
      sourceSha256: asset.sha256,
      // The executables are listed so the release build signs exactly the
      // files that are code, and not the package's data files.
      executables,
    });
    // The manifest lands last, so an interrupted extraction reads as absent
    // rather than as a usable package.
    rmSync(packageDirectory, { recursive: true, force: true });
    renameSync(stagingDirectory, packageDirectory);
  } catch (error) {
    rmSync(stagingDirectory, { recursive: true, force: true });
    throw error;
  }

  return packageDirectory;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const inlineTarget = args.find((arg) => arg.startsWith('--target='));
  const targetIndex = args.indexOf('--target');
  const target = inlineTarget
    ? inlineTarget.slice('--target='.length)
    : targetIndex >= 0 ? args[targetIndex + 1] : undefined;
  if (targetIndex >= 0 && !target) throw new Error('--target requires a Rust target triple');
  console.log(ensureInterpreterPackage({ target }));
}
