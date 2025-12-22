#!/usr/bin/env node

import { meilisearch } from '@/search';

await meilisearch.deleteIndexIfExists('posts');
await meilisearch.createIndex('posts', {
  primaryKey: 'id',
});

const posts = meilisearch.index('posts');
await posts.updateSettings({
  searchableAttributes: ['title', 'subtitle', 'text'],
  filterableAttributes: ['siteId', 'updatedAt'],
  sortableAttributes: ['updatedAt'],
  typoTolerance: { enabled: false },
});

console.log('Meilisearch index recreated.');

process.exit(0);
