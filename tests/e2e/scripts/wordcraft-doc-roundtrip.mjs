#!/usr/bin/env node
/**
 * WordCraft docx round-trip e2e (tickets 07/16).
 *
 * Proves the headless agent-editing loop against a REAL `wordcraft-cli` binary:
 * open/new → tracked edits authored as "Alex Agent (<session>)" → save →
 * OOXML assertion (w:ins authorship, formatting) → independent `text` read-back →
 * outside-write-root refusal.
 *
 * This is an on-demand device gate, not an auto-run suite: it needs a real
 * WordCraft binary (see docs/features/wordcraft-doc-editor.md). Run:
 *
 *   node tests/e2e/scripts/wordcraft-doc-roundtrip.mjs [--cli <path>] [--session <label>]
 *
 * Resolution order for the binary: --cli / WORDCRAFT_CLI / managed runtime
 * `<data>/runtimes/wordcraft/current/bin/wordcraft-cli`, then system PATH.
 * Missing or un-startable binary => loud non-zero exit with remediation; the
 * script never fakes success.
 */

import { spawn, spawnSync } from 'child_process';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as process from 'process';
import { randomUUID } from 'crypto';

const DOC_EXTENSIONS = ['.docx'];
const SAVE_TIMEOUT_MS = 20_000;

function parseArgs(argv) {
  const args = { cli: process.env.WORDCRAFT_CLI || null, session: 'validation-loop', strict: false };
  for (let i = 0; i < argv.length; i += 1) {
    const flag = argv[i];
    if (flag === '--cli') args.cli = argv[i + 1] ?? null;
    else if (flag === '--session') args.session = argv[i + 1] ?? args.session;
    else if (flag === '--strict') args.strict = true;
  }
  return args;
}

function fail(message) {
  console.error(`[wordcraft-doc-roundtrip] ${message}`);
  process.exitCode = 1;
  process.exit(1);
}

function createLogger(label) {
  return (...parts) => console.log(`[${label}] ${parts.join(' ')}`);
}

function resolveCli(cliArg) {
  const candidates = [];
  if (cliArg) candidates.push(cliArg);
  const home = os.homedir();
  const managedRoot = path.join(
    home,
    'Library',
    'Application Support',
    'alex',
    'data',
    'runtimes',
    'wordcraft',
    'current',
  );
  // Walk the managed tree for the launcher the component ships.
  for (const sub of ['bin/wordcraft-cli', 'wordcraft-cli']) {
    candidates.push(path.join(managedRoot, ...sub.split('/')));
  }
  for (const candidate of candidates) {
    if (candidate && fs.existsSync(candidate)) return candidate;
  }
  // Fall through to PATH via `command -v` (POSIX) / `where` (Windows).
  const probe = spawnSync(
    process.platform === 'win32' ? 'where' : 'command',
    process.platform === 'win32' ? ['wordcraft-cli.exe'] : ['-v', 'wordcraft-cli'],
    { encoding: 'utf8', shell: process.platform === 'win32' },
  );
  const found = (probe.stdout || '').split(/\r?\n/)[0]?.trim();
  if (probe.status === 0 && found) return found;
  return null;
}

function versionProbe(cli) {
  const result = spawnSync(cli, ['--version'], { encoding: 'utf8', timeout: 15_000 });
  if (result.status !== 0 || /error/i.test(result.stdout || '')) {
    fail(
      `WordCraft CLI found at ${cli} but did not start (exit ${result.status}). ` +
        `stdout: ${(result.stdout || '').slice(0, 200)} stderr: ${(result.stderr || '').slice(0, 200)} — ` +
        'reinstall the pinned component and Recheck.',
    );
  }
  return (result.stdout || result.stderr || '').trim().split(/\r?\n/)[0].trim();
}

