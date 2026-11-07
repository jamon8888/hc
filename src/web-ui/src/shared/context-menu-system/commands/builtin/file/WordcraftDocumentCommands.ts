import { BaseCommand } from '../../BaseCommand';
import { CommandResult } from '../../../types/command.types';
import {
  ContextType,
  FileNodeContext,
  MenuContext,
  TabContext,
} from '../../../types/context.types';
import { wordcraftAPI } from '@/infrastructure/api/service-api/WordcraftAPI';
import { workspaceAPI } from '@/infrastructure/api/service-api/WorkspaceAPI';
import { i18nService } from '@/infrastructure/i18n';
import { notificationService } from '@/shared/notification-system/services/NotificationService';
import { createLogger } from '@/shared/utils/logger';
import { openFileInBestTarget } from '@/shared/utils/tabUtils';
import { joinWorkspaceTargetPath } from '@/tools/file-system/services/workspaceFileTransfer';
import { getDocumentAgentContextForFile } from '@/tools/editor/services/DocumentAgentContext';
import { canRevealInExplorer } from './RevealInExplorerCommand';

const log = createLogger('OpenInWordcraftCommands');

/** Day-one WordCraft file scope; WordCraft owns other formats later. */
export function isWordcraftDocumentPath(filePath: string): boolean {
  const base = filePath.toLowerCase().split('/').pop() ?? '';
  // ".docx" with no document stem is a hidden-style file name, not a WordCraft
  // document (desktop resolution refuses it too).
  if (base.length > 5 && base.endsWith('.docx')) return true;
  // Legacy ".doc" shares the WordCraft route. Without a routed editor the
  // fallback opened the binary blob in the code editor, producing the
  // "hieroglyphs" experience users reported.
  return base.length > 4 && base.endsWith('.doc');
}

function getContextFilePath(context: MenuContext): string | undefined {
  if (context.type === ContextType.FILE_NODE || context.type === ContextType.FOLDER_NODE) {
    return (context as FileNodeContext).filePath;
  }

  if (context.type === ContextType.TAB) {
    return (context as TabContext).filePath;
  }

  return undefined;
}

/**
 * WordCraft opens its own OS window on the desktop host, so remote workspace
 * paths (and any surface other than the active one) are refused before launch.
 * Reuses the local-path guard shared by the native reveal actions.
 */
export function canOpenInWordcraft(context: MenuContext): boolean {
  const filePath = getContextFilePath(context);
  return Boolean(filePath && isWordcraftDocumentPath(filePath) && canRevealInExplorer(context));
}

function isCreateConflict(error: unknown): boolean {
  return error instanceof Error && /FILE_CONFLICT|already exists|choose another name/i.test(error.message);
}

function newWordcraftFileName(index: number): string {
  return index === 1
    ? 'New WordCraft Document.docx'
    : `New WordCraft Document (${index}).docx`;
}

/** Shared launcher so both entries report failures loudly and identically. */
async function launchWordcraft(
  openDocument: () => Promise<unknown>,
  detail: { documentPath?: string }
): Promise<CommandResult> {
  const t = i18nService.getT();
  try {
    await openDocument();
    return {
      success: true,
      message: t(detail.documentPath ? 'common:file.openInWordcraftOpening' : 'common:file.newWordcraftDocumentOpening'),
      data: detail,
    };
  } catch (error) {
    const cause = error instanceof Error ? error : new Error(String(error));
    const message = t('errors:file.wordcraftOpenFailed');
    log.error('Failed to open WordCraft editor window', {
      documentPath: detail.documentPath,
      error: cause,
    });
    // Loud degradation: the backend message carries the shared status record's
    // remediation verbatim; show it next to the generic failure title.
    notificationService.error(cause.message || message, { title: message });
    return { success: false, message, data: cause };
  }
}

/**
 * Menu action: open a local `.docx` document in the visible WordCraft editor
 * window. The editor owns its chrome, profile, and control channel.
 */
