import { api } from './ApiClient';
import {
  BasemindUnavailableError,
  getBasemindSurfaceState,
  isUnknownBasemindCommandError,
  markBasemindSurfaceAvailable,
  markBasemindSurfaceUnavailable,
} from '@/infrastructure/basemind/basemindSurface';

export interface RevealEntityRequest {
  workspaceId?: string;
  entityId: string;
}

export interface RedactedEntity {
  entityId: string;
  entityType: string;
  occurrenceCount: number;
}

export interface RedactedDocumentResponse {
  markdown: string;
  entities: RedactedEntity[];
  sourceFormat: string;
}

const REVEAL_COMMAND = 'basemind_reveal_entity';
const READ_COMMAND = 'basemind_read_redacted_document';

class BasemindAPI {
  /**
   * Reveal the original value of a redacted entity.
   * UI-only operation - never exposed to the agent.
   */
  async revealEntity(entityId: string, workspaceId?: string): Promise<string> {
    return this.invoke<string>(REVEAL_COMMAND, { workspaceId, entityId });
  }

  /**
   * Get the redacted version of a document with entities.
   */
  async readRedactedDocument(filePath: string, workspaceId?: string): Promise<RedactedDocumentResponse> {
    return this.invoke<RedactedDocumentResponse>(READ_COMMAND, { workspaceId, filePath });
  }

  /**
   * Whether the active host implements the Basemind redaction commands.
   *
   * The probe reveals an empty entity id: it resolves no value, and a host
   * without the command answers with an unknown-command error instead. The
   * answer is cached as the surface state, so the probe runs at most once per
   * host.
   */
  async probeSurface(): Promise<boolean> {
    const state = getBasemindSurfaceState();
    if (state !== 'unknown') {
      return state === 'available';
    }
    try {
      await api.invoke(REVEAL_COMMAND, { request: { entityId: '' } });
      markBasemindSurfaceAvailable();
      return true;
    } catch (error) {
      if (isUnknownBasemindCommandError(error, REVEAL_COMMAND)) {
        markBasemindSurfaceUnavailable();
        return false;
      }
      // The command exists; only the probe target was rejected.
      markBasemindSurfaceAvailable();
      return true;
    }
  }

  private async invoke<T>(command: string, request: Record<string, unknown>): Promise<T> {
    try {
      const result = await api.invoke<T>(command, { request });
      markBasemindSurfaceAvailable();
      return result;
    } catch (error) {
      if (isUnknownBasemindCommandError(error, command)) {
        markBasemindSurfaceUnavailable();
        throw new BasemindUnavailableError(command);
      }
      throw error;
    }
  }
}

export const basemindAPI = new BasemindAPI();