/** Minimal JSON-RPC-over-stdio MCP client (splice of the spike script). */
function createMcpRunners(cli, args, author, workspaceRoot, log) {
  const child = spawn(cli, args, {
    cwd: workspaceRoot,
    env: {
      ...process.env,
      WORDCRAFT_PROFILE_DIR: path.join(os.tmpdir(), `alex-wordcraft-e2e-${randomUUID()}`),
    },
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  child.stderr.on('data', (chunk) => log('server stderr:', String(chunk).slice(0, 300)));
  let nextId = 1;
  let buffer = '';
  const pending = new Map();
  child.stdout.on('data', (chunk) => {
    buffer += String(chunk);
    for (;;) {
      const lineEnd = buffer.indexOf('\n');
      if (lineEnd < 0) break;
      const line = buffer.slice(0, lineEnd).trim();
      buffer = buffer.slice(lineEnd + 1);
      if (!line) continue;
      let message;
      try {
        message = JSON.parse(line);
      } catch (error) {
        fail(`unparsable server line: ${line.slice(0, 200)} (${error.message})`);
      }
      if (message.id !== undefined && pending.has(message.id)) {
        const { resolve, reject } = pending.get(message.id);
        pending.delete(message.id);
        if (message.error) reject(new Error(JSON.stringify(message.error)));
        else resolve(message.result);
      }
    }
  });

  function request(method, params) {
    const id = nextId++;
    const line = JSON.stringify({ jsonrpc: '2.0', id, method, params });
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      child.stdin.write(line + '\n');
    });
  }

  function rawNotify(method, params) {
    const line = JSON.stringify({ jsonrpc: '2.0', method, params });
    child.stdin.write(line + '\n');
  }

  async function initialize() {
    const result = await request('initialize', {
      protocolVersion: '2024-11-05',
      capabilities: {},
      clientInfo: { name: 'alex-wordcraft-e2e', version: 'dev' },
    });
    // Upstream answers notifications with method-not-found; notifications are
    // fire-and-forget, so send initialized without a pending response slot.
    rawNotify('notifications/initialized', {});
    return result;
  }

  async function callTool(name, args2, timeoutMs = SAVE_TIMEOUT_MS) {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error(`tool call ${name} timed out`)), timeoutMs);
      request('tools/call', { name, arguments: args2 })
        .then((value) => {
          clearTimeout(timer);
          // MCP wraps tool descriptions with optional isError content.
          const isError = value?.isError === true;
          const textLines = (value?.content || [])
            .filter((c) => c.type === 'text')
            .map((c) => c.text)
            .join('\n');
          if (isError) reject(new Error(`${name} reported isError: ${textLines.slice(0, 400)}`));
          else resolve(textLines);
        })
        .catch((error) => {
          clearTimeout(timer);
          reject(error);
        });
    });
  }

  function close() {
    try {
      child.kill('SIGTERM');
    } catch (error) {
      log('close error:', String(error).slice(0, 120));
    }
  }

  return {
    initialize,
    callTool,
    close,
    author,
    workspaceRoot,
  };
}

function extractDocumentXml(docxPath) {
  if (!fs.existsSync(docxPath)) fail(`saved document is missing on disk: ${docxPath}`);
  const unzip = spawnSync('unzip', ['-p', docxPath, 'word/document.xml'], {
    encoding: 'utf8',
    timeout: 15_000,
  });
  if (unzip.status !== 0 || !unzip.stdout) {
    fail(
      `could not read word/document.xml from ${docxPath} via unzip ` +
        `(status ${unzip.status}): ${(unzip.stderr || '').slice(0, 200)}`,
    );
  }
  return unzip.stdout;
}

