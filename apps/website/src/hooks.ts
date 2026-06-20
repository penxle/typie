import { env } from '$env/dynamic/public';
import type { Reroute } from '@sveltejs/kit';

export const reroute: Reroute = async ({ url }) => {
  if (
    url.pathname === '/graphql' ||
    url.pathname === '/robots.txt' ||
    url.pathname === '/api/bootstrap' ||
    url.pathname === '/_internal/bb' ||
    url.pathname === '/_internal/ffi'
  ) {
    return url.pathname;
  }
  if (url.origin === env.PUBLIC_AUTH_URL) {
    return `/auth${url.pathname}`;
  }
  if (url.origin === env.PUBLIC_WEBSITE_URL) {
    return `/website${url.pathname}`;
  }
  if (url.host === env.PUBLIC_USERSITE_HOST) {
    return `/usersite/apex${url.pathname}`;
  }
  if (url.host.endsWith(`.${env.PUBLIC_USERSITE_HOST}`)) {
    return `/usersite/wildcard${url.pathname}`;
  }

  throw new Error('Invalid URL');
};
