#!/usr/bin/env node

import { meili } from '@/search';

await meili.deleteIndexIfExists('posts');
await meili.createIndex('posts', {
  primaryKey: 'id',
});

const posts = meili.index('posts');
await posts.updateSettings({
  searchableAttributes: ['title', 'subtitle', 'text'],
  filterableAttributes: ['siteId', 'updatedAt'],
  sortableAttributes: ['updatedAt'],
  typoTolerance: { enabled: false },
});

console.log('Meilisearch index recreated.');

process.exit(0);
