import { api } from './ApiClient';

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

export interface RagSearchHit {
  chunk: string;
  sourceDoc: string;
  score: number;
  entityIds: string[];
}

export interface RagSearchRequest {
  query: string;
  topK?: number;
}

class BasemindAPI {
  /**
   * Reveal the original value of a redacted entity.
   * UI-only operation - never exposed to the agent.
   */
  async revealEntity(entityId: string, workspaceId?: string): Promise<string> {
    return api.invoke<string>('basemind_reveal_entity', {
      request: { workspaceId, entityId },
    });
  }

  /**
   * Get the redacted version of a document with entities.
   */
  async readRedactedDocument(filePath: string, workspaceId?: string): Promise<RedactedDocumentResponse> {
    return api.invoke<RedactedDocumentResponse>('basemind_read_redacted_document', {
      request: { workspaceId, filePath },
    });
  }

  /**
   * Perform a RAG search on the workspace.
   */
  async ragSearch(request: { query: string; topK?: number }): Promise<Array<{
    chunk: string;
    sourceDoc: string;
    score: number;
    entityIds: string[];
  }>> {
    return api.invoke<Array<{
      chunk: string;
      sourceDoc: string;
      score: number;
      entityIds: string[];
    }>>('rag_search', {
      query: 'query',
      top_k: 8,
    });
  }
}

export const basemindAPI = new BasemindAPI();