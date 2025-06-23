/** @type {import('prettier').Config} */
// eslint-disable-next-line import/no-default-export
export default {
  htmlWhitespaceSensitivity: 'ignore',
  printWidth: 140,
  singleQuote: true,

  xmlWhitespaceSensitivity: 'ignore',
  xmlQuoteAttributes: 'double',

  plugins: ['prettier-plugin-packagejson', 'prettier-plugin-sh', 'prettier-plugin-svelte', '@prettier/plugin-xml'],

  overrides: [
    {
      files: '**/*.{ts,tsx}',
      options: {
        parser: 'typescript',
      },
    },
  ],
};
