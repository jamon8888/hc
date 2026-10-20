 

import type { Language, FileDetectionResult, DetectionContext } from '../types';
import { languageDetector } from '../core/LanguageDetector';

 
export function detectLanguage(
  filePathOrName: string,
  context?: Partial<DetectionContext>
): FileDetectionResult {
  return languageDetector.detectFile(filePathOrName, context);
}

 
export function getMonacoLanguage(filePathOrName: string): string {
  return languageDetector.getMonacoLanguage(filePathOrName);
}

 
export function getFileIconType(filePathOrName: string): string {
  return languageDetector.getIconType(filePathOrName);
}

 
export function getFileColor(filePathOrName: string): string | undefined {
  return languageDetector.getColor(filePathOrName);
}

 
export function getLanguageById(languageId: string): Language | undefined {
  return languageDetector.getLanguage(languageId);
}

 
export function isImageFile(filePathOrName: string): boolean {
  const result = detectLanguage(filePathOrName);
  return result.language.iconType === 'image';
}

 
export function isCodeFile(filePathOrName: string): boolean {
  const result = detectLanguage(filePathOrName);
  return result.language.category === 'programming';
}

 
export function isConfigFile(filePathOrName: string): boolean {
  const result = detectLanguage(filePathOrName);
  return result.language.category === 'config' || result.language.category === 'data';
}

 
export function isDocumentationFile(filePathOrName: string): boolean {
  const result = detectLanguage(filePathOrName);
  return result.language.category === 'documentation';
}

 
export function getCommentPrefix(languageId: string): string | undefined {
  const lang = languageDetector.getLanguage(languageId);
  return lang?.lineCommentPrefix;
}

 
export function getPrismLanguage(filePathOrName: string): string {
  const result = detectLanguage(filePathOrName);
  const lang = result.language;
  
  
  if (lang.prismId) {
    return lang.prismId;
  }
  
  
  const prismFallbackMap: Record<string, string> = {
    'csharp': 'csharp',
    'cpp': 'cpp',
    'c': 'c',
    'plaintext': 'text',
    'shell': 'bash',
    'bat': 'batch',
    'dockerfile': 'docker',
  };
  
  return prismFallbackMap[lang.monacoId] || lang.monacoId;
}

 
export function getPrismLanguageFromAlias(alias: string): string {
  if (!alias) return 'text';
  
  const lowerAlias = alias.toLowerCase();
  
  
  const lang = languageDetector.getLanguage(lowerAlias);
  if (lang) {
    return lang.prismId || lang.monacoId;
  }
  
  
  return lowerAlias;
}

 
/**
 * Document formats the Basemind redaction surface can read.
 *
 * `.docx` is absent on purpose: WordCraft owns that format, and the file tab
 * manager routes it to the WordCraft decision first. `.csv` and `.pot` keep
 * their plain code-editor behavior because no redaction contract covers them.
 */
const REDACTED_DOCUMENT_EXTENSIONS = [
  '.doc', '.docm', '.odt', '.rtf', '.epub',
  '.pptx', '.pptm', '.ppsx', '.ppsm', '.ppt', '.pps',
  '.xlsx', '.xlsm', '.xlsb', '.xls', '.ods', '.odp',
];

/** Whether a file name is a document the redacted document viewer can open. */
export function isRedactedDocumentPath(filePathOrName: string): boolean {
  const fileName = filePathOrName.split(/[/\\]/).pop()?.toLowerCase() || '';
  return REDACTED_DOCUMENT_EXTENSIONS.some(ext => fileName.endsWith(ext));
}

export interface EditorTypeOptions {
  /**
   * Whether the active host implements the Basemind redaction surface.
   * Document formats keep their plain editor behavior while it is missing.
   */
  basemindAvailable?: boolean;
}

export function getEditorType(
  filePathOrName: string,
  options: EditorTypeOptions = {},
): 'code-editor' | 'markdown-editor' | 'image-viewer' | 'pdf-viewer' | 'plan-viewer' | 'html-preview' | 'redacted-document-viewer' {
  const result = detectLanguage(filePathOrName);
  const iconType = result.language.iconType;
  
  
  const fileName = filePathOrName.split(/[/\\]/).pop()?.toLowerCase() || '';
  if (fileName.endsWith('.plan.md')) {
    return 'plan-viewer';
  }

  if (fileName.endsWith('.pdf')) {
    return 'pdf-viewer';
  }

  if (fileName.endsWith('.html') || fileName.endsWith('.htm')) {
    return 'html-preview';
  }

  // Office and document formats have no built-in viewer; the redacted
  // document viewer renders them through basemind. PDF keeps the built-in
  // pdf-viewer: it renders locally and does not need document conversion.
  // A host without the Basemind surface keeps the plain editor behavior
  // instead of opening a viewer it cannot serve.
  if (options.basemindAvailable && isRedactedDocumentPath(fileName)) {
    return 'redacted-document-viewer';
  }
  
  
  if (iconType === 'image') {
    return 'image-viewer';
  }
  
  
  if (result.language.id === 'markdown') {
    return 'markdown-editor';
  }
  
  
  return 'code-editor';
}
