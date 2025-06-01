import { Client, HttpConnection } from '@elastic/elasticsearch';
import { env } from '@/env';

export const elastic = new Client({
  Connection: HttpConnection,
  node: env.ELASTICSEARCH_URL,
  auth: { apiKey: env.ELASTICSEARCH_API_KEY },
});
