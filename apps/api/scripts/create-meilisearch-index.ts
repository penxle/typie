#!/usr/bin/env node

import { meilisearch } from '@/search';

await meilisearch.deleteIndexIfExists('documents');
await meilisearch.createIndex('documents', {
  primaryKey: 'id',
});

const documents = meilisearch.index('documents');
await documents.updateSettings({
  searchableAttributes: ['title', 'subtitle', 'text'],
  filterableAttributes: ['siteId', 'updatedAt'],
  sortableAttributes: ['updatedAt'],
  typoTolerance: { enabled: false },
});

await meilisearch.deleteIndexIfExists('folders');
await meilisearch.createIndex('folders', {
  primaryKey: 'id',
});

const folders = meilisearch.index('folders');
await folders.updateSettings({
  searchableAttributes: ['name'],
  filterableAttributes: ['siteId'],
  typoTolerance: { enabled: false },
});

console.log('Meilisearch index recreated.');

process.exit(0);