export class OpenInWordcraftCommand extends BaseCommand {
  constructor() {
    const t = i18nService.getT();
    super({
      id: 'file.open-in-wordcraft',
      label: t('common:file.openInWordcraft'),
      description: t('common:file.openInWordcraftDescription'),
      icon: 'FileText',
      category: 'file'
    });
  }

  canExecute(context: MenuContext): boolean {
    return canOpenInWordcraft(context);
  }

  async execute(context: MenuContext): Promise<CommandResult> {
    const filePath = getContextFilePath(context);
    if (!filePath || !this.canExecute(context)) {
      return this.failure(i18nService.t('errors:file.wordcraftOpenFailed'));
    }
    const scope = context.resourceScope;
    if (getDocumentAgentContextForFile(filePath, scope)) {
      const message = i18nService.t('errors:file.wordcraftOpenFailed');
      notificationService.error(message, { title: message });
      return this.failure(message);
    }
    return launchWordcraft(
      () => wordcraftAPI.openDocument({ path: filePath }),
      { documentPath: filePath }
    );
  }
}

/**
 * Menu action: open the WordCraft start screen; document creation (blank,
 * templates, save location) stays inside WordCraft's own UI.
 */
export class NewWordcraftDocumentCommand extends BaseCommand {
  constructor() {
    const t = i18nService.getT();
    super({
      id: 'file.new-wordcraft-document',
      label: t('common:file.newWordcraftDocument'),
      description: t('common:file.newWordcraftDocumentDescription'),
      icon: 'FilePlus',
      category: 'file'
    });
  }

  canExecute(context: MenuContext): boolean {
    // The blank document must be created through the workspace adapter so the
    // embedded surface receives a real file identity and can save it through
    // the normal optimistic-write path.
    return Boolean(context.resourceScope?.workspaceId && canRevealInExplorer(context));
  }

  async execute(context: MenuContext): Promise<CommandResult> {
    const parentPath = getContextFilePath(context);
    const scope = context.resourceScope;
    const workspaceId = scope?.workspaceId;
    if (!parentPath || !workspaceId || !this.canExecute(context)) {
      return this.failure(i18nService.t('errors:file.wordcraftOpenFailed'));
    }

    const message = i18nService.t('errors:file.wordcraftOpenFailed');
    let fileName = '';
    let filePath = '';
    try {
      // WordCraft intentionally accepts a zero-byte .docx as its blank
      // document representation. Creating it here gives the embedded panel a
      // stable path, workspace scope, and persistence target immediately.
      for (let index = 1; index <= 100; index += 1) {
        fileName = newWordcraftFileName(index);
        filePath = joinWorkspaceTargetPath(parentPath, fileName, false);
        try {
          await workspaceAPI.createWorkspaceFile(workspaceId, filePath, true);
          break;
        } catch (error) {
          if (!isCreateConflict(error) || index === 100) throw error;
        }
      }
      // The routing verdict is observable (CodeRabbit F9): a refused open
      // must not record a successful history entry even though the file was
      // created. The fabricated .docx name always routes through the
      // WordCraft decision, which reports exactly once.
      const routed = await new Promise<boolean>((resolve) => {
        openFileInBestTarget({
          filePath,
          fileName,
          scope,
          workspaceId,
          workspacePath: scope.workspacePath,
          onWordcraftRouted: resolve,
        });
      });
      if (!routed) {
        return { success: false, message, data: { path: filePath } };
      }
      return {
        success: true,
        message: i18nService.t('common:file.newWordcraftDocumentOpening'),
        data: { path: filePath },
      };
    } catch (error) {
      const cause = error instanceof Error ? error : new Error(String(error));
      log.error('Failed to create a new WordCraft document', { filePath, error: cause });
      notificationService.error(cause.message || message, { title: message });
      return { success: false, message, data: cause };
    }
  }
}
