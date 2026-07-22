import { describe, expect, it } from 'vitest';
import { corpusConflict, pickLiterary, selectSuccessfulExtracts, shuffle } from './sampling-select.ts';
import type { Candidate } from './internal-api.ts';

const candidate = (documentId: string): Candidate => ({ documentId, text: `본문 ${documentId}`, characterCount: 100 });

describe('pickLiterary', () => {
  it('literary=true 후보만 남긴다', () => {
    const result = pickLiterary([
      { candidate: candidate('a'), literary: true, kind: '소설' },
      { candidate: candidate('b'), literary: false, kind: '메모' },
      { candidate: candidate('c'), literary: true, kind: '수필' },
    ]);
    expect(result.map((c) => c.documentId)).toEqual(['a', 'c']);
  });
});

describe('shuffle', () => {
  it('요소를 보존하고 주입된 rng로 결정적이다', () => {
    const seq = [0, 0, 0];
    let i = 0;
    const rng = () => seq[i++] ?? 0;
    const input = ['a', 'b', 'c', 'd'];
    const out = shuffle(input, rng);
    expect(new Set(out)).toEqual(new Set(['a', 'b', 'c', 'd']));
    expect(shuffle(input, () => 0)).toEqual(shuffle(input, () => 0));
  });
});

describe('selectSuccessfulExtracts', () => {
  it('prose 있는 문서만 코드포인트 길이로 선발한다', () => {
    let n = 0;
    const newId = () => `id-${++n}`;
    const selected = selectSuccessfulExtracts(
      [
        { documentId: 'a', prose: '가나다' },
        { documentId: 'b', prose: null },
        { documentId: 'c', prose: ' '.repeat(3) },
        { documentId: 'd', prose: '𠀀𠀁' },
      ],
      newId,
    );
    expect(selected).toEqual([
      { id: 'id-1', refId: 'a', content: '가나다', characterCount: 3 },
      { id: 'id-2', refId: 'd', content: '𠀀𠀁', characterCount: 2 },
    ]);
  });
});

describe('corpusConflict', () => {
  it('비어 있으면 충돌 아님 (최초 동결)', () => {
    expect(corpusConflict([], ['a', 'b'])).toBe(false);
  });

  it('기존 세트 == 내 선발이면 충돌 아님 (재시도 시 succeeded 유지)', () => {
    expect(corpusConflict(['b', 'a'], ['a', 'b'])).toBe(false);
  });

  it('다른 문서가 섞였으면 충돌', () => {
    expect(corpusConflict(['a', 'x'], ['a', 'b'])).toBe(true);
  });

  it('개수가 다르면 충돌', () => {
    expect(corpusConflict(['a'], ['a', 'b'])).toBe(true);
  });
});
