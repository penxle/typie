import { describe, expect, it, vi } from 'vitest';
import { OpenDocumentRegistry } from './open-documents.svelte.ts';

describe('OpenDocumentRegistry', () => {
  it('페인별 등록을 문서 id로 합치고, 제거하면 빠진다', () => {
    const r = new OpenDocumentRegistry();
    r.setExpectedPanes(['p1', 'p2', 'p3']);
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
    r.setExpectedPanes(['p1']);
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

  it('같은 페인을 다시 분류하면 값이 대체되고 항목은 하나다', () => {
    const r = new OpenDocumentRegistry();
    r.setExpectedPanes(['p1']);
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

    r.expectPane('p1');
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
    r.setExpectedPanes(['p1', 'p2']);
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
    r.expectPane('p1');
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

  it('모든 엔티티 페인의 분류가 끝날 때까지 스냅샷을 기다린다', async () => {
    const r = new OpenDocumentRegistry();
    r.setExpectedPanes(['p1', 'p2']);

    const pending = r.snapshotWhenReady();
    r.upsert('p1', {
      kind: 'document',
      documentId: 'D1',
      entityId: 'E1',
      title: '가',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: true,
    });

    let settled = false;
    void pending.then(() => (settled = true));
    await Promise.resolve();
    expect(settled).toBe(false);

    r.resolvePane('p2');
    await expect(pending).resolves.toEqual({
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
      ],
    });
  });

  it('동기화되지 않았거나 미분류 페인이 남으면 2초 뒤 거부한다', async () => {
    vi.useFakeTimers();

    try {
      const unsynchronized = new OpenDocumentRegistry();
      const unsynchronizedResult = expect(unsynchronized.snapshotWhenReady()).rejects.toThrow();
      await vi.advanceTimersByTimeAsync(2000);
      await unsynchronizedResult;

      const unresolved = new OpenDocumentRegistry();
      unresolved.setExpectedPanes(['p1']);
      const unresolvedResult = expect(unresolved.snapshotWhenReady()).rejects.toThrow();
      await vi.advanceTimersByTimeAsync(2000);
      await unresolvedResult;
    } finally {
      vi.useRealTimers();
    }
  });

  it('기다리던 페인이 예상 목록에서 빠지면 준비된 스냅샷을 반환한다', async () => {
    const r = new OpenDocumentRegistry();
    r.setExpectedPanes(['p1', 'p2']);
    r.resolvePane('p1');

    const pending = r.snapshotWhenReady();
    r.setExpectedPanes(['p1']);

    await expect(pending).resolves.toEqual({ documents: [] });
  });

  it('분류를 다시 시작한 페인은 재분류될 때까지 기다리고 이전 문서를 지운다', async () => {
    const r = new OpenDocumentRegistry();
    r.setExpectedPanes(['p1']);
    r.upsert('p1', {
      kind: 'document',
      documentId: 'D1',
      entityId: 'E1',
      title: '가',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: true,
    });
    await expect(r.snapshotWhenReady()).resolves.toHaveProperty('documents.0.documentId', 'D1');

    r.expectPane('p1');
    const pending = r.snapshotWhenReady();
    r.resolvePane('p1');

    await expect(pending).resolves.toEqual({ documents: [] });
  });

  it('무효화 후에는 이전 문서를 버리고 새 페인 분류를 기다린다', async () => {
    const r = new OpenDocumentRegistry();
    r.setExpectedPanes(['p1']);
    r.upsert('p1', {
      kind: 'document',
      documentId: 'D1',
      entityId: 'E1',
      title: '이전 문서',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: true,
    });
    await expect(r.snapshotWhenReady()).resolves.toHaveProperty('documents.0.documentId', 'D1');

    r.invalidate();
    const pending = r.snapshotWhenReady();
    r.setExpectedPanes(['p1']);

    let settled = false;
    void pending.then(() => (settled = true));
    await Promise.resolve();
    expect(settled).toBe(false);

    r.upsert('p1', {
      kind: 'document',
      documentId: 'D2',
      entityId: 'E2',
      title: '복원된 문서',
      subtitle: null,
      icon: 'file',
      iconColor: 'gray',
      active: true,
    });
    await expect(pending).resolves.toHaveProperty('documents.0.documentId', 'D2');
  });
});
