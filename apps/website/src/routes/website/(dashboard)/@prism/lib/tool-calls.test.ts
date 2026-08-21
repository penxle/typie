import { describe, expect, it } from 'vitest';
import { collapseRows, foldToolCalls } from './tool-calls.ts';
import type { TranscriptMessage } from './conversation.ts';

const tool = (key: string, name: string): TranscriptMessage => ({ role: 'tool', key, name, phase: 'executed', ok: true, at: 0 });
const user = (key: string): TranscriptMessage => ({ role: 'user', key, text: 'a', at: 0 });

const foldable = (message: TranscriptMessage) => message.role === 'tool';
const labelOf = (message: TranscriptMessage) => (message.role === 'tool' && message.name !== 'skip' ? message.name : null);

describe('collapseRows', () => {
  it('잇달아 같은 라벨만 한 줄로 접고 끊긴 재등장은 새 줄이다', () => {
    expect(collapseRows(['a', 'a', 'b', 'a'])).toEqual([
      { label: 'a', count: 2 },
      { label: 'b', count: 1 },
      { label: 'a', count: 1 },
    ]);
  });

  it('빈 목록은 빈 줄 목록', () => {
    expect(collapseRows([])).toEqual([]);
  });
});

describe('foldToolCalls', () => {
  it('연속한 접이 대상은 첫 메시지 key로 묶이고 그 외 메시지는 제자리에 남는다', () => {
    const entries = foldToolCalls([user('u1'), tool('t1', 'read'), tool('t2', 'read'), user('u2')], foldable, labelOf);
    expect(entries).toEqual([
      user('u1'),
      { role: 'tool-calls', key: 'tools:t1', count: 2, rows: [{ label: 'read', count: 2 }] },
      user('u2'),
    ]);
  });

  it('접이 대상 사이에 다른 메시지가 끼면 묶음이 갈린다', () => {
    const entries = foldToolCalls([tool('t1', 'read'), user('u1'), tool('t2', 'grep')], foldable, labelOf);
    expect(entries).toMatchObject([
      { role: 'tool-calls', key: 'tools:t1', count: 1 },
      { role: 'user', key: 'u1' },
      { role: 'tool-calls', key: 'tools:t2', count: 1, rows: [{ label: 'grep', count: 1 }] },
    ]);
  });

  it('라벨이 null인 접이 대상은 세지 않고 버린다', () => {
    const entries = foldToolCalls([tool('t1', 'skip'), tool('t2', 'read'), tool('t3', 'skip')], foldable, labelOf);
    expect(entries).toEqual([{ role: 'tool-calls', key: 'tools:t2', count: 1, rows: [{ label: 'read', count: 1 }] }]);
  });

  it('전부 라벨이 null이면 아무 묶음도 만들지 않는다', () => {
    expect(foldToolCalls([tool('t1', 'skip')], foldable, labelOf)).toEqual([]);
  });

  it('꼬리의 묶음도 흘려보낸다', () => {
    const entries = foldToolCalls([user('u1'), tool('t1', 'read'), tool('t2', 'grep')], foldable, labelOf);
    expect(entries.at(-1)).toEqual({
      role: 'tool-calls',
      key: 'tools:t1',
      count: 2,
      rows: [
        { label: 'read', count: 1 },
        { label: 'grep', count: 1 },
      ],
    });
  });
});
