import { describe, expect, it } from 'vitest';
import { splitHighlight } from './highlight-terms';

describe('splitHighlight', () => {
  it('marks every occurrence of the query', () => {
    expect(splitHighlight('고양이 둘, 작은 집', '고양이')).toEqual([
      { text: '고양이', hit: true },
      { text: ' 둘, 작은 집', hit: false },
    ]);
  });

  it('matches each whitespace-separated term case-insensitively', () => {
    expect(splitHighlight('SQLite를 브라우저에서', 'sqlite 브라우저')).toEqual([
      { text: 'SQLite', hit: true },
      { text: '를 ', hit: false },
      { text: '브라우저', hit: true },
      { text: '에서', hit: false },
    ]);
  });

  it('treats regex metacharacters in the query literally', () => {
    expect(splitHighlight('C++ 입문', 'C++')).toEqual([
      { text: 'C++', hit: true },
      { text: ' 입문', hit: false },
    ]);
  });

  it('returns the text untouched without a query or a match', () => {
    expect(splitHighlight('느린 산책자', ' '.repeat(3))).toEqual([{ text: '느린 산책자', hit: false }]);
    expect(splitHighlight('느린 산책자', '고양이')).toEqual([{ text: '느린 산책자', hit: false }]);
    expect(splitHighlight('', '고양이')).toEqual([{ text: '', hit: false }]);
  });
});
