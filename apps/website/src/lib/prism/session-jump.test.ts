import { beforeEach, describe, expect, it } from 'vitest';
import { createTrackedEffect } from '../editor-ffi/editor-effect-harness.svelte';
import { requestSessionJump, takeSessionJump } from './session-jump.svelte.ts';

beforeEach(() => {
  requestSessionJump('session-0');
  takeSessionJump();
});

describe('session-jump', () => {
  it('요청이 없으면 가져갈 것도 없다', () => {
    expect(takeSessionJump()).toBeNull();
  });

  it('한 번 가져가면 비워진다', () => {
    requestSessionJump('session-1');

    expect(takeSessionJump()).toBe('session-1');
    expect(takeSessionJump()).toBeNull();
  });

  it('나중 요청이 앞 요청을 덮는다', () => {
    requestSessionJump('session-1');
    requestSessionJump('session-2');

    expect(takeSessionJump()).toBe('session-2');
  });

  // 패널은 늘 마운트되어 있다 — 큐가 반응성이 없으면 소비 효과가 영영 다시 돌지 않는다
  it('새 요청이 오면 가져가는 쪽이 다시 돈다', async () => {
    const seen: (string | null)[] = [];
    const tracked = createTrackedEffect(() => {
      seen.push(takeSessionJump());
    });

    try {
      await tracked.flush();
      expect(tracked.runs()).toBe(1);
      expect(seen).toEqual([null]);

      requestSessionJump('session-1');
      await tracked.flush();

      expect(tracked.runs()).toBeGreaterThan(1);
      expect(seen).toContainEqual('session-1');
    } finally {
      tracked.destroy();
    }
  });
});
