import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

const UPSTREAM = 'https://typie.co/api/changelog';
const MAX_PAGE = 200;

export const GET: RequestHandler = async ({ url, fetch }) => {
  const page = Math.max(1, Math.trunc(Number(url.searchParams.get('page'))) || 1);
  if (page > MAX_PAGE) return json({ entries: [], hasMore: false });

  try {
    const response = await fetch(`${UPSTREAM}?page=${page}`);
    if (!response.ok) return json({ entries: [], hasMore: false }, { status: 502 });
    return json(await response.json(), { headers: { 'cache-control': 'public, max-age=60' } });
  } catch {
    return json({ entries: [], hasMore: false }, { status: 502 });
  }
};
