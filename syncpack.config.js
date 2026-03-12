/** @type {import('syncpack').RcFile} */
export default {
  versionGroups: [
    {
      dependencies: ['svelte'],
      pinVersion: '5.53.6',
    },
    {
      dependencies: ['firebase-admin'],
      packages: ['firebase-messaging-scripts'],
      isIgnored: true,
    },
  ],
};
