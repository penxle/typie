import { describe, expect, it } from 'vitest';
import { spawnPlan } from './run-service.ts';

describe('spawnPlan', () => {
  it('동결된 세대는 거부한다', () => {
    expect(spawnPlan({ status: 'frozen' }, ['d1'])).toEqual({ error: '동결된 세대는 실행할 수 없습니다' });
  });

  it('문서가 없으면 거부한다', () => {
    expect(spawnPlan({ status: 'active' }, [])).toEqual({ error: '실행할 문서가 없습니다' });
  });

  it('동결이 문서 부재보다 먼저 걸린다', () => {
    expect(spawnPlan({ status: 'frozen' }, [])).toEqual({ error: '동결된 세대는 실행할 수 없습니다' });
  });

  it('활성 세대와 문서가 있으면 통과한다', () => {
    expect(spawnPlan({ status: 'active' }, ['d1', 'd2'])).toEqual({ ok: true });
  });
});
