export const DISCOVERY_QUERY_MAX_LENGTH = 200;
export const DISCOVERY_SEARCH_PUBLICATION_SIZE = 20;
export const DISCOVERY_SEARCH_SPACE_SIZE = 3;
export const DISCOVERY_SEARCH_TAG_SIZE = 10;

type SearchInput = { index: string; query: string; decomposedQuery: string | null };
type SortClause = Record<string, 'asc' | 'desc'>;

export const normalizeSearchQuery = (raw: string): string => raw.trim().slice(0, DISCOVERY_QUERY_MAX_LENGTH);

const highlight = {
  fields: {
    title: {},
    subtitle: {},
    text: { fragment_size: 200, number_of_fragments: 1 },
  },
  pre_tags: ['<em>'],
  post_tags: ['</em>'],
  encoder: 'html' as const,
};

export const buildPublicationSearchRequest = (input: SearchInput) => ({
  index: input.index,
  size: DISCOVERY_SEARCH_PUBLICATION_SIZE,
  _source: false as const,
  query: {
    bool: {
      should: [
        { match: { title: { query: input.query, boost: 3 } } },
        { match: { subtitle: { query: input.query, boost: 2 } } },
        { match: { tags_text: { query: input.query, boost: 2 } } },
        { match: { text: { query: input.query } } },
        ...(input.decomposedQuery
          ? [
              { match: { title_decomposed: { query: input.decomposedQuery, boost: 1.5 } } },
              { match: { subtitle_decomposed: { query: input.decomposedQuery, boost: 1 } } },
            ]
          : []),
      ],
      filter: [{ term: { discoverable: true } }],
      minimum_should_match: 1,
    },
  },
  highlight,
});

export const buildSpaceSearchRequest = (input: SearchInput) => ({
  index: input.index,
  size: DISCOVERY_SEARCH_SPACE_SIZE,
  _source: false as const,
  query: {
    bool: {
      should: [
        { match: { name: { query: input.query, boost: 3 } } },
        { match: { description: { query: input.query } } },
        ...(input.decomposedQuery ? [{ match: { name_decomposed: { query: input.decomposedQuery, boost: 1.5 } } }] : []),
      ],
      minimum_should_match: 1,
    },
  },
});

export const buildTagSearchRequest = (input: SearchInput) => ({
  index: input.index,
  size: DISCOVERY_SEARCH_TAG_SIZE,
  _source: false as const,
  query: {
    bool: {
      should: [
        { match: { name: { query: input.query, boost: 2 } } },
        ...(input.decomposedQuery ? [{ match: { name_decomposed: { query: input.decomposedQuery } } }] : []),
      ],
      minimum_should_match: 1,
    },
  },
  sort: [{ _score: 'desc' }, { count: 'desc' }] as SortClause[],
});

export const filterHitsByIds = <T extends { id: string }>(hits: T[], allowedIds: Iterable<string>): T[] => {
  const allowed = new Set(allowedIds);
  return hits.filter((hit) => allowed.has(hit.id));
};