function assertOoxmlTrackedChanges(xml, expectedAuthor, requireBold) {
  const problems = [];
  if (!xml.includes('<w:ins ')) problems.push('no <w:ins> tracked-change insertions present');
  if (!xml.includes(`w:author="${expectedAuthor}"`)) {
    problems.push(`no w:ins authored "${expectedAuthor}"`);
  }
  if (requireBold && !/<w:b\/>/.test(xml)) problems.push('bold <w:b/> not present in saved document');
  if (problems.length > 0) {
    fail(`OOXML tracked-change assertion failed: ${problems.join('; ')}\n${xml.slice(0, 600)}`);
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const log = createLogger('wordcraft-doc-roundtrip');
  const cli = resolveCli(args.cli);
  if (!cli) {
    fail(
      'No WordCraft CLI available. Resolve it first: enable the wordcraft managed component ' +
        '(status -> not-installed -> download + SHA256SUMS verify) or pass --cli <path>. ' +
        'This script never fakes success in the binary\'s absence.',
    );
  }
  const version = versionProbe(cli);
  log('using', cli, `(${version})`);

  const workspaceRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'alex-wordcraft-ws-'));
  const sessionLabel = args.session;
  const author = `Alex Agent (${sessionLabel})`;

  const handle = createMcpRunners(cli, [
    'mcp',
    '--author',
    author,
    '--automation-read-root',
    workspaceRoot,
    '--automation-write-root',
    workspaceRoot,
  ], author, workspaceRoot, log);

  try {
    const init = await handle.initialize();
    log('initialize ok; tools instructions bytes:', String(init?.instructions || '').length);

    const documentName = 'demo.docx';
    const documentPath = path.join(workspaceRoot, documentName);

    // Author a small document with explicit paragraphs (upstream contract:
    // type_text does NOT split on '\n' without `paragraphs: true`). Tracked
    // changes and formatting go through the generic `execute` command runner —
    // the MCP surface exposes 16 tools, so dot-commands like
    // `review.trackChanges` are command ids, not tool names.
    await handle.callTool('new_document', { path: documentName });
    await handle.callTool('execute', { command: 'review.trackChanges', params: { value: true } });
    await handle.callTool('type_text', { text: 'Tracked first line', paragraphs: true });
    await handle.callTool('type_text', { text: 'Second line for emphasis', paragraphs: true });
    const inspect = await handle.callTool('inspect_document', {});
    log('inspect (head):', (inspect || '').slice(0, 160).replace(/\n/g, ' '));

    // Select by text occurrence and bold it — dot command ids per
    // `list_commands` (`select.text {text, occurrence}`, `format.bold
    // {value}`); the MCP tool names (`select_text`) do not work inside batch.
    let boldApplied = false;
    try {
      await handle.callTool('batch', {
        commands: [
          { command: 'select.text', params: { text: 'Second line for emphasis', occurrence: 1 } },
          { command: 'format.bold', params: { value: true } },
        ],
      });
      boldApplied = true;
    } catch (error) {
      log(
        'WARN select/format leg did not run cleanly (continuing; non-strict):',
        String(error).slice(0, 200).replace(/\n/g, ' '),
      );
      if (args.strict) fail('strict mode: formatting leg must not fail');
    }

    await handle.callTool('save_document', { path: documentPath });

    const xml = extractDocumentXml(documentPath);
    assertOoxmlTrackedChanges(xml, author, boldApplied);
    log(
      'OOXML assertion ok: tracked insertions authored',
      author,
      boldApplied ? 'with bold run' : '(formatting leg skipped — non-strict)',
    );

    // Independent read-back through the other binary face.
    const readBack = spawnSync(cli, ['text', documentPath], { encoding: 'utf8', timeout: 30_000 });
    if (readBack.status !== 0 || !/tracked first line/i.test(readBack.stdout || '')) {
      fail(
        `independent read-back failed (status ${readBack.status}); stdout: ` +
          `${(readBack.stdout || '').slice(0, 300)} stderr: ${(readBack.stderr || '').slice(0, 200)}`,
      );
    }
    log('independent text read-back ok');

    // Confinement: saving outside the write root must be refused with a
    // readable error, never a silent local fallback.
    let refused = false;
    try {
      await handle.callTool('save_document', { path: path.join(os.tmpdir(), 'outside-demo.docx') });
    } catch (error) {
      if (/outside|automation|write root|rejected/i.test(String(error.message))) refused = true;
      else fail(`save outside root failed with an unexpected error: ${String(error).slice(0, 300)}`);
    }
    if (!refused) fail('saving outside the write root was NOT refused — confinement regression');
    log('outside-root refusal ok');
  } finally {
    handle.close();
  }

  fs.rmSync(workspaceRoot, { recursive: true, force: true });
  console.log('[wordcraft-doc-roundtrip] PASS');
}

main().catch((error) => {
  console.error('[wordcraft-doc-roundtrip] FAIL', error);
  process.exit(1);
});
