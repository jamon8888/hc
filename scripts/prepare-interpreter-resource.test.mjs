import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import {
  ensureInterpreterPackage,
  interpreterAssetName,
  interpreterHostTag,
  interpreterPackageDirectory,
  interpreterTarget,
} from './prepare-interpreter-resource.mjs';

for (const [target, tag] of [
  ['x86_64-apple-darwin', 'macos-x86_64'],
  ['aarch64-apple-darwin', 'macos-aarch64'],
  ['x86_64-pc-windows-msvc', 'windows-x86_64'],
  ['aarch64-pc-windows-msvc', 'windows-aarch64'],
  ['x86_64-unknown-linux-musl', 'linux-x86_64'],
  ['aarch64-unknown-linux-musl', 'linux-aarch64'],
]) {
  test(`selects ${target} independently of the build host`, () => {
    assert.equal(interpreterTarget({ target, platform: 'darwin', arch: 'arm64' }), target);
    // The host tag follows the target, so a cross-compiled package is
    // labelled for the machine it will run on.
    assert.equal(
      interpreterHostTag({ target, platform: 'darwin', arch: 'arm64' }),
      tag,
    );
  });
}

test('the host tag follows the target rather than the build host', () => {
  assert.equal(
    interpreterHostTag({ target: 'x86_64-unknown-linux-musl', platform: 'darwin', arch: 'arm64' }),
    'linux-x86_64',
  );
  assert.equal(
    interpreterHostTag({ target: 'x86_64-pc-windows-msvc', platform: 'linux', arch: 'x64' }),
    'windows-x86_64',
  );
});

test('without an explicit target the host decides', () => {
  assert.equal(interpreterHostTag({ platform: 'darwin', arch: 'arm64' }), 'macos-aarch64');
  assert.equal(interpreterHostTag({ platform: 'linux', arch: 'x64' }), 'linux-x86_64');
  assert.equal(interpreterHostTag({ platform: 'win32', arch: 'x64' }), 'windows-x86_64');
});

test('a linux gnu target resolves to the musl asset', () => {
  assert.equal(
    interpreterTarget({ target: 'x86_64-unknown-linux-gnu' }),
    'x86_64-unknown-linux-musl',
  );
});

test('unsupported targets fail explicitly', () => {
  assert.throws(() => interpreterTarget({ target: 'riscv64-unknown-linux-gnu' }), /Unsupported/);
  assert.throws(() => interpreterHostTag({ platform: 'linux', arch: 'ia32' }), /Unsupported/);
});
function buildFakePackage(root) {
  mkdirSync(join(root, 'bin'), { recursive: true });
  mkdirSync(join(root, 'codex-path'), { recursive: true });
  mkdirSync(join(root, 'codex-resources', 'zsh', 'bin'), { recursive: true });
  writeFileSync(join(root, 'bin/interpreter'), '#!/bin/sh\nexit 0\n');
  writeFileSync(join(root, 'bin/i'), '#!/bin/sh\nexit 0\n');
  writeFileSync(join(root, 'codex-path/rg'), 'rg\n');
  // A nested helper, like the macOS package's zsh. It has to be discovered,
  // because an unsigned nested binary fails notarization.
  writeFileSync(join(root, 'codex-resources/zsh/bin/zsh'), '#!/bin/sh\nexit 0\n');
  writeFileSync(
    join(root, 'codex-package.json'),
    JSON.stringify({
      layoutVersion: 1,
      version: '0.0.56',
      entrypoint: 'bin/interpreter',
      resourcesDir: 'codex-resources',
      pathDir: 'codex-path',
    }),
  );
}

function tarFakePackage(root, destination) {
  const result = spawnSync('tar', ['-czf', destination, '-C', root, '.'], { encoding: 'utf8' });
  if (result.error || result.status !== 0) {
    throw new Error(`failed to build fixture archive: ${result.error?.message || result.stderr}`);
  }
}

