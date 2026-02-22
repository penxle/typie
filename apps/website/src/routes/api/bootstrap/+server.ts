import { json } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
import type { Bootstrap } from '$lib/bootstrap.server';
import type { RequestHandler } from './$types';

const CACHE_TTL = 60_000;

let cached: { data: Bootstrap; fetchedAt: number } | null = null;
let fetching: Promise<Bootstrap | null> | null = null;

async function fetchBootstrap(): Promise<Bootstrap | null> {
  try {
    const resp = await fetch(`https://config.typie.net/bootstrap/${env.PUBLIC_ENVIRONMENT}.json`);
    if (!resp.ok) return null;
    return (await resp.json()) as Bootstrap;
  } catch {
    return null;
  }
}

async function getBootstrap(): Promise<Bootstrap | null> {
  const now = Date.now();

  if (cached && now - cached.fetchedAt < CACHE_TTL) {
    return cached.data;
  }

  if (fetching) {
    return fetching;
  }

  fetching = fetchBootstrap().then((data) => {
    if (data) {
      cached = { data, fetchedAt: now };
    }
    fetching = null;
    return data ?? cached?.data ?? null;
  });

  return fetching;
}

export const GET: RequestHandler = async () => {
  const bootstrap = await getBootstrap();
  return json(bootstrap);
};
