import { api } from './ApiClient';
import { createLogger } from '@/shared/utils/logger';

const log = createLogger('WordcraftAPI');

/**
 * WordCraft editor-surface API.
 *
 * Mirrors desktop `src/apps/desktop/src/api/wordcraft_api.rs` (camelCase
 * serde). The visible editor and its status record are desktop-host concerns;
 * failures propagate so every entry point degrades loudly with the shared
 * remediation instead of guessing locally.
 */

/** Matches desktop `WordcraftResolvedCommand` (camelCase serde). */
export interface WordcraftResolvedCommandDto {
  command: string;
  resolvedPath: string | null;
  source: string | null;
}

/** Matches desktop `WordcraftCapability` (camelCase serde). */
export interface WordcraftCapabilityDto {
  status: 'available' | 'unsupported' | 'not-installed' | 'error';
  cli: WordcraftResolvedCommandDto | null;
  app: WordcraftResolvedCommandDto | null;
  version: string | null;
  message: string | null;
  remediation: string | null;
  checkedAtEpochMs: number;
}

/** Matches desktop `WordcraftOpenDocumentResponse` (camelCase serde). */
export interface WordcraftOpenDocumentResponseDto {
  appPath: string;
  controlHost: string;
  controlPort: number;
  profileDir: string;
}

export interface WordcraftOpenDocumentRequestDto {
  /** Workspace-relative or absolute local path; omit for the start screen. */
  path?: string | null;
  /** Workspace used for relative-path resolution; defaults to the active one. */
  workspaceId?: string | null;
}

export class WordcraftAPI {
  async status(recheck = false): Promise<WordcraftCapabilityDto> {
    return api.invoke<WordcraftCapabilityDto>('wordcraft_status_command', { recheck });
  }

  async openDocument(
    request: WordcraftOpenDocumentRequestDto
  ): Promise<WordcraftOpenDocumentResponseDto> {
    return api.invoke<WordcraftOpenDocumentResponseDto>('wordcraft_open_document_command', {
      request: {
        path: request.path ?? undefined,
        workspaceId: request.workspaceId ?? undefined,
      },
    });
  }

  /** Best-effort status read for entry-point availability; failures stay quiet here. */
  async statusQuiet(): Promise<WordcraftCapabilityDto | null> {
    try {
      return await this.status(false);
    } catch (error) {
      log.warn('WordCraft status probe is unavailable on this surface', {
        error: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  }
}

export const wordcraftAPI = new WordcraftAPI();
