import { describe, expect, it } from 'vitest';
import { parseDocumentIds, planPersonalIntake } from './personal-corpus.ts';

const body = '가'.repeat(300);

describe('parseDocumentIds', () => {
  it('공백·쉼표·줄바꿈을 모두 구분자로 본다', () => {
    expect(parseDocumentIds('A1, A2\nA3  A4')).toEqual(['A1', 'A2', 'A3', 'A4']);
  });

  it('중복은 한 번만 남는다', () => {
    expect(parseDocumentIds('A1 A1 A2')).toEqual(['A1', 'A2']);
  });

  it('빈 입력은 빈 배열', () => {
    expect(parseDocumentIds('  \n ')).toEqual([]);
  });
});

describe('planPersonalIntake', () => {
  const base = { requestedIds: ['A1'], publicIds: ['A1'], existingRefIds: [], extracted: [{ documentId: 'A1', prose: body }] };

  it('공개·추출 성공·길이 충족이면 받아들인다', () => {
    const result = planPersonalIntake(base);
    expect(result.accepted).toEqual([{ refId: 'A1', prose: body, characterCount: 300 }]);
    expect(result.rejected).toEqual([]);
  });

  // 공개 조건은 표집 경로와 같은 관문이다 — 여기서 새면 비공개 글이 평가 시스템에 들어온다.
  it('공개 목록에 없으면 거절한다', () => {
    const result = planPersonalIntake({ ...base, publicIds: [] });
    expect(result.accepted).toEqual([]);
    expect(result.rejected).toEqual([{ refId: 'A1', reason: '공개 상태가 아니거나 존재하지 않는 글입니다' }]);
  });

  it('프로즈 추출에 실패하면 거절한다', () => {
    const result = planPersonalIntake({ ...base, extracted: [{ documentId: 'A1', prose: null }] });
    expect(result.rejected).toEqual([{ refId: 'A1', reason: '본문을 추출하지 못했습니다' }]);
  });

  it('너무 짧으면 자수를 붙여 거절한다', () => {
    const result = planPersonalIntake({ ...base, extracted: [{ documentId: 'A1', prose: '짧다' }] });
    expect(result.rejected).toEqual([{ refId: 'A1', reason: '본문이 너무 짧습니다 (2자)' }]);
  });

  it('이미 들여온 글은 다시 넣지 않는다', () => {
    const result = planPersonalIntake({ ...base, existingRefIds: ['A1'] });
    expect(result.accepted).toEqual([]);
    expect(result.rejected).toEqual([{ refId: 'A1', reason: '이미 들여온 글입니다' }]);
  });

  // 이모지 150개는 UTF-16 길이로는 300이라 .length로 세면 최소 길이를 통과해 버린다.
  it('자수는 코드 포인트로 센다', () => {
    const result = planPersonalIntake({ ...base, extracted: [{ documentId: 'A1', prose: '🙂'.repeat(150) }] });
    expect(result.accepted).toEqual([]);
    expect(result.rejected).toEqual([{ refId: 'A1', reason: '본문이 너무 짧습니다 (150자)' }]);

    const longer = planPersonalIntake({ ...base, extracted: [{ documentId: 'A1', prose: '🙂'.repeat(250) }] });
    expect(longer.accepted[0].characterCount).toBe(250);
  });

  it('받아들인 것과 거절한 것이 섞여도 각각 모인다', () => {
    const result = planPersonalIntake({
      requestedIds: ['A1', 'A2'],
      publicIds: ['A1'],
      existingRefIds: [],
      extracted: [{ documentId: 'A1', prose: body }],
    });
    expect(result.accepted.map((a) => a.refId)).toEqual(['A1']);
    expect(result.rejected.map((r) => r.refId)).toEqual(['A2']);
  });
});
