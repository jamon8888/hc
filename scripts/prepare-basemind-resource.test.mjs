import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import {
  basemindArchiveName,
  basemindBinaryName,
  basemindHostTargetTriple,
  basemindPinnedVersion,
  ensureBasemindBinary,
} from './prepare-basemind-resource.mjs';

for (const [target, archive, binary] of [
  ['x86_64-apple-darwin', 'basemind-x86_64-apple-darwin.tar.gz', 'basemind'],
  ['aarch64-apple-darwin', 'basemind-aarch64-apple-darwin.tar.gz', 'basemind'],
  ['x86_64-unknown-linux-gnu', 'basemind-x86_64-unknown-linux-gnu.tar.gz', 'basemind'],
  ['aarch64-unknown-linux-gnu', 'basemind-aarch64-unknown-linux-gnu.tar.gz', 'basemind'],
  ['x86_64-pc-windows-msvc', 'basemind-x86_64-pc-windows-msvc.zip', 'basemind.exe'],
]) {
  test(`selects ${target} independently of the build host`, () => {
    assert.equal(basemindArchiveName(target), archive);
    assert.equal(basemindBinaryName(target), binary);
  });
}

test('unsupported targets fail explicitly', () => {
  assert.throws(() => basemindArchiveName('riscv64-unknown-linux-gnu'), /Unsupported/);
  assert.throws(() => basemindBinaryName('riscv64-unknown-linux-gnu'), /Unsupported/);
});

test('the host target triple follows the build host', () => {
  assert.equal(basemindHostTargetTriple({ platform: 'darwin', arch: 'arm64' }), 'aarch64-apple-darwin');
  assert.equal(basemindHostTargetTriple({ platform: 'darwin', arch: 'x64' }), 'x86_64-apple-darwin');
  assert.equal(basemindHostTargetTriple({ platform: 'linux', arch: 'x64' }), 'x86_64-unknown-linux-gnu');
  assert.equal(basemindHostTargetTriple({ platform: 'linux', arch: 'arm64' }), 'aarch64-unknown-linux-gnu');
  assert.equal(basemindHostTargetTriple({ platform: 'win32', arch: 'x64' }), 'x86_64-pc-windows-msvc');
  assert.throws(() => basemindHostTargetTriple({ platform: 'win32', arch: 'ia32' }), /Unsupported/);
});

test('the pinned version matches the integration crate', () => {
  // The sidecar and the product must agree on the fork version: the
  // desktop refuses a daemon that reports a different one.
  assert.equal(basemindPinnedVersion(), '0.34.0');
});

test('a verified download is unpacked, and a corrupt download never replaces it', () => {
  const resourceDir = mkdtempSync(join(tmpdir(), 'basemind-download-'));
  const target = 'aarch64-apple-darwin';
  const archiveName = basemindArchiveName(target);
  const binaryName = basemindBinaryName(target);
  const manifest = {
    repo: 'jamon8888/basemind',
    tag: 'v0.34.0',
    assets: {},
  };

  // The archive carries the binary and the shared libraries it loads
  // relative to itself, so the whole tree has to travel together.
  const stage = mkdtempSync(join(tmpdir(), 'basemind-archive-'));
  const bytes = Buffer.from('verified sidecar');
  writeFileSync(join(stage, binaryName), bytes);
  writeFileSync(join(stage, 'libonnxruntime.dylib'), 'shared library');
  const archivePath = join(stage, archiveName);
  execFileSync('tar', ['-czf', archivePath, '-C', stage, binaryName, 'libonnxruntime.dylib']);
  const archiveBytes = readFileSync(archivePath);
  manifest.assets[archiveName] = {
    sha256: createHash('sha256').update(archiveBytes).digest('hex'),
    size: archiveBytes.length,
    binary: {
      sha256: createHash('sha256').update(bytes).digest('hex'),
      size: bytes.length,
    },
  };

  let downloads = 0;
  const options = {
    resourceDir,
    target,
    manifest,
    download(url, destination) {
      downloads++;
      assert.equal(url, `https://github.com/jamon8888/basemind/releases/download/v0.34.0/${archiveName}`);
      writeFileSync(destination, archiveBytes);
    },
  };

  try {
    const path = ensureBasemindBinary(options);
    assert.deepEqual(readFileSync(path), bytes);
    // The shared libraries travel with the binary.
    assert.equal(readFileSync(join(resourceDir, 'libonnxruntime.dylib'), 'utf8'), 'shared library');
    // A second call is served from the cache.
    assert.equal(ensureBasemindBinary(options), path);
    assert.equal(downloads, 1);

    // A cached binary that does not match its pin is repaired, not trusted.
    writeFileSync(path, 'corrupt');
    ensureBasemindBinary(options);
    assert.deepEqual(readFileSync(path), bytes);
    assert.equal(downloads, 2);

    // A download that does not match the pin never replaces the asset.
    writeFileSync(path, 'corrupt');
    assert.throws(() => ensureBasemindBinary({
      ...options,
      download: (_, destination) => writeFileSync(destination, 'bad release'),
    }), /mismatch/);
    assert.equal(readFileSync(path, 'utf8'), 'corrupt');

    // A manifest without the asset is refused before anything is fetched.
    assert.throws(() => ensureBasemindBinary({
      ...options,
      manifest: { ...manifest, assets: {} },
    }), /Missing/);
  } finally {
    rmSync(resourceDir, { recursive: true, force: true });
    rmSync(stage, { recursive: true, force: true });
  }
});
