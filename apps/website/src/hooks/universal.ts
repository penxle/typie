import { env } from '$env/dynamic/public';
import type { Reroute } from '@sveltejs/kit';

export const reroute: Reroute = async ({ url }) => {
  if (url.pathname === '/graphql') {
    return url.pathname;
  } else if (url.origin === env.PUBLIC_AUTH_URL) {
    return `/auth${url.pathname}`;
  } else if (url.origin === env.PUBLIC_WEBSITE_URL) {
    return `/website${url.pathname}`;
  } else if (url.host === env.PUBLIC_USERSITE_HOST) {
    return `/usersite/apex${url.pathname}`;
  } else if (url.host.endsWith(`.${env.PUBLIC_USERSITE_HOST}`)) {
    return `/usersite/wildcard${url.pathname}`;
  }

  throw new Error('Invalid URL');
};
