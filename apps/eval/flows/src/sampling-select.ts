import { isAccepted } from './corpus-filter.ts';
import { allocateByLargestRemainder, GENRES } from './genres.ts';
import type { Candidate, ExtractResult } from './internal-api.ts';

export type Classified = {
  candidate: Candidate;
  kind: string;
  genre: string;
  narrative: boolean;
  singleWork: boolean;
  selfContained: boolean;
  original: boolean;
};
export type LiteraryDoc = { documentId: string; genre: string };
export type SelectedDocument = { id: string; refId: string; content: string; characterCount: number };
export type StratifiedSelection = {
  genreDist: Record<string, number>;
  allocation: Record<string, number>;
  picks: LiteraryDoc[];
};

const SPARES_PER_STRATUM = 2;

// 후보 수는 코퍼스 크기에 비례한다. TABLESAMPLE 시절에는 20배를 요청해도 자격 풀의
// ~10%(~230편)만 와서 상한이 유명무실했는데, 행 단위 무작위로 바꾸며 상한이 실효화됐다 —
// 후보 전수가 opus 심사를 거치므로 배수가 곧 표집 비용이다. 5배는 실측 성공 깔때기
// (50편 선별에 후보 230=4.6배로 충분, 2026-07-30)에 여유를 얹은 값이고, 모자라면 실행을
// 한 번 더 하는 쪽이 배수를 올려두는 것보다 싸다(오너 확정).
// 하한 100은 소형 코퍼스에서도 장르 배분이 성립하게, 상한은 api candidatesSchema와 동일
// (후보 텍스트가 한 응답으로 오므로 응답 크기가 실질 상한이다 — 2000편 ≈ 최대 60MB).
export const CANDIDATES_PER_DOC = 5;
export const MAX_CANDIDATES = 2000;
export const candidateLimitFor = (size: number): number => Math.min(MAX_CANDIDATES, Math.max(100, size * CANDIDATES_PER_DOC));

export const pickLiteraryDocs = (classified: Classified[]): LiteraryDoc[] =>
  classified.filter((c) => isAccepted(c)).map((c) => ({ documentId: c.candidate.documentId, genre: c.genre }));

// 이미 들인 문서(이전 표집·반입 전부)는 후보에서 뺀다 — 본문·심사 비용을 내기 전에 걸러야
// 하고, 어드민 문구("이미 들인 글은 건너뜁니다")가 약속하는 동작이기도 하다.
export const excludeExisting = (candidates: Candidate[], existingRefIds: Set<string>): Candidate[] =>
  candidates.filter((c) => !existingRefIds.has(c.documentId));

// 작성자당 상한 — 문서 단위 무작위는 다작 작성자에게 비례 이상으로 쏠리고(50편 중 한
// 작성자 5편, 2026-07-30 실측), 장르 쿼터가 희소 장르에서 이를 다시 증폭한다. 심사를
// 통과한 뒤·배분 전에 자르면 어느 장르로 세든 작성자당 cap편을 넘지 못한다.
export const capPerAuthor = (docs: LiteraryDoc[], authorOf: Map<string, string>, cap = 1): LiteraryDoc[] => {
  const counts = new Map<string, number>();
  const kept: LiteraryDoc[] = [];
  for (const doc of docs) {
    const author = authorOf.get(doc.documentId);
    // 작성자를 모르는 문서는 자르지 않는다 — 상한은 방어 장치이지 자격 조건이 아니다.
    if (author === undefined) {
      kept.push(doc);
      continue;
    }
    const n = counts.get(author) ?? 0;
    if (n >= cap) continue;
    counts.set(author, n + 1);
    kept.push(doc);
  }
  return kept;
};

export const shuffle = <T>(items: T[], random: () => number = Math.random): T[] => {
  const result = [...items];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    const tmp = result[i];
    result[i] = result[j];
    result[j] = tmp;
  }
  return result;
};

const groupBy = <T>(items: T[], key: (item: T) => string): Map<string, T[]> => {
  const grouped = new Map<string, T[]>();
  for (const item of items) {
    const bucket = grouped.get(key(item)) ?? [];
    bucket.push(item);
    grouped.set(key(item), bucket);
  }
  return grouped;
};

const distByGenre = <T>(grouped: Map<string, T[]>): Record<string, number> => {
  const dist: Record<string, number> = {};
  for (const { key } of GENRES) {
    const n = grouped.get(key)?.length ?? 0;
    if (n > 0) dist[key] = n;
  }
  return dist;
};

export const stratifySelection = (docs: LiteraryDoc[], size: number, random: () => number = Math.random): StratifiedSelection => {
  const grouped = groupBy(docs, (d) => d.genre);
  const genreDist = distByGenre(grouped);
  const allocation = allocateByLargestRemainder(genreDist, size);
  const picks: LiteraryDoc[] = [];
  for (const { key } of GENRES) {
    const pool = grouped.get(key);
    if (!pool || pool.length === 0) continue;
    const take = Math.min((allocation[key] ?? 0) + SPARES_PER_STRATUM, pool.length);
    for (const doc of shuffle(pool, random).slice(0, take)) {
      picks.push(doc);
    }
  }
  return { genreDist, allocation, picks };
};

export const fillQuotas = <T extends { genre: string }>(extracts: T[], allocation: Record<string, number>, size: number): T[] => {
  const grouped = groupBy(extracts, (e) => e.genre);
  const chosen: T[] = [];
  const used = new Set<T>();
  for (const { key } of GENRES) {
    const pool = grouped.get(key) ?? [];
    const quota = allocation[key] ?? 0;
    for (let i = 0; i < pool.length && i < quota && chosen.length < size; i++) {
      chosen.push(pool[i]);
      used.add(pool[i]);
    }
  }
  if (chosen.length < size) {
    const remaining = extracts.filter((e) => !used.has(e));
    const extra = allocateByLargestRemainder(distByGenre(groupBy(remaining, (e) => e.genre)), size - chosen.length);
    for (const { key } of GENRES) {
      const take = extra[key] ?? 0;
      let taken = 0;
      for (const item of remaining) {
        if (taken >= take || chosen.length >= size) break;
        if (item.genre === key) {
          chosen.push(item);
          taken += 1;
        }
      }
    }
  }
  return chosen;
};

export const selectSuccessfulExtracts = (results: ExtractResult[], newId: () => string): SelectedDocument[] => {
  const selected: SelectedDocument[] = [];
  for (const { documentId, prose } of results) {
    if (!prose || !prose.trim()) continue;
    selected.push({ id: newId(), refId: documentId, content: prose, characterCount: [...prose].length });
  }
  return selected;
};
