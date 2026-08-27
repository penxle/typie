import { describe, expect, it } from 'vitest';
import { saveDocumentView } from './save-document-view.ts';

describe('saveDocumentView', () => {
  it('경로에서 문서 id를 뽑고 summary를 그대로 둔다', () => {
    expect(saveDocumentView({ path: 'documents/D0ABC.xml', summary: '오탈자를 고쳤어요' })).toEqual({
      documentId: 'D0ABC',
      summary: '오탈자를 고쳤어요',
    });
    expect(saveDocumentView({ path: 'manuscript/v1.txt', summary: 'x' })).toBeNull();
    expect(saveDocumentView({ path: 'documents/D0ABC.xml' })).toBeNull();
  });
});
