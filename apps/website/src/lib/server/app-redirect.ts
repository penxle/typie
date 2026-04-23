import { APP_STORE_URL, PLAY_STORE_URL } from '@typie/lib/const';

export const resolveAppStoreUrl = (userAgent: string | null | undefined, desktopFallback: string) => {
  if (!userAgent) {
    return desktopFallback;
  }

  if (/android/i.test(userAgent)) {
    return PLAY_STORE_URL;
  } else if (/iphone|ipad|ipod/i.test(userAgent)) {
    return APP_STORE_URL;
  } else {
    return desktopFallback;
  }
};
