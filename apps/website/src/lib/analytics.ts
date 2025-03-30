import Mixpanel from 'mixpanel-browser';
import { env } from '$env/dynamic/public';

export const setupMixpanel = () => {
  Mixpanel.init(env.PUBLIC_MIXPANEL_TOKEN, {
    api_host: 'https://mpp.typie.co',
    ignore_dnt: true,
    persistence: 'localStorage',
  });
};
