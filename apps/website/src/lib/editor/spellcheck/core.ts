import { nanoid } from 'nanoid';
import type { Transaction } from '@tiptap/pm/state';
import type { Mapping } from '@tiptap/pm/transform';
import type { CheckSpellingResult, SpellingError } from './types';

export function mapPosition(pos: number, mapping: { mapResult: (pos: number) => { deleted: boolean; pos: number } }): number | null {
  const result = mapping.mapResult(pos);
  return result.deleted ? null : result.pos;
}

export function mapErrors(errors: CheckSpellingResult[], mapping?: Mapping): SpellingError[] {
  return errors
    .map((error) => {
      const mappedFrom = mapping ? mapPosition(error.from, mapping) : error.from;
      const mappedTo = mapping ? mapPosition(error.to, mapping) : error.to;

      if (mappedFrom === null || mappedTo === null) {
        return null;
      }

      return {
        id: nanoid(),
        from: mappedFrom,
        to: mappedTo,
        context: error.context,
        corrections: error.corrections,
        explanation: error.explanation,
      };
    })
    .filter((error): error is SpellingError => error !== null);
}

export function updateErrorPositions(errors: SpellingError[], transaction: Transaction): SpellingError[] {
  const newErrors: SpellingError[] = [];

  for (const error of errors) {
    const mappedFrom = mapPosition(error.from, transaction.mapping);
    const mappedTo = mapPosition(error.to, transaction.mapping);

    if (mappedFrom !== null && mappedTo !== null) {
      newErrors.push({
        ...error,
        from: mappedFrom,
        to: mappedTo,
      });
    }
  }

  return newErrors;
}

export function decodeHtmlEntities(html: string): string {
  const doc = new DOMParser().parseFromString(html, 'text/html');
  return doc.documentElement.textContent || '';
}
