import { MeiliSearch } from 'meilisearch';
import { env } from '@/env';

export const meili = new MeiliSearch({
  host: 'https://meili.typie.io',
  apiKey: env.MEILISEARCH_API_KEY,
});
