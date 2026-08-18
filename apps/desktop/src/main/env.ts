export type EnvName = 'prod' | 'dev' | 'local';

export type Env = {
  name: EnvName;
  websiteUrl: string;
  authUrl: string;
  oidcClientId: string;
  updateUrl: string;
  secureCookies: boolean;
};

export const ENVIRONMENTS: Record<EnvName, Env> = {
  prod: {
    name: 'prod',
    websiteUrl: 'https://typie.co',
    authUrl: 'https://auth.typie.co',
    oidcClientId: 'typie',
    updateUrl: 'https://download.typie.net/desktop/',
    secureCookies: true,
  },
  dev: {
    name: 'dev',
    websiteUrl: 'https://typie.dev',
    authUrl: 'https://auth.typie.dev',
    oidcClientId: 'typie',
    updateUrl: 'https://download.typie.net/desktop/',
    secureCookies: true,
  },
  local: {
    name: 'local',
    websiteUrl: 'http://localhost:4100',
    authUrl: 'http://localhost:4300',
    oidcClientId: 'typie',
    updateUrl: 'https://download.typie.net/desktop/',
    secureCookies: false,
  },
};

const isEnvName = (value: string | undefined): value is EnvName => value === 'prod' || value === 'dev' || value === 'local';

const resolveEnvName = (): EnvName => {
  const mode = import.meta.env.MODE;
  if (mode === 'prod' || mode === 'production') return 'prod';
  const override = process.env.ENVIRONMENT;
  if (isEnvName(override)) return override;
  return mode === 'dev' ? 'dev' : 'local';
};

export const env: Env = ENVIRONMENTS[resolveEnvName()];
