import { GetParameterCommand, SSMClient } from '@aws-sdk/client-ssm';

type Env = {
  CLOUDFLARE_API_KEY: string;
  CLOUDFLARE_AIGATEWAY_URL: string;
  SLACK_BOT_TOKEN: string;
  SLACK_SIGNING_SECRET: string;
  API_KEY: string;
  API_BASE_URL: string;
};

const SSM_PARAMS: Record<keyof Env, string> = {
  CLOUDFLARE_API_KEY: '/bmo/CLOUDFLARE_API_KEY',
  CLOUDFLARE_AIGATEWAY_URL: '/bmo/CLOUDFLARE_AIGATEWAY_URL',
  SLACK_BOT_TOKEN: '/bmo/SLACK_BOT_TOKEN',
  SLACK_SIGNING_SECRET: '/bmo/SLACK_SIGNING_SECRET',
  API_KEY: '/bmo/API_KEY',
  API_BASE_URL: '/bmo/API_BASE_URL',
};

const ssm = new SSMClient({});
let cached: Env | null = null;

const fetchParam = async (name: string): Promise<string> => {
  const res = await ssm.send(new GetParameterCommand({ Name: name, WithDecryption: true }));
  const value = res.Parameter?.Value;
  if (!value) throw new Error(`SSM parameter ${name} not found`);
  return value;
};

export const loadEnv = async (): Promise<Env> => {
  if (cached) return cached;

  const entries = await Promise.all(
    (Object.entries(SSM_PARAMS) as [keyof Env, string][]).map(async ([key, param]) => [key, await fetchParam(param)] as const),
  );

  cached = Object.fromEntries(entries) as Env;
  return cached;
};
