// 2차 창작의 원작 배경을 웹에서 찾아온다.
//
// 질의는 원작 쪽 정보(derivativeSource, properNouns)로만 짓는다. 원고 문장을 질의에 넣으면
// 이용자가 쓴 글이 외부 검색 서비스로 나가고, 평가 동의서의 "외부 유출 금지"와 정면으로 어긋난다.
// 고유명사는 원작에 속한 이름이라 같은 문제가 생기지 않는다 — 이 경계를 코드로 못박아 둔다.

export type SearchHit = { title: string; url: string; text: string };

// 고유명사를 통째로 넣으면 질의가 흐려진다. 원작명이 있으면 그것이 주어이고, 없을 때만
// 이름 몇 개로 원작을 되짚는다.
export const MAX_QUERY_NOUNS = 6;

export const buildBackgroundQuery = (input: { derivativeSource?: string | null; properNouns: string[] }): string | null => {
  const source = input.derivativeSource?.trim();
  const nouns = input.properNouns
    .map((n) => n.trim())
    .filter((n) => n.length > 0)
    .slice(0, MAX_QUERY_NOUNS);

  if (source && source !== '원작 불명') {
    return `${source} 등장인물 관계 설정 줄거리${nouns.length > 0 ? ` ${nouns.slice(0, 3).join(' ')}` : ''}`;
  }
  // 원작명을 모르면 이름만으로 되짚는다. 이름이 너무 적으면 엉뚱한 결과를 부르므로 포기한다.
  if (nouns.length < 2) return null;
  return `${nouns.join(' ')} 등장인물 원작 어느 작품`;
};

export type ExaResult = { title?: string; url?: string; text?: string };

// 실패는 조용히 삼킨다 — 배경은 있으면 좋은 것이고, 없다고 분석을 멈출 이유가 없다.
export const searchBackground = async (input: {
  apiKey: string;
  query: string;
  numResults?: number;
  maxCharacters?: number;
}): Promise<SearchHit[]> => {
  const response = await fetch('https://api.exa.ai/search', {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'x-api-key': input.apiKey },
    body: JSON.stringify({
      query: input.query,
      numResults: input.numResults ?? 5,
      type: 'auto',
      contents: { text: { maxCharacters: input.maxCharacters ?? 4000 } },
    }),
  });
  if (!response.ok) {
    throw new Error(`exa search failed: ${response.status}`);
  }
  const body = (await response.json()) as { results?: ExaResult[] };
  return (body.results ?? [])
    .filter((r): r is ExaResult & { text: string } => typeof r.text === 'string' && r.text.trim().length > 0)
    .map((r) => ({ title: r.title ?? '', url: r.url ?? '', text: r.text }));
};

export const renderSearchHits = (hits: SearchHit[]): string =>
  hits.map((h, i) => `[${i + 1}] ${h.title}\n${h.url}\n${h.text}`).join('\n\n---\n\n');
