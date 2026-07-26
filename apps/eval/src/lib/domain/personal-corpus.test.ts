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
  const base = { requestedIds: ['A1'], existingRefIds: [], extracted: [{ documentId: 'A1', prose: body }] };

  // 표집 코퍼스는 공개 글만 받지만 이 경로는 다르다 — 본인이 자기 글을 들이는 자리다.
  // 판정 기준은 본문을 뽑았는지와 길이뿐이며, 공개 여부는 입력에 아예 없다.
  it('본문을 뽑았고 길이가 충족되면 받아들인다', () => {
    const result = planPersonalIntake(base);
    expect(result.accepted).toEqual([{ refId: 'A1', prose: body, characterCount: 300 }]);
    expect(result.rejected).toEqual([]);
  });

  it('없는 문서는 추출 실패로 거절한다', () => {
    const result = planPersonalIntake({ ...base, extracted: [] });
    expect(result.rejected).toEqual([{ refId: 'A1', reason: '없는 문서이거나 본문을 추출하지 못했습니다' }]);
  });

  it('프로즈 추출에 실패하면 거절한다', () => {
    const result = planPersonalIntake({ ...base, extracted: [{ documentId: 'A1', prose: null }] });
    expect(result.rejected).toEqual([{ refId: 'A1', reason: '없는 문서이거나 본문을 추출하지 못했습니다' }]);
  });

  it('너무 짧으면 자수를 붙여 거절한다', () => {
    const result = planPersonalIntake({ ...base, extracted: [{ documentId: 'A1', prose: '짧다' }] });
    expect(result.rejected).toEqual([{ refId: 'A1', reason: '본문이 너무 짧습니다 (2자)' }]);
  });

  // 같은 글을 다른 프롬프트 세트로 다시 돌려 견주는 것이 이 기능의 주된 쓰임이다 —
  // 중복을 거절하면 그 길이 막힌다.
  it('이미 들여온 글은 새로 넣지 않고 재사용으로 넘긴다', () => {
    const result = planPersonalIntake({ ...base, existingRefIds: ['A1'] });
    expect(result.accepted).toEqual([]);
    expect(result.reused).toEqual(['A1']);
    expect(result.rejected).toEqual([]);
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
      existingRefIds: [],
      extracted: [{ documentId: 'A1', prose: body }],
    });
    expect(result.accepted.map((a) => a.refId)).toEqual(['A1']);
    expect(result.rejected.map((r) => r.refId)).toEqual(['A2']);
  });
});
