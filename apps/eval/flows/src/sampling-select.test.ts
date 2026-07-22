import { describe, expect, it } from 'vitest';
import {
  candidateLimitFor,
  corpusConflict,
  fillQuotas,
  MAX_CANDIDATES,
  pickLiteraryDocs,
  selectSuccessfulExtracts,
  shuffle,
  stratifySelection,
} from './sampling-select.ts';
import type { Candidate } from './internal-api.ts';
import type { LiteraryDoc } from './sampling-select.ts';

const candidate = (documentId: string): Candidate => ({ documentId, characterCount: 100 });

const seededRng = (seq: number[]): (() => number) => {
  let i = 0;
  return () => seq[i++ % seq.length] ?? 0;
};

describe('pickLiteraryDocs', () => {
  it('literary=true 후보만 장르와 함께 남긴다', () => {
    const result = pickLiteraryDocs([
      { candidate: candidate('a'), literary: true, kind: '소설', genre: 'romance' },
      { candidate: candidate('b'), literary: false, kind: '메모', genre: 'etc' },
      { candidate: candidate('c'), literary: true, kind: '수필', genre: 'literary-fiction' },
    ]);
    expect(result).toEqual([
      { documentId: 'a', genre: 'romance' },
      { documentId: 'c', genre: 'literary-fiction' },
    ]);
  });
});

describe('stratifySelection', () => {
  const doc = (documentId: string, genre: string): LiteraryDoc => ({ documentId, genre });

  it('genreDist를 GENRES 순서로 만들고 배분+여유 2편을 층 크기로 제한한다', () => {
    const docs = [
      ...Array.from({ length: 6 }, (_, i) => doc(`r${i}`, 'romance')),
      ...Array.from({ length: 4 }, (_, i) => doc(`f${i}`, 'fantasy')),
      doc('s0', 'sf'),
    ];
    const { genreDist, allocation, picks } = stratifySelection(docs, 5, () => 0);
    expect(Object.keys(genreDist)).toEqual(['romance', 'fantasy', 'sf']);
    expect(genreDist).toEqual({ romance: 6, fantasy: 4, sf: 1 });
    expect(Object.values(allocation).reduce((s, n) => s + n, 0)).toBe(5);
    const perGenre = new Map<string, number>();
    for (const p of picks) perGenre.set(p.genre, (perGenre.get(p.genre) ?? 0) + 1);
    expect(perGenre.get('romance')).toBe(Math.min((allocation.romance ?? 0) + 2, 6));
    expect(perGenre.get('sf')).toBe(1);
    expect(picks.length).toBeGreaterThanOrEqual(5);
  });

  it('동일 rng로 결정적이다', () => {
    const docs = Array.from({ length: 10 }, (_, i) => doc(`r${i}`, 'romance'));
    const a = stratifySelection(docs, 3, seededRng([0.1, 0.7, 0.3, 0.9]));
    const b = stratifySelection(docs, 3, seededRng([0.1, 0.7, 0.3, 0.9]));
    expect(a).toEqual(b);
  });
});

describe('fillQuotas', () => {
  const ext = (id: string, genre: string) => ({ id, genre });

  it('층별 배분량만큼 채운다', () => {
    const chosen = fillQuotas(
      [ext('r0', 'romance'), ext('r1', 'romance'), ext('f0', 'fantasy'), ext('s0', 'sf')],
      { romance: 1, fantasy: 1, sf: 1 },
      3,
    );
    expect(chosen.map((c) => c.id).toSorted((a, b) => a.localeCompare(b))).toEqual(['f0', 'r0', 's0']);
  });

  it('부족분은 성공분 전체에서 최대 잔여법으로 재배분한다', () => {
    const chosen = fillQuotas(
      [ext('r0', 'romance'), ext('r1', 'romance'), ext('r2', 'romance'), ext('f0', 'fantasy')],
      { romance: 1, sf: 2 },
      3,
    );
    expect(chosen.length).toBe(3);
    const ids = new Set(chosen.map((c) => c.id));
    expect(ids.has('r0')).toBe(true);
  });

  it('성공분이 부족하면 가능한 만큼만 반환한다', () => {
    const chosen = fillQuotas([ext('r0', 'romance'), ext('f0', 'fantasy')], { romance: 5, fantasy: 5 }, 5);
    expect(chosen.length).toBe(2);
  });
});

describe('candidateLimitFor', () => {
  it('코퍼스 크기의 20배수를 요청한다', () => {
    expect(candidateLimitFor(30)).toBe(600);
  });

  it('소형 코퍼스는 하한 100을 지킨다', () => {
    expect(candidateLimitFor(3)).toBe(100);
  });

  it('대형 코퍼스는 api 스키마 상한에서 잘린다', () => {
    expect(candidateLimitFor(500)).toBe(MAX_CANDIDATES);
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
