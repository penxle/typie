import { describe, expect, it } from 'vitest';
import { isJudgmentComplete, targetFor } from '../../../core/evaluation.ts';
import { TRIAXIAL } from './triaxial.ts';

const finding = { id: 'i1', kind: 'finding', facets: { layer: 'plan' } };
const characterization = { id: 'c1', kind: 'characterization', facets: {} };
const strength = { id: 's1', kind: 'strength', facets: {} };
const cleared = { id: 'x1', kind: 'cleared', facets: {} };
const pattern = { id: 'p1', kind: 'pattern', facets: {} };
const priority = { id: 'q1', kind: 'priority', facets: {} };

const [judgment, artifacts] = TRIAXIAL.stages;
const runOk = { sourceFamiliarity: 'known', priorityUseful: true, consistent: true, helpfulness: 4 };

describe('TRIAXIAL', () => {
  it('두 단계로 구성된다', () => {
    expect(TRIAXIAL.stages.map((s) => s.key)).toEqual(['judgment', 'artifacts']);
  });

  it('지적·파악·강점·무혐의에 컨트롤을 걸고 패턴·우선순위에는 걸지 않는다', () => {
    expect(targetFor(TRIAXIAL, finding)).not.toBeNull();
    expect(targetFor(TRIAXIAL, characterization)).not.toBeNull();
    expect(targetFor(TRIAXIAL, strength)).not.toBeNull();
    expect(targetFor(TRIAXIAL, cleared)).not.toBeNull();
    expect(targetFor(TRIAXIAL, pattern)).toBeNull();
    expect(targetFor(TRIAXIAL, priority)).toBeNull();
  });

  it('지적 세 축이 다 차야 완결이고 사유 분류는 완결 조건이 아니다', () => {
    expect(isJudgmentComplete(judgment, [finding], runOk, { i1: { correct: true, needed: true } })).toBe(false);
    expect(isJudgmentComplete(judgment, [finding], runOk, { i1: { correct: true, needed: true, useful: false } })).toBe(true);
  });

  it('파악·무혐의는 판단불가로도 완결된다', () => {
    const answers = { c1: { readCorrectly: 'unknown' }, x1: { clear: 'unknown' } };
    expect(isJudgmentComplete(judgment, [characterization, cleared], runOk, answers)).toBe(true);
    expect(isJudgmentComplete(judgment, [characterization, cleared], runOk, { c1: { readCorrectly: 'unknown' } })).toBe(false);
  });

  it('강점은 동의 답이 있어야 완결이다', () => {
    expect(isJudgmentComplete(judgment, [strength], runOk, {})).toBe(false);
    expect(isJudgmentComplete(judgment, [strength], runOk, { s1: { agree: false } })).toBe(true);
  });

  it('원작 신고·순서·일관·도움도가 다 차야 첫 단계가 완결된다', () => {
    expect(isJudgmentComplete(judgment, [], runOk, {})).toBe(true);
    expect(isJudgmentComplete(judgment, [], { ...runOk, sourceFamiliarity: undefined }, {})).toBe(false);
    expect(isJudgmentComplete(judgment, [], { ...runOk, consistent: undefined }, {})).toBe(false);
    expect(isJudgmentComplete(judgment, [], { ...runOk, sourceFamiliarity: 'junk' }, {})).toBe(false);
  });

  it('둘째 단계는 리서치·계획·신뢰가 차야 완결이고 사유·수정 희망은 아니다', () => {
    expect(isJudgmentComplete(artifacts, [], {}, {})).toBe(false);
    expect(isJudgmentComplete(artifacts, [], { researchAccurate: 'unknown', planApt: true, trustChange: 'same' }, {})).toBe(true);
    expect(isJudgmentComplete(artifacts, [], { researchAccurate: true, planApt: true, trustChange: 'whatever' }, {})).toBe(false);
  });

  it('둘째 단계에는 항목 판정이 없다 — 첫 단계 항목은 둘째 단계의 완결 조건이 아니다', () => {
    expect(isJudgmentComplete(artifacts, [finding, strength], { researchAccurate: true, planApt: true, trustChange: 'more' }, {})).toBe(
      true,
    );
  });

  it('run 필드 키는 전 단계에 걸쳐 유일하다 — 단계들이 한 페이로드를 공유한다', () => {
    const keys = TRIAXIAL.stages.flatMap((s) => s.run.map((f) => f.key));
    expect(new Set(keys).size).toBe(keys.length);
  });

  it('사유 분류는 선언된 값만 통과한다 — 복수 선택이고, 홑 문자열 답도 배열로 읽는다', () => {
    const field = targetFor(TRIAXIAL, finding)?.fields.find((f) => f.key === 'reasonKind');
    expect(field?.sanitize(['convention', 'taste'])).toEqual(['convention', 'taste']);
    expect(field?.sanitize(['convention', 'junk', 'convention'])).toEqual(['convention']);
    expect(field?.sanitize('convention')).toEqual(['convention']);
    expect(field?.sanitize(['junk'])).toBeNull();
    expect(field?.sanitize('junk')).toBeNull();
  });
});
