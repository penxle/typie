import Mixpanel from 'mixpanel-browser';
import { env } from '$env/dynamic/public';

export const setupMixpanel = () => {
  Mixpanel.init(env.PUBLIC_MIXPANEL_TOKEN, {
    persistence: 'localStorage',
  });
};
