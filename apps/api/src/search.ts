import { Client, HttpConnection } from '@elastic/elasticsearch';
import { env, production } from '@/env';

export const elasticsearch = new Client({
  cloud: { id: env.ELASTICSEARCH_CLOUD_ID },
  auth: { apiKey: env.ELASTICSEARCH_API_KEY },
  Connection: HttpConnection,
});

const indexPrefix = production ? 'prod' : 'dev';

export const esIndex = {
  documents: `${indexPrefix}-documents`,
  folders: `${indexPrefix}-folders`,
} as const;
