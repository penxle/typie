import { env } from '$env/dynamic/public';

const getStartUrl = (userAgent?: string) => {
  if (!userAgent) {
    return env.PUBLIC_AUTH_URL;
  }

  if (/android/i.test(userAgent)) {
    return 'https://play.google.com/store/apps/details?id=co.typie';
  } else if (/iphone|ipad|ipod/i.test(userAgent ?? '')) {
    return 'https://apps.apple.com/app/id6745595771';
  } else {
    return env.PUBLIC_AUTH_URL;
  }
};

export const load = (event) => {
  const userAgent = event.request.headers.get('user-agent')?.toLowerCase();

  return {
    startUrl: getStartUrl(userAgent),
  };
};
