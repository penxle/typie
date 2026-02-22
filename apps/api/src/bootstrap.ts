import { GetObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';
import { stack } from '@/env';
import { s3 } from '@/external/aws';
import { bootstrapSchema } from '@/validation';
import type { z } from 'zod';

type Bootstrap = z.infer<typeof bootstrapSchema>;

export async function fetchBootstrap(): Promise<Bootstrap> {
  const response = await s3.send(
    new GetObjectCommand({
      Bucket: 'typie-config',
      Key: `bootstrap/${stack}.json`,
    }),
  );

  const body = await response.Body?.transformToString();
  if (!body) {
    throw new Error('bootstrap config not found');
  }

  return bootstrapSchema.parse(JSON.parse(body));
}

export async function putBootstrap(data: Omit<Bootstrap, 'version' | 'updatedAt'>): Promise<Bootstrap> {
  const newData: Bootstrap = {
    version: 1,
    updatedAt: new Date().toISOString(),
    ...data,
  };

  await s3.send(
    new PutObjectCommand({
      Bucket: 'typie-config',
      Key: `bootstrap/${stack}.json`,
      Body: JSON.stringify(newData, null, 2),
      ContentType: 'application/json',
      CacheControl: 'no-cache, no-store',
    }),
  );

  return newData;
}

const CACHE_TTL = 60_000;
let cached: { data: Bootstrap; fetchedAt: number } | null = null;
let fetching: Promise<Bootstrap | null> | null = null;

export async function getBootstrap(): Promise<Bootstrap | null> {
  const now = Date.now();

  if (cached && now - cached.fetchedAt < CACHE_TTL) {
    return cached.data;
  }

  if (fetching) {
    return fetching;
  }

  fetching = (async () => {
    try {
      const data = await fetchBootstrap();
      cached = { data, fetchedAt: now };
      return data;
    } catch {
      return cached?.data ?? null;
    } finally {
      fetching = null;
    }
  })();

  return fetching;
}
