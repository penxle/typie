import { describe, expect, it } from 'vitest';
import { collectEvents, threadId, threadsFromResult } from './project.ts';
import type { FeedbackResult } from '../feedback/types.ts';

const result: FeedbackResult = {
  version: 1,
  issues: [
    { axis: '인물의 동기', pass: 'critique', body: '지적 문면', anchors: [{ start: 10, end: 20, head: '머리', tail: '꼬리' }] },
    { axis: '문장 결', pass: 'proofread', body: null, anchors: [] },
  ],
  conclusion: { understanding: null, strengths: [], clearances: [], patterns: [], priorities: [] },
};

describe('threadsFromResult', () => {
  it('issue 인덱스를 보존하며 thread 행으로 정규화한다', () => {
    const rows = threadsFromResult('s1', 1, result);
    expect(rows).toEqual([
      {
        id: threadId('s1', 1, 0),
        sessionId: 's1',
        reviewRound: 1,
        issueIndex: 0,
        axis: '인물의 동기',
        pass: 'critique',
        body: '지적 문면',
        anchors: [{ start: 10, end: 20, head: '머리', tail: '꼬리' }],
        state: 'open',
        stateChangedAt: null,
      },
      expect.objectContaining({ id: threadId('s1', 1, 1), issueIndex: 1, body: null }),
    ]);
  });

  it('threadId는 결정적이다', () => {
    expect(threadId('s1', 1, 0)).toBe(threadId('s1', 1, 0));
    expect(threadId('s1', 1, 0)).not.toBe(threadId('s1', 2, 0));
  });
});

describe('collectEvents', () => {
  it('재생 스트림을 EOF까지 소비해 id 프레임만 수집한다', async () => {
    const body = new ReadableStream<Uint8Array>({
      start(c) {
        c.enqueue(new TextEncoder().encode('id: 1\nevent: run.started\ndata: {"run":1}\n\n: hb\n\nevent: turn.delta\ndata: {}\n\n'));
        c.enqueue(new TextEncoder().encode('id: 2\nevent: run.completed\ndata: {"run":1}\n\n'));
        c.close();
      },
    });
    const env = { PRISM_API_ORIGIN: 'x', PRISM_API_TOKEN: 'x' };
    const events = await collectEvents(env, 'ev-x', () => Promise.resolve(new Response(body)));
    expect(events.map((e) => e.event)).toEqual(['run.started', 'run.completed']);
  });
});
