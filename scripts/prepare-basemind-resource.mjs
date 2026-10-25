import { createHash, randomUUID } from 'node:crypto';
import { chmodSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const RESOURCE_DIR = join(ROOT, 'resources', 'basemind');

/**
 * The Rust target triples the basemind sidecar is published for.
 *
 * The fork ships one archive per triple, named after the triple, so the
 * asset name is derived from the target rather than looked up in a table.
 */
const SUPPORTED_TARGETS = new Set([
  'aarch64-apple-darwin',
  'aarch64-unknown-linux-gnu',
  'x86_64-apple-darwin',
  'x86_64-pc-windows-msvc',
  'x86_64-unknown-linux-gnu',
]);

/** The host target triple, used when no `--target` is given. */
export function basemindHostTargetTriple({
  platform = process.platform,
  arch = process.arch,
} = {}) {
  if (platform === 'darwin') {
    return arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin';
  }
  if (platform === 'linux') {
    return arch === 'arm64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu';
  }
  if (platform === 'win32') {
    if (arch !== 'x64') {
      throw new Error(`Unsupported Basemind target: ${platform}/${arch}`);
    }
    return 'x86_64-pc-windows-msvc';
  }
  throw new Error(`Unsupported Basemind target: ${platform}/${arch}`);
}

/** The release archive for a target triple. */
export function basemindArchiveName(target) {
  if (!SUPPORTED_TARGETS.has(target)) {
    throw new Error(`Unsupported Basemind target: ${target}`);
  }
  // The Windows release ships a zip; every other platform ships a tarball.
  const extension = target === 'x86_64-pc-windows-msvc' ? 'zip' : 'tar.gz';
  return `basemind-${target}.${extension}`;
}

/**
 * The sidecar binary inside the release archive.
 *
 * The Windows release ships a zip; it carries the same `.exe` name.
 */
export function basemindBinaryName(target) {
  if (!SUPPORTED_TARGETS.has(target)) {
    throw new Error(`Unsupported Basemind target: ${target}`);
  }
  return target === 'x86_64-pc-windows-msvc'
    ? 'basemind.exe'
    : 'basemind';
}

/** The bundled sidecar path for a target triple. */
export function basemindBinaryPath(options = {}) {
  const resourceDir = options.resourceDir || RESOURCE_DIR;
  return join(resourceDir, basemindBinaryName(options.target || basemindHostTargetTriple()));
}

/**
 * Download a release asset.
 *
 * curl is used because it honours the system proxy settings on every
 * desktop build host, including Windows.
 */
export function downloadBasemind(url, destination) {
  const result = spawnSync(process.platform === 'win32' ? 'curl.exe' : 'curl', [
    '--fail', '--location', '--silent', '--show-error',
    '--proto', '=https', '--proto-redir', '=https',
    '--retry', '3', '--connect-timeout', '20', '--max-time', '900',
    '--output', destination, url,
  ], { encoding: 'utf8', windowsHide: true, shell: false });
  if (result.error || result.status !== 0) {
    throw new Error(`Failed to download Basemind: ${result.error?.message || result.stderr || result.status}`);
  }
}

/** List the entries of a tar archive. */
function listArchiveEntries(archive, cwd) {
  const result = spawnSync('tar', ['-tzf', archive], { encoding: 'utf8', cwd, shell: false });
  if (result.error || result.status !== 0) {
    throw new Error(`Failed to read the Basemind archive: ${result.error?.message || result.stderr || result.status}`);
  }
  return result.stdout.split('\n').map((line) => line.trim()).filter(Boolean);
}

/** List the entries of a zip archive. */
function listZipEntries(archive, cwd) {
  const result = spawnSync('unzip', ['-Z1', archive], { encoding: 'utf8', cwd, shell: false });
  if (result.error || result.status !== 0) {
    throw new Error(`Failed to read the Basemind archive: ${result.error?.message || result.stderr || result.status}`);
  }
  return result.stdout.split('\n').map((line) => line.trim()).filter(Boolean);
}

/**
 * Extract the sidecar binary out of a release archive.
 *
 * The archive holds the binary plus its runtime assets, so the whole
 * extracted tree is kept: the binary loads its models and shared
 * libraries relative to itself.
 */
function extractBinary(archive, binaryName, destination) {
  const entries = archive.endsWith('.zip')
    ? listZipEntries(archive, ROOT)
    : listArchiveEntries(archive, ROOT);
  const match = entries.find((entry) => entry.split('/').pop() === binaryName);
  if (!match) {
    throw new Error(`The Basemind archive does not contain ${binaryName}`);
  }
  const extract = archive.endsWith('.zip')
    ? spawnSync('unzip', ['-o', archive, match, '-d', destination], { encoding: 'utf8', cwd: ROOT, shell: false })
    : spawnSync('tar', ['-xzf', archive, '-C', destination], { encoding: 'utf8', cwd: ROOT, shell: false });
  if (extract.error || extract.status !== 0) {
    throw new Error(`Failed to extract the Basemind archive: ${extract.error?.message || extract.stderr || extract.status}`);
  }
  return join(destination, match);
}

/**
 * Ensure the basemind sidecar for a target is present and verified.
 *
 * The pinned manifest carries the release asset's SHA-256 and size; a
 * download that does not match is refused, so the bundled binary is the
 * published one and not something that merely claims to be.
 */
export function ensureBasemindBinary(options = {}) {
  const resourceDir = options.resourceDir || RESOURCE_DIR;
  const manifest = options.manifest || JSON.parse(readFileSync(join(RESOURCE_DIR, 'VERSION.json'), 'utf8'));
  const target = options.target || basemindHostTargetTriple();
  const archiveName = basemindArchiveName(target);
  const binaryName = basemindBinaryName(target);
  const asset = manifest.assets?.[archiveName];
  if (!asset || !/^[a-f0-9]{64}$/.test(asset.sha256) || !Number.isSafeInteger(asset.size) || asset.size <= 0) {
    throw new Error(`Missing or invalid pinned Basemind asset: ${archiveName}`);
  }
  if (!/^[\w.-]+\/[\w.-]+$/.test(manifest.repo) || !/^v[\w.-]+$/.test(manifest.tag)) {
    throw new Error('Invalid pinned Basemind release');
  }

  const binaryPath = join(resourceDir, binaryName);
  // The extracted binary is what ships, so it carries its own pin: a
  // cached binary that does not match it is repaired by downloading again.
  const binaryValid = (path) => {
    if (!existsSync(path)) return false;
    const bytes = readFileSync(path);
    return bytes.length === asset.binary.size
      && createHash('sha256').update(bytes).digest('hex') === asset.binary.sha256;
  };

  if (!binaryValid(binaryPath)) {
    mkdirSync(resourceDir, { recursive: true });
    const archivePath = join(resourceDir, archiveName);
    const temporaryArchive = `${archivePath}.download-${randomUUID()}`;
    try {
      const url = `https://github.com/${manifest.repo}/releases/download/${manifest.tag}/${archiveName}`;
      (options.download || downloadBasemind)(url, temporaryArchive);
      const bytes = readFileSync(temporaryArchive);
      if (bytes.length !== asset.size || createHash('sha256').update(bytes).digest('hex') !== asset.sha256) {
        throw new Error(`Basemind SHA-256 or size mismatch: ${archiveName}`);
      }
      // The archive is verified; the binary inside it is what ships, so it
      // is verified too.
      const extracted = extractBinary(temporaryArchive, binaryName, resourceDir);
      const extractedBytes = readFileSync(extracted);
      if (extractedBytes.length !== asset.binary.size
        || createHash('sha256').update(extractedBytes).digest('hex') !== asset.binary.sha256) {
        rmSync(extracted, { force: true });
        throw new Error(`Basemind binary SHA-256 or size mismatch: ${binaryName}`);
      }
      renameSync(extracted, binaryPath);
    } finally {
      rmSync(temporaryArchive, { force: true });
    }
  }
  if (!binaryName.endsWith('.exe')) chmodSync(binaryPath, 0o755);
  return binaryPath;
}

/**
 * The basemind sidecar version this product bundles.
 *
 * The integration crate pins the same version, and the desktop refuses a
 * daemon that reports a different one.
 */
export function basemindPinnedVersion(options = {}) {
  const manifest = options.manifest || JSON.parse(readFileSync(join(RESOURCE_DIR, 'VERSION.json'), 'utf8'));
  return manifest.tag.replace(/^v/, '');
}

/**
 * Ensure the pinned GLiNER2 PII model is present next to the sidecar.
 *
 * The redaction pipeline refuses a call whose text NER could not run
 * (`require_ner`), so the model is part of the sidecar the product
 * bundles: the pinned Hugging Face revision fetches `tokenizer.json`,
 * `model.safetensors` and `encoder_config/config.json` into
 * `ner-model/`, sized and checksum-validated against the pinned tree.
 * A revision-pinned download over HTTPS plus the first-install size
 * check is the provenance contract; the sha256 record beside the model
 * makes every later prep verify the local cache, the same way the
 * extracted binary is re-verified.
 */
export function ensureBasemindNerModel(options = {}) {
  const resourceDir = options.resourceDir || RESOURCE_DIR;
  const manifest = options.manifest || JSON.parse(readFileSync(join(RESOURCE_DIR, 'VERSION.json'), 'utf8'));
  const pin = manifest.ner_model;
  if (!pin || !/^[\w.-]+\/[\w.-]+$/.test(pin.repo) || !/^[a-f0-9]{40}$/.test(pin.revision)) {
    throw new Error('Invalid pinned Basemind NER model');
  }
  const modelDir = join(resourceDir, 'ner-model');
  const sumsPath = join(modelDir, '.sha256sums');
  const expected = pin.files.map((file) => ({
    path: file.path,
    size: Number(file.size),
    downloadedPath: join(modelDir, file.path),
  }));
  if (!Number.isSafeInteger(expected.at(-1).size) || expected.some((file) => file.path.startsWith('/'))) {
    throw new Error('Invalid pinned Basemind NER model file list');
  }

  const shaRecorded = existsSync(sumsPath)
    ? readFileSync(sumsPath, 'utf8').trim().split('\n')
    : [];
  const recordedSum = (path) => shaRecorded.find((line) => line.endsWith(`  ${path}`))?.split(' ')[0] ?? '';
  // 'invalid' means present but not matching the pin: refuse rather than
  // hand out a replaced model. 'first' means the size matches but no
  // checksum was recorded yet (the first install completes that below).
  const fileState = (file) => {
    if (!existsSync(file.downloadedPath)) return 'missing';
    const bytes = readFileSync(file.downloadedPath);
    if (bytes.length !== file.size) return 'invalid';
    const recorded = recordedSum(file.path);
    if (!/^[a-f0-9]{64}$/.test(recorded)) return 'first';
    return createHash('sha256').update(bytes).digest('hex') === recorded ? 'valid' : 'invalid';
  };
  const states = new Map(expected.map((file) => {
    const state = fileState(file);
    if (state === 'invalid') {
      throw new Error(`Basemind NER model cache validation failed: ${file.path}`);
    }
    return [file.path, state];
  }));
  const downloadable = expected.filter((file) => states.get(file.path) !== 'valid');

  if (downloadable.length > 0) {
    mkdirSync(modelDir, { recursive: true });
    const handoff = options.download || downloadBasemind;
    for (const file of downloadable) {
      mkdirSync(dirname(file.downloadedPath), { recursive: true });
      const url = `https://huggingface.co/${pin.repo}/resolve/${pin.revision}/${encodeURIComponent(file.path)}`;
      handoff(url, file.downloadedPath);
      const bytes = readFileSync(file.downloadedPath);
      if (bytes.length !== file.size) {
        rmSync(file.downloadedPath, { force: true });
        throw new Error(`Basemind NER model size mismatch: ${file.path}`);
      }
    }
    // Record the checksums so every later prep can detect a corrupted or
    // replaced cache instead of silently handing out a differing model.
    const sums = expected.map((file) => {
      const bytes = readFileSync(file.downloadedPath);
      return `${createHash('sha256').update(bytes).digest('hex')}  ${file.path}`;
    });
    writeFileSync(sumsPath, `${sums.join('\n')}\n`);
  }
  return modelDir;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const inlineTarget = args.find((arg) => arg.startsWith('--target='));
  const targetIndex = args.indexOf('--target');
  const target = inlineTarget ? inlineTarget.slice('--target='.length) : targetIndex >= 0 ? args[targetIndex + 1] : undefined;
  if (targetIndex >= 0 && !target) throw new Error('--target requires a Rust target triple');
  console.log(ensureBasemindBinary({ target }));
  console.log(ensureBasemindNerModel({ target }));
}
