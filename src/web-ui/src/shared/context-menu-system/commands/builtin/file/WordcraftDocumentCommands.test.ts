import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CommandExecutor } from '../../CommandExecutor';
import { CommandRegistry } from '../../CommandRegistry';
import { ContextType, type FileNodeContext } from '../../../types/context.types';
import {
  canOpenInWordcraft,
  isWordcraftDocumentPath,
  NewWordcraftDocumentCommand,
  OpenInWordcraftCommand,
} from './WordcraftDocumentCommands';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  createWorkspaceFile: vi.fn(),
  openFileInBestTarget: vi.fn(),
  getDocumentAgentContextForFile: vi.fn(),
  notifyError: vi.fn(),
  currentWorkspace: null,
}));

vi.mock('@/infrastructure/api/service-api/ApiClient', () => ({
  api: { invoke: mocks.invoke },
}));

vi.mock('@/infrastructure/services/business/workspaceManager', () => ({
  workspaceManager: { getState: () => ({ currentWorkspace: mocks.currentWorkspace }) },
}));

vi.mock('@/infrastructure/api/service-api/WorkspaceAPI', () => ({
  workspaceAPI: { createWorkspaceFile: mocks.createWorkspaceFile },
}));

vi.mock('@/shared/utils/tabUtils', () => ({
  openFileInBestTarget: mocks.openFileInBestTarget,
}));

vi.mock('@/tools/editor/services/DocumentAgentContext', () => ({
  getDocumentAgentContextForFile: mocks.getDocumentAgentContextForFile,
}));

vi.mock('@/infrastructure/i18n', () => ({
  i18nService: Object.assign((key: string) => key, { t: (key: string) => key, getT: () => (key: string) => key }),
}));

vi.mock('@/shared/notification-system/services/NotificationService', () => ({
  notificationService: { error: mocks.notifyError },
}));

function fileContext(
  type: ContextType.FILE_NODE | ContextType.FOLDER_NODE,
  filePath: string,
): FileNodeContext {
  return {
    type,
    filePath,
    workspacePath: '/tmp/workspace',
    fileName: filePath.split('/').pop(),
    isDirectory: type === ContextType.FOLDER_NODE,
  } as FileNodeContext;
}

describe('OpenInWordcraftCommand', () => {
  let command: OpenInWordcraftCommand;
  let executor: CommandExecutor;

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getDocumentAgentContextForFile.mockReset();
    mocks.currentWorkspace = null;
    command = new OpenInWordcraftCommand();
    const registry = new CommandRegistry();
    registry.register(command);
    executor = new CommandExecutor(registry, { enableHistory: false });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('accepts only local .docx files', () => {
    expect(isWordcraftDocumentPath('report.docx')).toBe(true);
    expect(isWordcraftDocumentPath('Report.DOCX')).toBe(true);
    expect(isWordcraftDocumentPath('notes.rtf')).toBe(false);
    expect(canOpenInWordcraft(fileContext(ContextType.FILE_NODE, '/repo/report.docx'))).toBe(true);
    expect(canOpenInWordcraft(fileContext(ContextType.FILE_NODE, '/repo/.docx'))).toBe(false);
    expect(canOpenInWordcraft(fileContext(ContextType.FOLDER_NODE, '/repo'))).toBe(false);
  });

  it('refuses remote workspace paths before launch', () => {
    const context = fileContext(ContextType.FILE_NODE, '/repo/report.docx');
    context.resourceScope = { surfaceId: 'local', workspaceId: 'w', workspacePath: '/x', remoteConnectionId: 'ssh-a' };
    expect(canOpenInWordcraft(context)).toBe(false);
  });

  it('invokes the desktop open command with the document path', async () => {
    mocks.invoke.mockResolvedValue({
      appPath: '/opt/wordcraft/wordcraft',
      controlHost: '127.0.0.1',
      controlPort: 45123,
      profileDir: '/profiles/a',
    });
    const result = await command.execute(fileContext(ContextType.FILE_NODE, '/repo/report.docx'));
    expect(result.success).toBe(true);
    expect(mocks.invoke).toHaveBeenCalledWith('wordcraft_open_document_command', {
      request: { path: '/repo/report.docx', workspaceId: undefined },
    });
    expect(mocks.notifyError).not.toHaveBeenCalled();
  });

  it('refuses a second native writer while the embedded surface owns the file', async () => {
    const context = fileContext(ContextType.FILE_NODE, '/repo/report.docx');
    context.resourceScope = { surfaceId: 'local', workspaceId: 'workspace-1' };
    mocks.getDocumentAgentContextForFile.mockReturnValue({ status: 'ready' });
    const result = await command.execute(context);
    expect(result.success).toBe(false);
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(mocks.notifyError).toHaveBeenCalledWith('errors:file.wordcraftOpenFailed', {
      title: 'errors:file.wordcraftOpenFailed',
    });
  });

  it('surfaces launch failures loudly with the backend remediation message', async () => {
    mocks.invoke.mockRejectedValue(
      new Error(
        'WordCraft is not available (not-installed). Managed component has not been downloaded yet. Reinstall, then Recheck.'
      ),
    );
    const result = await command.execute(fileContext(ContextType.FILE_NODE, '/repo/report.docx'));
    expect(result.success).toBe(false);
    expect(mocks.notifyError).toHaveBeenCalledWith(
      'WordCraft is not available (not-installed). Managed component has not been downloaded yet. Reinstall, then Recheck.',
      { title: 'errors:file.wordcraftOpenFailed' },
    );
  });

  it('executes through the registered command id', async () => {
    mocks.invoke.mockResolvedValue({ appPath: '/app', controlHost: '127.0.0.1', controlPort: 1, profileDir: '/p' });
    const result = await executor.execute('file.open-in-wordcraft', fileContext(ContextType.FILE_NODE, '/repo/report.docx'));
    expect(result.success).toBe(true);
  });
});