test('a verified download is extracted into a package the product can read', () => {
  const workspace = mkdtempSync(join(tmpdir(), 'interpreter-download-'));
  const resourceDir = join(workspace, 'resources');
  const fixtureRoot = join(workspace, 'fixture');
  mkdirSync(resourceDir, { recursive: true });
  writeFileSync(join(resourceDir, 'LICENSE'), 'Apache License 2.0');
  writeFileSync(join(resourceDir, 'NOTICE'), 'Open Interpreter notice');
  buildFakePackage(fixtureRoot);
  const archivePath = join(workspace, 'package.tar.gz');
  tarFakePackage(fixtureRoot, archivePath);
  const bytes = readFileSync(archivePath);

  const target = 'aarch64-apple-darwin';
  const manifest = {
    repo: 'openinterpreter/openinterpreter',
    tag: 'rust-v0.0.56',
    version: '0.0.56',
    assetStem: 'open-interpreter-package',
    assets: {
      [target]: { sha256: createHash('sha256').update(bytes).digest('hex'), size: bytes.length },
    },
  };

  let downloads = 0;
  const options = {
    resourceDir,
    target,
    manifest,
    platform: 'darwin',
    arch: 'arm64',
    download(url, destination) {
      downloads++;
      assert.equal(
        url,
        `https://github.com/openinterpreter/openinterpreter/releases/download/rust-v0.0.56/open-interpreter-package-${target}.tar.gz`,
      );
      writeFileSync(destination, bytes);
    },
  };

  try {
    const packageDirectory = ensureInterpreterPackage(options);
    assert.equal(packageDirectory, interpreterPackageDirectory({ resourceDir, target }));

    // The product reads this manifest, not the upstream descriptor.
    const packageManifest = JSON.parse(
      readFileSync(join(packageDirectory, 'alex-bundled-tool.json'), 'utf8'),
    );
    assert.equal(packageManifest.toolId, 'interpreter');
    assert.equal(packageManifest.version, '0.0.56');
    assert.equal(packageManifest.hostTag, 'macos-aarch64');
    assert.equal(packageManifest.target, target);
    assert.equal(packageManifest.entrypoint, 'bin/interpreter');
    assert.equal(packageManifest.sourceSha256, manifest.assets[target].sha256);
    assert.ok(existsSync(join(packageDirectory, 'bin/interpreter')));
    assert.ok(existsSync(join(packageDirectory, 'codex-path/rg')));
    // Apache-2.0 requires the license and the upstream NOTICE to travel with
    // the redistributed code.
    assert.equal(readFileSync(join(packageDirectory, 'LICENSE'), 'utf8'), 'Apache License 2.0');
    assert.equal(readFileSync(join(packageDirectory, 'NOTICE'), 'utf8'), 'Open Interpreter notice');

    // The files that are code are derived from the upstream descriptor, so a
    // nested helper is covered and the package's data files are not.
    assert.deepEqual(packageManifest.executables, [
      'bin/i',
      'bin/interpreter',
      'codex-path/rg',
      'codex-resources/zsh/bin/zsh',
    ]);
    for (const dataFile of ['LICENSE', 'NOTICE', 'codex-package.json', 'alex-bundled-tool.json']) {
      assert.ok(
        !packageManifest.executables.includes(dataFile),
        `${dataFile} is a data file, not code`,
      );
    }

    // A second run reuses the packaged copy instead of downloading again.
    assert.equal(ensureInterpreterPackage(options), packageDirectory);
    assert.equal(downloads, 1);
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('a download that does not match the pinned digest never becomes a package', () => {
  const workspace = mkdtempSync(join(tmpdir(), 'interpreter-corrupt-'));
  const resourceDir = join(workspace, 'resources');
  mkdirSync(resourceDir, { recursive: true });
  const target = 'aarch64-apple-darwin';
  const manifest = {
    repo: 'openinterpreter/openinterpreter',
    tag: 'rust-v0.0.56',
    version: '0.0.56',
    assetStem: 'open-interpreter-package',
    assets: { [target]: { sha256: 'a'.repeat(64), size: 10 } },
  };
  try {
    assert.throws(
      () => ensureInterpreterPackage({
        resourceDir,
        target,
        manifest,
        platform: 'darwin',
        arch: 'arm64',
        download: (_, destination) => writeFileSync(destination, 'not the release'),
      }),
      /mismatch/,
    );
    assert.equal(existsSync(join(resourceDir, target, 'alex-bundled-tool.json')), false);
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('a pinned release without the target asset fails explicitly', () => {
  const resourceDir = mkdtempSync(join(tmpdir(), 'interpreter-missing-'));
  try {
    assert.throws(
      () => ensureInterpreterPackage({
        resourceDir,
        target: 'aarch64-apple-darwin',
        manifest: {
          repo: 'openinterpreter/openinterpreter',
          tag: 'rust-v0.0.56',
          version: '0.0.56',
          assetStem: 'open-interpreter-package',
          assets: {},
        },
        platform: 'darwin',
        arch: 'arm64',
      }),
      /Missing or invalid/,
    );
  } finally {
    rmSync(resourceDir, { recursive: true, force: true });
  }
});

test('the asset name follows the pinned stem and target', () => {
  assert.equal(
    interpreterAssetName({
      target: 'x86_64-unknown-linux-musl',
      manifest: { assetStem: 'open-interpreter-package' },
    }),
    'open-interpreter-package-x86_64-unknown-linux-musl.tar.gz',
  );
});

test('an interrupted preparation does not leave its staging directory behind', () => {
  const workspace = mkdtempSync(join(tmpdir(), 'interpreter-staging-'));
  const resourceDir = join(workspace, 'resources');
  const target = 'aarch64-apple-darwin';
  mkdirSync(resourceDir, { recursive: true });

  // A previous run was interrupted after extracting, so its staging copy is
  // still there. The package is a few hundred megabytes, so leaving one
  // behind for every interrupted run would fill the build machine.
  const stale = join(resourceDir, `.staging-${target}-interrupted`);
  mkdirSync(join(stale, 'bin'), { recursive: true });
  writeFileSync(join(stale, 'bin', 'interpreter'), 'partial');

  // A staging directory for a different target is not this run's business.
  const otherTarget = join(resourceDir, '.staging-x86_64-unknown-linux-musl-other');
  mkdirSync(otherTarget, { recursive: true });

  const manifest = {
    repo: 'openinterpreter/openinterpreter',
    tag: 'rust-v0.0.56',
    version: '0.0.56',
    assetStem: 'open-interpreter-package',
    assets: { [target]: { sha256: 'a'.repeat(64), size: 10 } },
  };

  try {
    // The download fails, but the sweep runs before the download.
    assert.throws(() => ensureInterpreterPackage({
      resourceDir,
      target,
      manifest,
      platform: 'darwin',
      arch: 'arm64',
      download: (_, destination) => writeFileSync(destination, 'not the release'),
    }));

    assert.equal(existsSync(stale), false, 'the stale staging directory should be swept');
    assert.equal(existsSync(otherTarget), true, 'another target\'s staging directory is not ours');
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('a caller-supplied pinned manifest is used instead of the repository one', () => {  // The helper must not silently fall back to the repository's pinned
  // release, or a test asserts against the real file.
  assert.equal(
    interpreterAssetName({
      target: 'x86_64-unknown-linux-musl',
      manifest: { assetStem: 'a-different-stem' },
    }),
    'a-different-stem-x86_64-unknown-linux-musl.tar.gz',
  );
});
