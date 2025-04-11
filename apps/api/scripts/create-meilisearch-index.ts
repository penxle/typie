#!/usr/bin/env bun

import { meili } from '@/search';

await meili.createIndex('posts', {
  primaryKey: 'id',
});

const posts = meili.index('posts');

await posts.updateSearchableAttributes(['title', 'subtitle', 'text']);
await posts.updateFilterableAttributes(['siteId', 'updatedAt']);
await posts.updateSortableAttributes(['updatedAt']);
