import { beforeEach, describe, expect, it } from 'vitest';
import { createTrackedEffect } from '../editor-ffi/editor-effect-harness.svelte';
import { requestMarginJump, takeMarginJump } from './margin-jump.svelte.ts';
import type { MarginJump } from './margin-jump.svelte.ts';

const make = (over: Partial<MarginJump> = {}): MarginJump => ({
  documentId: 'document-1',
  roundId: 'round-1',
  itemId: 'thread-1',
  ...over,
});

beforeEach(() => {
  requestMarginJump(make());
  takeMarginJump('document-1');
});

describe('margin-jump', () => {
  it('요청이 없으면 가져갈 것도 없다', () => {
    expect(takeMarginJump('document-1')).toBeNull();
  });

  it('다른 문서의 여백은 가져가지 못한다', () => {
    const jump = make();
    requestMarginJump(jump);

    expect(takeMarginJump('document-2')).toBeNull();
    expect(takeMarginJump(jump.documentId)).toEqual(jump);
  });

  it('문서가 아직 열리지 않은 여백은 가져가지 못한다', () => {
    const jump = make();
    requestMarginJump(jump);

    expect(takeMarginJump(null)).toBeNull();
    expect(takeMarginJump(jump.documentId)).toEqual(jump);
  });

  it('한 번 가져가면 비워진다', () => {
    const jump = make();
    requestMarginJump(jump);

    expect(takeMarginJump(jump.documentId)).toEqual(jump);
    expect(takeMarginJump(jump.documentId)).toBeNull();
  });

  it('나중 요청이 앞 요청을 덮는다', () => {
    requestMarginJump(make({ roundId: 'round-1' }));
    requestMarginJump(make({ roundId: 'round-2', itemId: 'strength:0' }));

    expect(takeMarginJump('document-1')).toEqual(make({ roundId: 'round-2', itemId: 'strength:0' }));
  });

  // 이미 열려 있는 문서를 누르면 여백의 documentId는 그대로다 — 큐가 반응성이 없으면 소비 효과가 영영 다시 돌지 않는다
  it('문서가 그대로여도 새 요청이 오면 가져가는 쪽이 다시 돈다', async () => {
    const seen: (MarginJump | null)[] = [];
    const tracked = createTrackedEffect(() => {
      seen.push(takeMarginJump('document-1'));
    });

    try {
      await tracked.flush();
      expect(tracked.runs()).toBe(1);
      expect(seen).toEqual([null]);

      const jump = make();
      requestMarginJump(jump);
      await tracked.flush();

      expect(tracked.runs()).toBeGreaterThan(1);
      expect(seen).toContainEqual(jump);
    } finally {
      tracked.destroy();
    }
  });
});
