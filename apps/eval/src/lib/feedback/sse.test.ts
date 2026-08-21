/* eslint-disable unicorn/no-return-array-push -- the parser's push() returns parsed events */
import { describe, expect, it } from 'vitest';
import { PROJECTED_KINDS } from './frames.ts';
import { CONSUMED_EVENTS } from './live.ts';
import { createSseParser, EVENT_NAMES, projectEvent, serializeEvent } from './sse.ts';

describe('sse parser', () => {
  it('프레임을 id·event·data로 파싱한다', () => {
    const p = createSseParser();
    const events = p.push('id: 3\nevent: step.started\ndata: {"context":{"step":"research-0"}}\n\n');
    expect(events).toEqual([{ id: 3, event: 'step.started', data: '{"context":{"step":"research-0"}}' }]);
  });

  it('청크 경계에 걸친 프레임을 이어 붙인다', () => {
    const p = createSseParser();
    expect(p.push('id: 1\nevent: run.st')).toEqual([]);
    expect(p.push('arted\ndata: {"run":1}\n\n')).toEqual([{ id: 1, event: 'run.started', data: '{"run":1}' }]);
  });

  it('한 번의 push에 담긴 프레임 두 개를 순서대로 낸다', () => {
    const p = createSseParser();
    expect(p.push('event: a\ndata: 1\n\nevent: b\ndata: 2\n\n')).toEqual([
      { id: null, event: 'a', data: '1' },
      { id: null, event: 'b', data: '2' },
    ]);
  });

  it('주석 프레임은 이벤트를 내지 않는다', () => {
    const p = createSseParser();
    expect(p.push(': hb\n\n')).toEqual([]);
  });

  it('id 없는 프레임은 id null로 통과시킨다', () => {
    const p = createSseParser();
    expect(p.push('event: turn.delta\ndata: {}\n\n')).toEqual([{ id: null, event: 'turn.delta', data: '{}' }]);
  });
});

// EventSource는 구독한 이름만 리스너에 흘린다 — 리듀서가 소화하는 kind가 구독 목록이나 릴레이 사영에서 빠지면 프레임이
// 아예 닿지 않아 화면만 조용히 서지 않는다(리듀서 테스트는 전부 통과한 채로).
describe('구독 목록', () => {
  it('리듀서가 소화하는 kind를 모두 구독하고 사영한다', () => {
    expect(EVENT_NAMES).toEqual(expect.arrayContaining([...CONSUMED_EVENTS]));
    expect(PROJECTED_KINDS).toEqual(expect.arrayContaining([...CONSUMED_EVENTS]));
  });

  it('흐르는 턴 조각의 유통기한(turn.started)도 구독한다', () => {
    expect(EVENT_NAMES).toContain('turn.started');
  });
});

describe('projectEvent', () => {
  const envelope = (kind: string, context: object, data: object) =>
    JSON.stringify({ seq: 5, kind, occurredAt: 1000, loggedAt: 1001, context, data });

  it('로그 이벤트의 봉투를 화면 형태로 줄인다', () => {
    const event = {
      id: 5,
      event: 'turn.completed',
      data: envelope(
        'turn.completed',
        { agent: { id: 'a', name: 'n' }, run: 1, turn: 2, attempt: 1 },
        { text: '말', thinking: '긴 생각', raw: {}, usage: null },
      ),
    };
    const projected = projectEvent(event);
    expect(projected?.id).toBe(5);
    expect(projected?.event).toBe('turn.completed');
    expect(JSON.parse(projected?.data ?? '')).toEqual({
      seq: 5,
      kind: 'turn.completed',
      occurredAt: 1000,
      context: { agent: { id: 'a', name: 'n' }, run: 1, turn: 2, attempt: 1 },
      data: { text: '말', usage: null },
    });
  });

  it('사영 밖 kind·깨진 봉투는 null이다', () => {
    expect(
      projectEvent({ id: 5, event: 'run.started', data: envelope('run.started', { agent: { id: 'a', name: 'n' }, run: 1 }, {}) }),
    ).toBeNull();
    expect(projectEvent({ id: 5, event: 'step.started', data: 'not json' })).toBeNull();
  });

  it('id 없는 프로토콜 프레임은 그대로 통과한다', () => {
    const delta = { id: null, event: 'turn.delta', data: '{"channel":"text"}' };
    expect(projectEvent(delta)).toBe(delta);
    const sync = { id: null, event: 'sync', data: '{"seq":9}' };
    expect(projectEvent(sync)).toBe(sync);
  });
});

describe('serializeEvent', () => {
  it('id 유무에 따라 프레임을 조립하고 파서와 왕복한다', () => {
    const withId = { id: 7, event: 'step.started', data: '{"a":1}' };
    const withoutId = { id: null, event: 'heartbeat', data: '{}' };
    expect(serializeEvent(withId)).toBe('id: 7\nevent: step.started\ndata: {"a":1}\n\n');
    expect(serializeEvent(withoutId)).toBe('event: heartbeat\ndata: {}\n\n');
    expect(createSseParser().push(serializeEvent(withId) + serializeEvent(withoutId))).toEqual([withId, withoutId]);
  });
});
