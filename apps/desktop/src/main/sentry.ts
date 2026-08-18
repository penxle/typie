import { init } from '@sentry/electron/main';

export const initSentry = (version: string) => {
  const dsn = import.meta.env.PUBLIC_SENTRY_DSN as string | undefined;
  if (!dsn) return;
  init({ dsn, release: `typie-desktop@${version}` });
};
