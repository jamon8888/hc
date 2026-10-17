import { BaseCommand } from '../../BaseCommand';
import { CommandResult } from '../../../types/command.types';
import {
  ContextType,
  FileNodeContext,
  MenuContext,
  TabContext,
} from '../../../types/context.types';
import { wordcraftAPI } from '@/infrastructure/api/service-api/WordcraftAPI';
import { i18nService } from '@/infrastructure/i18n';
import { notificationService } from '@/shared/notification-system/services/NotificationService';
import { createLogger } from '@/shared/utils/logger';
import { canRevealInExplorer } from './RevealInExplorerCommand';

const log = createLogger('OpenInWordcraftCommands');

/** Day-one WordCraft file scope; WordCraft owns other formats later. */
export function isWordcraftDocumentPath(filePath: string): boolean {
  const base = filePath.toLowerCase().split('/').pop() ?? '';
  // ".docx" with no document stem is a hidden-style file name, not a WordCraft
  // document (desktop resolution refuses it too).
  return base.length > 5 && base.endsWith('.docx');
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
    // Creating via the start screen only needs a local, active surface; there
    // is no file path involved yet.
    return canRevealInExplorer(context);
  }

  async execute(context: MenuContext): Promise<CommandResult> {
    if (!this.canExecute(context)) {
      return this.failure(i18nService.t('errors:file.wordcraftOpenFailed'));
    }
    return launchWordcraft(() => wordcraftAPI.openDocument({}), {});
  }
}
