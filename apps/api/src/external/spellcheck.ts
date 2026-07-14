import ky from 'ky';
import { redis } from '#/cache.ts';
import { env } from '#/env.ts';
import { createSpellcheck } from './spellcheck-core.ts';

export type { SpellingError } from './spellcheck-core.ts';

export const check = createSpellcheck({
  cacheGet: (key) => redis.get(key),
  cacheSet: async (key, ttlSeconds, value) => {
    await redis.setex(key, ttlSeconds, value);
  },
  requestXml: (text, signal) =>
    ky
      .post(env.SPELLCHECK_URL, {
        headers: { 'x-api-key': env.SPELLCHECK_API_KEY },
        json: { sentence: text },
        signal,
      })
      .text(),
});
