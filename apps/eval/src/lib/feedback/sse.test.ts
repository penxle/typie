/* eslint-disable unicorn/no-return-array-push -- the parser's push() returns parsed events */
import { describe, expect, it } from 'vitest';
import { createSseParser } from './sse.ts';

describe('sse parser', () => {
  it('프레임을 id·event·data로 파싱한다', () => {
    const p = createSseParser();
    const events = p.push('id: 3\nevent: step.started\ndata: {"step":"research-0"}\n\n');
    expect(events).toEqual([{ id: 3, event: 'step.started', data: '{"step":"research-0"}' }]);
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

  it('하트비트 주석 프레임은 이벤트를 내지 않는다', () => {
    const p = createSseParser();
    expect(p.push(': hb\n\n')).toEqual([]);
  });

  it('id 없는 프레임은 id null로 통과시킨다', () => {
    const p = createSseParser();
    expect(p.push('event: turn.delta\ndata: {}\n\n')).toEqual([{ id: null, event: 'turn.delta', data: '{}' }]);
  });
});
