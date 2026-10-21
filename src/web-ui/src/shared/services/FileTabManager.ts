/**
 * File tab manager.
 *
 * Opens files beside an already open session in their workspace, or in a main tab.
 */
import { resourceFilePath } from '@/app/workbench/contentResourceStore';
import { getEditorType, isRedactedDocumentPath } from '@/infrastructure/language-detection';
import { i18nService } from '@/infrastructure/i18n';
import { basemindAPI } from '@/infrastructure/api/service-api/BasemindAPI';
import { isBasemindPeerRefused } from '@/infrastructure/basemind/basemindPeerPolicy';
import { wordcraftAPI } from '@/infrastructure/api/service-api/WordcraftAPI';
import { notificationService } from '@/shared/notification-system/services/NotificationService';
import { createLogger } from '@/shared/utils/logger';
import { isWordcraftDocumentPath } from '@/shared/context-menu-system/commands/builtin/file/WordcraftDocumentCommands';
import { type LineRange } from '@/shared/editor/LineRange';
import { captureContentScope, openContentInBestTarget } from './workbenchContentService';
import type { ContentResourceScope } from '../types/contentResource';

const log = createLogger('FileTabManager');

export interface FileTabOptions {
  scope?: ContentResourceScope;
   
  filePath: string;
   
  fileName?: string;
  /** Owning workspace ID; selects the scope when no explicit scope is given. */
  workspaceId?: string;
  /** Upgrade-only selector for callers that predate workspace IDs. */
  workspacePath?: string;
   
  jumpToLine?: number;
   
  jumpToColumn?: number;
   
  jumpToRange?: LineRange;
  
  navigationToken?: number;
   
  mode?: 'agent' | 'project';
   
  forceNew?: boolean;
   
  splitView?: boolean;
   
  targetGroup?: 'primary' | 'secondary';
  /** Compatibility input; opening no longer depends on a mounted canvas. */
  sceneJustOpened?: boolean;

  /** Explicitly choose a panel type, e.g. opening HTML as source text. */
  editorType?: ReturnType<typeof getEditorType>;
  remoteConnectionId?: string;
}

 
let fileNavigationSequence = 0;

class FileTabManager {
  private static instance: FileTabManager;

  private constructor() {}

  public static getInstance(): FileTabManager {
    if (!FileTabManager.instance) {
      FileTabManager.instance = new FileTabManager();
    }
    return FileTabManager.instance;
  }

   
  public openFile(options: FileTabOptions): void {
    const {
      filePath,
      fileName: providedFileName,
      workspaceId,
      workspacePath,
      editorType: explicitEditorType,
      remoteConnectionId,
    } = options;

    
    const scope = options.scope ?? captureContentScope({ workspaceId, workspacePath, remoteConnectionId });
    const normalizedPath = resourceFilePath(filePath, scope);
    
    
    const fileName = providedFileName || normalizedPath.split(/[/\\]/).pop() || '';
    
    
    const editorType = explicitEditorType || getEditorType(fileName);

    // WordCraft owns .docx opens (tickets 13/17): when no caller picked an
    // explicit surface ("Open as Text" keeps working), route through the
    // capability-aware WordCraft decision instead of the code-editor binary
    // dump. The decision is async behind the capability probe; surfaces
    // without the probe keep the current tab behavior.
    if (!explicitEditorType && isWordcraftDocumentPath(fileName)) {
      void this.openWordcraftOrLoudFallback({
        options,
        scope,
        filePath: normalizedPath,
        fileName,
        editorType,
      });
      return;
    }

    // Document formats the redacted viewer can read are decided behind the
    // Basemind capability probe: a host without that surface keeps the plain
    // editor instead of opening a viewer it cannot serve.
    if (!explicitEditorType && isRedactedDocumentPath(fileName)) {
      void this.openRedactedDocumentOrFallback({
        options,
        scope,
        filePath: normalizedPath,
        fileName,
      });
      return;
    }

    this.openEditorTab(options, scope, normalizedPath, fileName, editorType);
  }

