/** @type {import('syncpack').RcFile} */
export default {
  versionGroups: [
    {
      dependencies: ['firebase-admin'],
      packages: ['firebase-messaging-scripts'],
      isIgnored: true,
    },
  ],
};
