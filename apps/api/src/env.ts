import { z } from 'zod';

const schema = z.object({
  AUTH_URL: z.string(),
  DATABASE_URL: z.string(),
  GOOGLE_OAUTH_CLIENT_ID: z.string(),
  GOOGLE_OAUTH_CLIENT_SECRET: z.string(),
  IFRAMELY_API_KEY: z.string(),
  KAKAO_CLIENT_ID: z.string(),
  KAKAO_CLIENT_SECRET: z.string(),
  LISTEN_PORT: z.coerce.number().optional(),
  MEILISEARCH_API_KEY: z.string(),
  NAVER_CLIENT_ID: z.string(),
  NAVER_CLIENT_SECRET: z.string(),
  OIDC_CLIENT_ID: z.string(),
  OIDC_CLIENT_SECRET: z.string(),
  OIDC_JWK: z.string(),
  PORTONE_API_SECRET: z.string(),
  PORTONE_CHANNEL_KEY: z.string(),
  PUBLIC_PULUMI_STACK: z.string().optional(),
  REDIS_URL: z.string(),
  SENTRY_DSN: z.string().optional(),
  USERSITE_URL: z.string(),
  WEBSITE_URL: z.string(),
});

export const env = schema.parse(process.env);
export const dev = process.env.NODE_ENV !== 'production';
export const production = process.env.PUBLIC_PULUMI_STACK
  ? process.env.PUBLIC_PULUMI_STACK === 'prod'
  : process.env.DOPPLER_ENVIRONMENT === 'prod';
