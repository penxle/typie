import type { Candidate, ExtractResult } from './internal-api.ts';

export type Classified = { candidate: Candidate; literary: boolean; kind: string };
export type SelectedDocument = { id: string; refId: string; content: string; characterCount: number };

export const pickLiterary = (classified: Classified[]): Candidate[] => classified.filter((c) => c.literary).map((c) => c.candidate);

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
