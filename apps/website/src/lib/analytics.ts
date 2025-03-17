import Mixpanel from 'mixpanel-browser';

export const setupMixpanel = () => {
  Mixpanel.init('1e356c26031e78256d39f6907b366be8', {
    ignore_dnt: true,
    persistence: 'localStorage',
  });
};