  /**
   * Capability-aware routing for document formats the redacted viewer owns.
   *
   * The Basemind surface is controller-local, so a workspace that lives on a
   * connected device is not redactable here. The viewer still serves that
   * document: its source tab reads through the workspace owner, and the
   * redacted tab states that redaction is unavailable. The plain editor would
   * only show a binary dump. A host without the Basemind capability keeps the
   * plain editor, with a loud notice so the missing feature is never silent.
   */
  private async openRedactedDocumentOrFallback(args: {
    options: FileTabOptions;
    scope: ContentResourceScope;
    filePath: string;
    fileName: string;
  }): Promise<void> {
    const { options, scope, filePath, fileName } = args;
    if (isBasemindPeerRefused(scope.workspaceId, scope.surfaceId)) {
      this.openEditorTab(options, scope, filePath, fileName, 'redacted-document-viewer');
      return;
    }

    const basemindAvailable = await basemindAPI.probeSurface();
    if (!basemindAvailable) {
      const t = i18nService.getT();
      notificationService.error(t('errors:file.redactedDocumentUnavailable'), {
        title: t('errors:file.redactedDocumentUnavailable'),
      });
      log.warn('Basemind redaction surface unavailable; document opened in the plain editor', {
        filePath,
        workspaceId: scope.workspaceId,
      });
      this.openEditorTab(options, scope, filePath, fileName, 'code-editor');
      return;
    }

    this.openEditorTab(options, scope, filePath, fileName, 'redacted-document-viewer');
  }

  /** Builds and opens the regular editor tab for a file (non-WordCraft path). */
  private openEditorTab(
    options: FileTabOptions,
    scope: ContentResourceScope,
    filePath: string,
    fileName: string,
    editorType: ReturnType<typeof getEditorType>,
  ): void {
    const {
      jumpToLine,
      jumpToColumn,
      jumpToRange,
      navigationToken,
      editorType: explicitEditorType,
    } = options;
    const finalJumpToRange = jumpToRange;
    
    
    const tabData = {
      filePath: filePath,
      fileName: fileName,
      surfaceId: scope.surfaceId,
      workspaceId: scope.workspaceId,
      workspacePath: scope.workspacePath,
      remoteConnectionId: scope.remoteConnectionId,
      navigationToken: navigationToken ?? ++fileNavigationSequence,
      
      ...(finalJumpToRange && { jumpToRange: finalJumpToRange }),
      
      ...(!finalJumpToRange && jumpToLine && { jumpToLine }),
      ...(!finalJumpToRange && jumpToColumn && { jumpToColumn })
    };
    
    
    const content = {
      type: editorType,
      title: fileName,
      data: tabData,
      metadata: {
        duplicateCheckKey: explicitEditorType ? `${filePath}:${editorType}` : filePath
      }
    };

    
    openContentInBestTarget(content, { scope, replaceExisting: Boolean(explicitEditorType),
      splitView: options.splitView, targetGroup: options.targetGroup });
  }

  /**
   * Capability-aware routing for default `.docx` opens (tickets 13/17):
   * capability `available` launches the WordCraft editor window (the same
   * action as the context entry, failures loud with the record's remediation);
   * any other record state is a loud toast, never a code-editor binary dump.
   * Where the probe is unavailable (server/web surfaces with no desktop host
   * adapter) the previous tab behavior is kept — the loud path is also
   * unavailable there.
   */
  private async openWordcraftOrLoudFallback(args: {
    options: FileTabOptions;
    scope: ContentResourceScope;
    filePath: string;
    fileName: string;
    editorType: ReturnType<typeof getEditorType>;
  }): Promise<void> {
    const { options, scope, filePath, fileName, editorType } = args;
    const capability = await wordcraftAPI.statusQuiet();
    if (!capability) {
      this.openEditorTab(options, scope, filePath, fileName, editorType);
      return;
    }

    const t = i18nService.getT();
    if (capability.status !== 'available' || !capability.app) {
      const detail = capability.remediation || capability.message || t('errors:file.wordcraftOpenFailed');
      notificationService.error(detail, { title: t('errors:file.wordcraftOpenFailed') });
      log.warn('WordCraft unavailable; docx open refused loudly', {
        filePath,
        status: capability.status,
      });
      return;
    }

    try {
      await wordcraftAPI.openDocument({ path: filePath, workspaceId: scope.workspaceId });
    } catch (error) {
      const cause = error instanceof Error ? error : new Error(String(error));
      // Loud degradation: remote targets and unrunnable editors surface the
      // backend message verbatim; the code-editor dump stays forbidden.
      notificationService.error(cause.message || t('errors:file.wordcraftOpenFailed'), {
        title: t('errors:file.wordcraftOpenFailed'),
      });
      log.error('Failed to route docx open to WordCraft', { filePath, error: cause });
    }
  }

   
  public openFileAndJump(
    filePath: string,
    line: number,
    column?: number,
    options?: Partial<FileTabOptions>
  ): void {
    this.openFile({
      filePath,
      jumpToLine: line,
      jumpToColumn: column,
      ...options
    });
  }

   
  public openFileAndJumpToRange(
    filePath: string,
    range: LineRange,
    options?: Partial<FileTabOptions>
  ): void {
    this.openFile({
      filePath,
      jumpToRange: range,
      ...options
    });
  }
}


export const fileTabManager = FileTabManager.getInstance();


export type { FileTabManager };
