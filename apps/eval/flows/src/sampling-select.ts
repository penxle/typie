import { allocateByLargestRemainder, GENRES } from './genres.ts';
import type { Candidate, ExtractResult } from './internal-api.ts';

export type Classified = { candidate: Candidate; literary: boolean; kind: string; genre: string };
export type LiteraryDoc = { documentId: string; genre: string };
export type SelectedDocument = { id: string; refId: string; content: string; characterCount: number };
export type StratifiedSelection = {
  genreDist: Record<string, number>;
  allocation: Record<string, number>;
  picks: LiteraryDoc[];
};

const SPARES_PER_STRATUM = 2;

export const pickLiteraryDocs = (classified: Classified[]): LiteraryDoc[] =>
  classified.filter((c) => c.literary).map((c) => ({ documentId: c.candidate.documentId, genre: c.genre }));

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

export const corpusConflict = (existingRefIds: string[], selectedRefIds: string[]): boolean => {
  if (existingRefIds.length === 0) return false;
  if (existingRefIds.length !== selectedRefIds.length) return true;
  const mine = new Set(selectedRefIds);
  return existingRefIds.some((refId) => !mine.has(refId));
};
