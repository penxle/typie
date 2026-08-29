import { json } from '@sveltejs/kit';
import { gql, GraphQLClient } from 'graphql-request';
import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

const ITEMS_PER_PAGE = 5;
const MAX_PAGE = 200;
const CACHE_TTL = 60_000;

type ChangelogEntry = {
  id: string;
  title: string;
  date: string;
  image: { url: string } | null;
  body: string;
};

type ChangelogHighlight = {
  id: string;
  title: string;
  date: string;
  image: { url: string } | null;
};

type ListResult = { entries: ChangelogEntry[]; hasMore: boolean };
type HighlightResult = { entry: ChangelogHighlight | null };

const listQuery = gql`
  query GetChangelogs($stage: Stage!, $first: Int!, $skip: Int!) {
    changelogs(orderBy: date_DESC, stage: $stage, first: $first, skip: $skip) {
      id
      title
      date
      image {
        url(transformation: { document: { output: { format: autoImage } } })
      }
      body
    }
    changelogsConnection(stage: $stage) {
      aggregate {
        count
      }
    }
  }
`;

const highlightQuery = gql`
  query GetHighlightedChangelog($stage: Stage!) {
    changelogs(orderBy: date_DESC, stage: $stage, where: { highlight: true }, first: 1) {
      id
      title
      date
      image {
        url(transformation: { document: { output: { format: autoImage } } })
      }
    }
  }
`;

const listCache = new Map<number, { data: ListResult; fetchedAt: number }>();
const listFetching = new Map<number, Promise<ListResult | null>>();

let highlightCache: { data: HighlightResult; fetchedAt: number } | null = null;
let highlightFetching: Promise<HighlightResult | null> | null = null;

const client = () => new GraphQLClient(env.PUBLIC_CMS_URL);

const fetchList = async (page: number): Promise<ListResult | null> => {
  try {
    const data = await client().request<{
      changelogs: ChangelogEntry[];
      changelogsConnection: { aggregate: { count: number } };
    }>(listQuery, {
      stage: env.PUBLIC_CMS_STAGE,
      first: ITEMS_PER_PAGE,
      skip: (page - 1) * ITEMS_PER_PAGE,
    });

    const entries = data.changelogs.map((entry) => ({ ...entry, image: entry.image ?? null }));

    return { entries, hasMore: page * ITEMS_PER_PAGE < data.changelogsConnection.aggregate.count };
  } catch {
    return null;
  }
};

const fetchHighlight = async (): Promise<HighlightResult | null> => {
  try {
    const data = await client().request<{ changelogs: ChangelogHighlight[] }>(highlightQuery, {
      stage: env.PUBLIC_CMS_STAGE,
    });

    const entry = data.changelogs[0];

    return { entry: entry ? { ...entry, image: entry.image ?? null } : null };
  } catch {
    return null;
  }
};

const getList = async (page: number): Promise<ListResult | null> => {
  const now = Date.now();

  const cached = listCache.get(page);
  if (cached && now - cached.fetchedAt < CACHE_TTL) {
    return cached.data;
  }

  const inflight = listFetching.get(page);
  if (inflight) {
    return inflight;
  }

  const fetching = fetchList(page).then((data) => {
    if (data) {
      listCache.set(page, { data, fetchedAt: now });
    }
    listFetching.delete(page);
    return data ?? listCache.get(page)?.data ?? null;
  });

  listFetching.set(page, fetching);

  return fetching;
};

const getHighlight = async (): Promise<HighlightResult | null> => {
  const now = Date.now();

  if (highlightCache && now - highlightCache.fetchedAt < CACHE_TTL) {
    return highlightCache.data;
  }

  if (highlightFetching) {
    return highlightFetching;
  }

  highlightFetching = fetchHighlight().then((data) => {
    if (data) {
      highlightCache = { data, fetchedAt: now };
    }
    highlightFetching = null;
    return data ?? highlightCache?.data ?? null;
  });

  return highlightFetching;
};

export const GET: RequestHandler = async ({ url }) => {
  if (url.searchParams.get('highlight') === '1') {
    const result = await getHighlight();
    return result ? json(result) : json({ entry: null } satisfies HighlightResult, { status: 502 });
  }

  const page = Math.max(1, Math.trunc(Number(url.searchParams.get('page'))) || 1);

  if (page > MAX_PAGE) {
    return json({ entries: [], hasMore: false } satisfies ListResult);
  }

  const result = await getList(page);

  return result ? json(result) : json({ entries: [], hasMore: false } satisfies ListResult, { status: 502 });
};
