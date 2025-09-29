/** @type {import("syncpack").RcFile} */
module.exports = {
  dependencyTypes: ['prod', 'dev'],
  lintFormatting: false,
  semverGroups: [
    {
      packages: ['**'],
      dependencies: ['@pulumi/pulumi'],
      range: '',
    },
    {
      packages: ['**'],
      dependencies: ['**'],
      range: '^',
    },
  ],
};
