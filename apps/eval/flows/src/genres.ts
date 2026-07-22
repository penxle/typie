export const GENRES = [
  { key: 'romance', name: '로맨스' },
  { key: 'fantasy', name: '판타지' },
  { key: 'sf', name: 'SF' },
  { key: 'mystery-thriller', name: '미스터리·스릴러' },
  { key: 'modern-drama', name: '현대드라마' },
  { key: 'historical', name: '역사' },
  { key: 'literary-fiction', name: '순문학' },
  { key: 'etc', name: '기타' },
] as const;

export const GENRE_KEYS = new Set<string>(GENRES.map((g) => g.key));

export const normalizeGenre = (value: string): string => (GENRE_KEYS.has(value) ? value : 'etc');

export const allocateByLargestRemainder = (dist: Record<string, number>, size: number): Record<string, number> => {
  const total = Object.values(dist).reduce((s, n) => s + n, 0);
  if (total === 0) return {};
  if (size >= total) return { ...dist };

  const entries = Object.entries(dist).map(([key, count]) => {
    const exact = (count / total) * size;
    return { key, count, floor: Math.floor(exact), remainder: exact - Math.floor(exact) };
  });
  const alloc = Object.fromEntries(entries.map((e) => [e.key, Math.min(e.floor, e.count)]));
  let remaining = size - Object.values(alloc).reduce((s, n) => s + n, 0);
  const byRemainder = entries.toSorted((a, b) => b.remainder - a.remainder);
  while (remaining > 0) {
    let advanced = false;
    for (const e of byRemainder) {
      if (remaining === 0) break;
      if (alloc[e.key] < e.count) {
        alloc[e.key] += 1;
        remaining -= 1;
        advanced = true;
      }
    }
    if (!advanced) break;
  }
  return alloc;
};
