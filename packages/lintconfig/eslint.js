import js from '@eslint/js';
import { defineConfig } from 'eslint/config';
import import_ from 'eslint-plugin-import';
import simpleImportSort from 'eslint-plugin-simple-import-sort';
import svelte from 'eslint-plugin-svelte';
import unicorn from 'eslint-plugin-unicorn';
import globals from 'globals';
import typescript from 'typescript-eslint';
import { ignore } from './eslint-ignore.js';

// eslint-disable-next-line import/no-default-export
export default defineConfig([
  { ignores: [ignore] },
  js.configs.recommended,
  unicorn.configs.recommended,
  typescript.configs.strict,
  typescript.configs.stylistic,
  svelte.configs.recommended,
  svelte.configs.prettier,
  {
    languageOptions: {
      globals: { ...globals.node, ...globals.browser },
    },
    linterOptions: {
      reportUnusedDisableDirectives: true,
    },
    plugins: { import: import_, 'simple-import-sort': simpleImportSort },
    rules: {
      'no-undef': 'off',
      'object-shorthand': ['error', 'always'],
      '@typescript-eslint/consistent-type-definitions': ['error', 'type'],
      'import/consistent-type-specifier-style': ['error', 'prefer-top-level'],
      'import/first': 'error',
      'import/newline-after-import': ['error', { considerComments: true }],
      'import/no-default-export': 'error',
      'import/no-duplicates': 'error',
      'import/no-named-default': 'error',
      'simple-import-sort/exports': 'error',
      'simple-import-sort/imports': [
        'error',
        {
          groups: [
            [String.raw`^\u0000`],
            [
              '^node:',
              String.raw`^@?\w`,
              '^',
              String.raw`^\.`,
              String.raw`^node:.*\u0000$`,
              String.raw`^@?\w.*\u0000$`,
              String.raw`\u0000$`,
              String.raw`^\..*\u0000$`,
            ],
          ],
        },
      ],
      'svelte/no-target-blank': 'error',
      'svelte/block-lang': ['error', { script: ['ts'] }],
      'svelte/button-has-type': 'error',
      'svelte/require-store-reactive-access': 'off',
      'svelte/sort-attributes': 'error',
      'unicorn/catch-error-name': ['error', { name: 'err' }],
      'unicorn/consistent-function-scoping': 'off',
      'unicorn/no-array-callback-reference': 'off',
      'unicorn/no-array-for-each': 'off',
      'unicorn/no-array-method-this-argument': 'off',
      'unicorn/no-array-reduce': 'off',
      'unicorn/no-empty-file': 'off',
      'unicorn/no-null': 'off',
      'unicorn/prefer-switch': 'off',
      'unicorn/prefer-ternary': 'off',
      'unicorn/prevent-abbreviations': 'off',
    },
  },
  {
    files: ['**/*.config.[jt]s'],
    rules: {
      'import/no-default-export': 'off',
    },
  },
  {
    files: ['**/*.[jt]sx'],
    rules: {
      'import/no-default-export': 'off',
      'unicorn/filename-case': ['error', { cases: { kebabCase: true, pascalCase: true } }],
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.[jt]s'],
    languageOptions: {
      parserOptions: {
        projectService: true,
        extraFileExtensions: ['.svelte'],
        parser: typescript.parser,
      },
    },
    rules: {
      'unicorn/filename-case': ['error', { cases: { kebabCase: true, pascalCase: true } }],
    },
  },
  {
    files: ['**/pulumi/**/*.ts'],
    rules: {
      'unicorn/prefer-spread': 'off',
    },
  },
]);
