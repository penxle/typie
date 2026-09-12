#!/usr/bin/env node

import { elasticsearch, esIndex } from '#/search.ts';

process.env.SCRIPT = '1';

const indexSettings = {
  analysis: {
    analyzer: {
      korean: {
        type: 'custom' as const,
        tokenizer: 'nori_mixed',
        filter: ['nori_readingform', 'lowercase'],
      },
      decomposed: {
        type: 'custom' as const,
        tokenizer: 'standard',
        filter: ['edge_ngram_filter', 'lowercase'],
      },
      decomposed_search: {
        type: 'custom' as const,
        tokenizer: 'standard',
        filter: ['lowercase'],
      },
    },
    tokenizer: {
      nori_mixed: {
        type: 'nori_tokenizer' as const,
        decompound_mode: 'mixed' as const,
      },
    },
    filter: {
      edge_ngram_filter: {
        type: 'edge_ngram' as const,
        min_gram: 1,
        max_gram: 20,
      },
    },
  },
};

const skipExisting = process.argv.includes('--skip-existing');
const targets = [esIndex.documents, esIndex.folders, esIndex.publications, esIndex.spaces, esIndex.tags];
const existing = new Set<string>();

for (const index of targets) {
  const exists = await elasticsearch.indices.exists({ index });
  if (!exists) continue;
  if (skipExisting) {
    existing.add(index);
    continue;
  }
  await elasticsearch.indices.delete({ index });
}

const createIndex = async (index: string, mappings: Parameters<typeof elasticsearch.indices.create>[0]['mappings']) => {
  if (existing.has(index)) {
    console.log(`skipped ${index} (exists)`);
    return;
  }
  await elasticsearch.indices.create({ index, settings: indexSettings, mappings });
  console.log(`created ${index}`);
};

await createIndex(esIndex.documents, {
  properties: {
    site_id: { type: 'keyword' },
    title: { type: 'text', analyzer: 'korean' },
    title_decomposed: { type: 'text', analyzer: 'decomposed', search_analyzer: 'decomposed_search' },
    subtitle: { type: 'text', analyzer: 'korean' },
    subtitle_decomposed: { type: 'text', analyzer: 'decomposed', search_analyzer: 'decomposed_search' },
    text: { type: 'text', analyzer: 'korean', index_options: 'offsets' },
    ancestor_ids: { type: 'keyword' },
    updated_at: { type: 'date' },
  },
});

await createIndex(esIndex.folders, {
  properties: {
    site_id: { type: 'keyword' },
    name: { type: 'text', analyzer: 'korean' },
    name_decomposed: { type: 'text', analyzer: 'decomposed', search_analyzer: 'decomposed_search' },
    ancestor_ids: { type: 'keyword' },
    updated_at: { type: 'date' },
  },
});

await createIndex(esIndex.publications, {
  properties: {
    space_id: { type: 'keyword' },
    discoverable: { type: 'boolean' },
    title: { type: 'text', analyzer: 'korean' },
    title_decomposed: { type: 'text', analyzer: 'decomposed', search_analyzer: 'decomposed_search' },
    subtitle: { type: 'text', analyzer: 'korean' },
    subtitle_decomposed: { type: 'text', analyzer: 'decomposed', search_analyzer: 'decomposed_search' },
    text: { type: 'text', analyzer: 'korean', index_options: 'offsets' },
    tags: { type: 'keyword' },
    tags_text: { type: 'text', analyzer: 'korean' },
    published_at: { type: 'date' },
  },
});

await createIndex(esIndex.spaces, {
  properties: {
    name: { type: 'text', analyzer: 'korean' },
    name_decomposed: { type: 'text', analyzer: 'decomposed', search_analyzer: 'decomposed_search' },
    description: { type: 'text', analyzer: 'korean' },
  },
});

await createIndex(esIndex.tags, {
  properties: {
    name: { type: 'text', analyzer: 'korean' },
    name_decomposed: { type: 'text', analyzer: 'decomposed', search_analyzer: 'decomposed_search' },
    count: { type: 'integer' },
  },
});

console.log('Elasticsearch indices created.');

process.exit(0);
