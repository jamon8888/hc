import { describe, expect, it } from 'vitest';

import { getEditorType, isRedactedDocumentPath } from './helpers';

describe('getEditorType', () => {
  it('routes PDF files to the PDF viewer case-insensitively', () => {
    expect(getEditorType('report.PDF')).toBe('pdf-viewer');
  });

  it('routes HTML files to the embedded preview case-insensitively', () => {
    expect(getEditorType('index.HTML')).toBe('html-preview');
    expect(getEditorType('legacy.HTM')).toBe('html-preview');
  });

  it('keeps the plain editor for formats the redaction surface does not cover', () => {
    expect(getEditorType('rows.csv', { basemindAvailable: true })).toBe('code-editor');
    expect(getEditorType('messages.pot', { basemindAvailable: true })).toBe('code-editor');
  });

  it('keeps the plain editor for document formats while the redaction surface is missing', () => {
    expect(getEditorType('deck.pptx')).toBe('code-editor');
    expect(getEditorType('deck.pptx', { basemindAvailable: false })).toBe('code-editor');
  });

  it('routes document formats to the redacted viewer once the redaction surface is available', () => {
    expect(getEditorType('deck.pptx', { basemindAvailable: true })).toBe('redacted-document-viewer');
    expect(getEditorType('sheet.xlsx', { basemindAvailable: true })).toBe('redacted-document-viewer');
  });

  it('leaves WordCraft documents to the WordCraft decision', () => {
    expect(isRedactedDocumentPath('contract.docx')).toBe(false);
    expect(getEditorType('contract.docx', { basemindAvailable: true })).toBe('code-editor');
  });
});
