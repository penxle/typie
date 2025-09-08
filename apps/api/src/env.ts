import { z } from 'zod';

const schema = z.object({
  ANTHROPIC_API_KEY: z.string(),
  APPLE_APP_APPLE_ID: z.coerce.number(),
  APPLE_APP_BUNDLE_ID: z.string(),
  APPLE_IAP_ISSUER_ID: z.string(),
  APPLE_IAP_KEY_ID: z.string(),
  APPLE_IAP_PRIVATE_KEY: z.string(),
  APPLE_SIGN_IN_KEY_ID: z.string(),
  APPLE_SIGN_IN_PRIVATE_KEY: z.string(),
  APPLE_TEAM_ID: z.string(),
  AUTH_URL: z.string(),
  DATABASE_URL: z.string(),
  GITHUB_TOKEN: z.string(),
  GOOGLE_OAUTH_CLIENT_ID: z.string(),
  GOOGLE_OAUTH_CLIENT_SECRET: z.string(),
  GOOGLE_PLAY_PACKAGE_NAME: z.string(),
  GOOGLE_SERVICE_ACCOUNT: z.string(),
  IFRAMELY_API_KEY: z.string(),
  KAKAO_CLIENT_ID: z.string(),
  KAKAO_CLIENT_SECRET: z.string(),
  LISTEN_PORT: z.coerce.number().optional(),
  MEILISEARCH_API_KEY: z.string(),
  MEILISEARCH_URL: z.string(),
  NAVER_CLIENT_ID: z.string(),
  NAVER_CLIENT_SECRET: z.string(),
  OIDC_CLIENT_ID: z.string(),
  OIDC_CLIENT_SECRET: z.string(),
  OIDC_JWK: z.string(),
  PORTONE_API_SECRET: z.string(),
  PORTONE_CHANNEL_KEY: z.string(),
  RABBITMQ_URL: z.string(),
  REDIS_URL: z.string(),
  SENTRY_DSN: z.string().optional(),
  SLACK_BOT_TOKEN: z.string(),
  SLACK_SIGNING_SECRET: z.string(),
  SLACK_WEBHOOK_URL: z.string(),
  SPELLCHECK_API_KEY: z.string(),
  SPELLCHECK_URL: z.string(),
  USERSITE_URL: z.string(),
  WEBSITE_URL: z.string(),
});

export const env = schema.parse(process.env.ENV_JSON ? JSON.parse(process.env.ENV_JSON) : process.env);
export const stack = process.env.PUBLIC_PULUMI_STACK ?? process.env.DOPPLER_ENVIRONMENT ?? 'local';
export const dev = process.env.NODE_ENV !== 'production';
export const production = stack === 'prod';
