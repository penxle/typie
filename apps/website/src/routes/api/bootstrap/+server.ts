import { createHash } from 'node:crypto';
import { error, json } from '@sveltejs/kit';
import { env as privateEnv } from '$env/dynamic/private';
import { env } from '$env/dynamic/public';
import type { Bootstrap } from '$lib/bootstrap';
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

export const GET: RequestHandler = async ({ getClientAddress, cookies }) => {
  const bootstrap = await getBootstrap();

  if (bootstrap?.maintenance.enabled && bootstrap.maintenance.platforms.includes('web')) {
    const clientIp = getClientAddress();
    if (!['127.0.0.1', '::1', ...bootstrap.maintenance.allowedIps].includes(clientIp)) {
      const bypassKeyHash = cookies.get('typie-bb');
      if (privateEnv.PRIVATE_BOOTSTRAP_BYPASS_KEY) {
        const expectedHash = createHash('sha256').update(privateEnv.PRIVATE_BOOTSTRAP_BYPASS_KEY).digest('hex');
        if (bypassKeyHash === expectedHash) {
          return json(bootstrap);
        }
      }
      error(503, {
        message: bootstrap.maintenance.message,
        code: 'maintenance',
        maintenance: {
          title: bootstrap.maintenance.title,
          message: bootstrap.maintenance.message,
          until: bootstrap.maintenance.until,
        },
      });
    }
  }

  return json(bootstrap);
};
