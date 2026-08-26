import { describe, expect, it } from 'vitest';
import { OpenDocumentRegistry } from './open-documents.svelte.ts';

describe('OpenDocumentRegistry', () => {
  it('페인별 등록을 문서 id로 합치고, 제거하면 빠진다', () => {
    const r = new OpenDocumentRegistry();
    r.upsert('p1', {
      kind: 'document',
      documentId: 'D1',
      entityId: 'E1',
      title: '가',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: false,
    });
    r.upsert('p2', {
      kind: 'document',
      documentId: 'D1',
      entityId: 'E1',
      title: '가',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: true,
    });
    r.upsert('p3', {
      kind: 'document',
      documentId: 'D2',
      entityId: 'E2',
      title: null,
      subtitle: '부',
      icon: 'file',
      iconColor: 'gray',
      active: false,
    });
    expect(r.snapshot()).toEqual({
      documents: [
        {
          kind: 'document',
          documentId: 'D1',
          entityId: 'E1',
          title: '가',
          subtitle: null,
          icon: 'file',
          iconColor: 'gray',
          active: true,
        },
        {
          kind: 'document',
          documentId: 'D2',
          entityId: 'E2',
          title: null,
          subtitle: '부',
          icon: 'file',
          iconColor: 'gray',
          active: false,
        },
      ],
    });
    r.remove('p2');
    r.remove('p3');
    expect(r.snapshot()).toEqual({
      documents: [
        {
          kind: 'document',
          documentId: 'D1',
          entityId: 'E1',
          title: '가',
          subtitle: null,
          icon: 'file',
          iconColor: 'gray',
          active: false,
        },
      ],
    });
  });

  it('같은 페인 키에 다시 등록하면 값이 대체되고, 제거 후 재등록해도 항목은 하나다', () => {
    const r = new OpenDocumentRegistry();
    r.upsert('p1', {
      kind: 'document',
      documentId: 'D1',
      entityId: 'E1',
      title: '가',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: false,
    });
    r.upsert('p1', {
      kind: 'document',
      documentId: 'D1',
      entityId: 'E1',
      title: '나',
      subtitle: '부',
      icon: 'file',
      iconColor: 'gray',
      active: true,
    });
    expect(r.snapshot()).toEqual({
      documents: [
        {
          kind: 'document',
          documentId: 'D1',
          entityId: 'E1',
          title: '나',
          subtitle: '부',
          icon: 'file',
          iconColor: 'gray',
          active: true,
        },
      ],
    });

    r.remove('p1');
    r.upsert('p1', {
      kind: 'document',
      documentId: 'D2',
      entityId: 'E2',
      title: '다',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: false,
    });
    expect(r.snapshot()).toEqual({
      documents: [
        {
          kind: 'document',
          documentId: 'D2',
          entityId: 'E2',
          title: '다',
          subtitle: null,
          icon: 'file',
          iconColor: 'gray',
          active: false,
        },
      ],
    });
  });

  it('재등록 순서와 무관하게 문서 id 순으로 정렬한다', () => {
    const r = new OpenDocumentRegistry();
    r.upsert('p1', {
      kind: 'document',
      documentId: 'D1',
      entityId: 'E1',
      title: '가',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: false,
    });
    r.upsert('p2', {
      kind: 'document',
      documentId: 'D2',
      entityId: 'E2',
      title: '나',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: false,
    });
    r.remove('p1');
    r.upsert('p1', {
      kind: 'document',
      documentId: 'D1',
      entityId: 'E1',
      title: '가',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: false,
    });

    expect(r.snapshot().documents.map((doc) => doc.documentId)).toEqual(['D1', 'D2']);
  });
});
