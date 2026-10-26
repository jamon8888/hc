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

test('the noavx2 flavor selects its own pinned archive, only where it exists', () => {
  assert.equal(
    basemindArchiveName('x86_64-unknown-linux-gnu', 'noavx2'),
    'basemind-x86_64-unknown-linux-gnu-noavx2.tar.gz',
  );
  assert.throws(
    () => basemindArchiveName('x86_64-pc-windows-msvc', 'noavx2'),
    /only published for x86_64-unknown-linux-gnu/,
  );
  assert.throws(() => basemindArchiveName('x86_64-unknown-linux-gnu', 'bogus'), /Unsupported Basemind flavor/);
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
  assert.equal(basemindPinnedVersion(), '0.35.0');
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

test('the noavx2 flavor pins, verifies, and installs its own archive', () => {
  const target = 'x86_64-unknown-linux-gnu';
  const archive = basemindArchiveName(target, 'noavx2');
  const resourceDir = mkdtempSync(join(tmpdir(), 'basemind-noavx2-'));
  const stage = mkdtempSync(join(tmpdir(), 'basemind-noavx2-arch-'));
  try {
    const bytes = Buffer.from('noavx2 sidecar');
    writeFileSync(join(stage, 'basemind'), bytes);
    const archivePath = join(stage, archive);
    execFileSync('tar', ['-czf', archivePath, '-C', stage, 'basemind']);
    const archiveBytes = readFileSync(archivePath);
    const manifest = {
      repo: 'jamon8888/basemind',
      tag: 'v0.35.0',
      assets: {
        [archive]: {
          sha256: createHash('sha256').update(archiveBytes).digest('hex'),
          size: archiveBytes.length,
          binary: { sha256: createHash('sha256').update(bytes).digest('hex'), size: bytes.length },
        },
      },
    };
    let requested = null;
    const path = ensureBasemindBinary({
      resourceDir,
      target,
      flavor: 'noavx2',
      manifest,
      download(url, destination) {
        requested = url;
        writeFileSync(destination, archiveBytes);
      },
    });
    assert.equal(requested, `https://github.com/jamon8888/basemind/releases/download/v0.35.0/${archive}`);
    assert.deepEqual(readFileSync(path), bytes);
  } finally {
    rmSync(resourceDir, { recursive: true, force: true });
    rmSync(stage, { recursive: true, force: true });
  }
});

test('OPENBITFUN_BASEMIND_NOAVX2=1 opts the default flavor into noavx2', () => {
  const previous = process.env.OPENBITFUN_BASEMIND_NOAVX2;
  process.env.OPENBITFUN_BASEMIND_NOAVX2 = '1';
  try {
    const target = 'x86_64-unknown-linux-gnu';
    const archive = basemindArchiveName(target, 'noavx2');
    const resourceDir = mkdtempSync(join(tmpdir(), 'basemind-noavx2-env-'));
    const stage = mkdtempSync(join(tmpdir(), 'basemind-noavx2-env-arch-'));
    try {
      const bytes = Buffer.from('noavx2 sidecar');
      writeFileSync(join(stage, 'basemind'), bytes);
      const archivePath = join(stage, archive);
      execFileSync('tar', ['-czf', archivePath, '-C', stage, 'basemind']);
      const archiveBytes = readFileSync(archivePath);
      const path = ensureBasemindBinary({
        resourceDir,
        target,
        manifest: {
          repo: 'jamon8888/basemind',
          tag: 'v0.35.0',
          assets: {
            [archive]: {
              sha256: createHash('sha256').update(archiveBytes).digest('hex'),
              size: archiveBytes.length,
              binary: { sha256: createHash('sha256').update(bytes).digest('hex'), size: bytes.length },
            },
          },
        },
        download(url, destination) {
          writeFileSync(destination, archiveBytes);
        },
      });
      assert.deepEqual(readFileSync(path), bytes);
    } finally {
      rmSync(resourceDir, { recursive: true, force: true });
      rmSync(stage, { recursive: true, force: true });
    }
  } finally {
    if (previous === undefined) delete process.env.OPENBITFUN_BASEMIND_NOAVX2;
    else process.env.OPENBITFUN_BASEMIND_NOAVX2 = previous;
  }
});
