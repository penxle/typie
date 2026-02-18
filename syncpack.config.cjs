/** @type {import("syncpack").RcFile} */
module.exports = {
  semverGroups: [
    {
      packages: ['**'],
      dependencies: ['**'],
      range: '^',
    },
  ],
  versionGroups: [
    {
      label: 'Ignore canvas (optional native dependency)',
      dependencies: ['canvas'],
      isIgnored: true,
    },
  ],
};