describe('NewWordcraftDocumentCommand', () => {
  let command: NewWordcraftDocumentCommand;
  let executor: CommandExecutor;

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.currentWorkspace = null;
    mocks.createWorkspaceFile.mockResolvedValue(undefined);
    // Default routing: normal opener path reports a successful route once
    // the (mocked) async WordCraft decision resolves.
    mocks.openFileInBestTarget.mockImplementation(
      (options: { onWordcraftRouted?: (routed: boolean) => void }) => {
        options.onWordcraftRouted?.(true);
      },
    );
    command = new NewWordcraftDocumentCommand();
    const registry = new CommandRegistry();
    registry.register(command);
    executor = new CommandExecutor(registry, { enableHistory: false });
  });

  it('creates a blank document and routes it through the normal file opener', async () => {
    const context = fileContext(ContextType.FOLDER_NODE, '/repo');
    context.resourceScope = {
      surfaceId: 'local',
      workspaceId: 'workspace-1',
      workspacePath: '/repo',
    };
    const result = await command.execute(context);
    expect(result.success).toBe(true);
    expect(mocks.createWorkspaceFile).toHaveBeenCalledWith(
      'workspace-1',
      '/repo/New WordCraft Document.docx',
      true,
    );
    expect(mocks.openFileInBestTarget).toHaveBeenCalledWith(
      expect.objectContaining({
        filePath: '/repo/New WordCraft Document.docx',
        fileName: 'New WordCraft Document.docx',
        scope: context.resourceScope,
        workspaceId: 'workspace-1',
        workspacePath: '/repo',
      }),
    );
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it('reports a refused WordCraft route as a failed command (F9)', async () => {
    const context = fileContext(ContextType.FOLDER_NODE, '/repo');
    context.resourceScope = {
      surfaceId: 'local',
      workspaceId: 'workspace-1',
      workspacePath: '/repo',
    };
    // The status probe fails inside the opener; routing resolves `false`
    // while the loud notification path still runs.
    mocks.openFileInBestTarget.mockImplementation(
      (options: { onWordcraftRouted?: (routed: boolean) => void }) => {
        options.onWordcraftRouted?.(false);
      },
    );
    const result = await command.execute(context);
    expect(result.success).toBe(false);
    expect(result.message).toBe('errors:file.wordcraftOpenFailed');
    expect(result.data).toEqual({ path: '/repo/New WordCraft Document.docx' });
  });

  it('surfaces blank-document creation failures loudly', async () => {
    const context = fileContext(ContextType.FOLDER_NODE, '/repo');
    context.resourceScope = { surfaceId: 'local', workspaceId: 'workspace-1' };
    mocks.createWorkspaceFile.mockRejectedValue(new Error('workspace is unavailable'));
    const result = await command.execute(context);
    expect(result.success).toBe(false);
    expect(mocks.notifyError).toHaveBeenCalledWith('workspace is unavailable', {
      title: 'errors:file.wordcraftOpenFailed',
    });
    expect(mocks.openFileInBestTarget).not.toHaveBeenCalled();
  });

  it('executes through the registered command id', async () => {
    const registry = new CommandRegistry();
    registry.register(command);
    const localExecutor = new CommandExecutor(registry, { enableHistory: false });
    const context = fileContext(ContextType.FOLDER_NODE, '/repo');
    context.resourceScope = { surfaceId: 'local', workspaceId: 'workspace-1' };
    const result = await localExecutor.execute('file.new-wordcraft-document', context);
    expect(result.success).toBe(true);
    expect(mocks.createWorkspaceFile).toHaveBeenCalled();
  });
});
