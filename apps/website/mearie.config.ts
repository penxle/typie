import { defineConfig } from 'mearie';

export default defineConfig({
  schema: 'schema.graphql',
  document: 'src/**/*.{ts,svelte}',
  scalars: {
    BigInt: 'string',
    Binary: 'string',
    DateTime: 'string',
    JSON: 'any',
  },
});
