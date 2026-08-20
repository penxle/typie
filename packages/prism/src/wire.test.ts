import { describe, expect, it } from 'vitest';
import { StreamFrameSchema } from './wire.ts';

describe('StreamFrameSchema', () => {
  it('프레임 4종을 판별한다', () => {
    expect(StreamFrameSchema.parse({ event: 'heartbeat', data: '{}' })).toEqual({ type: 'heartbeat' });
    expect(StreamFrameSchema.parse({ event: 'sync', data: '{"seq":9}' })).toEqual({ type: 'sync', seq: 9 });

    const delta = StreamFrameSchema.parse({
      event: 'turn.delta',
      data: JSON.stringify({
        context: { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 },
        channel: 'text',
        offset: 0,
        data: '안',
      }),
    });
    expect(delta).toEqual({
      type: 'delta',
      delta: { context: { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 }, channel: 'text', offset: 0, data: '안' },
    });

    const frame = StreamFrameSchema.parse({
      event: 'run.started',
      data: JSON.stringify({
        seq: 2,
        kind: 'run.started',
        occurredAt: 5,
        loggedAt: 5,
        context: { agent: { id: 'a', name: 'x' }, run: 1 },
        data: { message: '안녕' },
      }),
    });
    expect(frame).toEqual({
      type: 'event',
      event: { seq: 2, kind: 'run.started', occurredAt: 5, context: { agent: { id: 'a', name: 'x' }, run: 1 }, data: { message: '안녕' } },
    });
  });

  it('깨진 JSON·봉투 결손·tool 없는 tool.input 델타를 거부한다', () => {
    expect(StreamFrameSchema.safeParse({ event: 'run.started', data: '{broken' }).success).toBe(false);
    expect(StreamFrameSchema.safeParse({ event: 'run.started', data: '{"kind":"run.started"}' }).success).toBe(false);
    expect(
      StreamFrameSchema.safeParse({
        event: 'turn.delta',
        data: JSON.stringify({
          context: { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 },
          channel: 'tool.input',
          offset: 0,
          data: '{',
        }),
      }).success,
    ).toBe(false);
  });
});
