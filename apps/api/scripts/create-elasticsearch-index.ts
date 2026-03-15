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

// Delete existing indices
for (const index of [esIndex.documents, esIndex.folders]) {
  const exists = await elasticsearch.indices.exists({ index });
  if (exists) {
    await elasticsearch.indices.delete({ index });
  }
}

// Create documents index
await elasticsearch.indices.create({
  index: esIndex.documents,
  settings: indexSettings,
  mappings: {
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
  },
});

// Create folders index
await elasticsearch.indices.create({
  index: esIndex.folders,
  settings: indexSettings,
  mappings: {
    properties: {
      site_id: { type: 'keyword' },
      name: { type: 'text', analyzer: 'korean' },
      name_decomposed: { type: 'text', analyzer: 'decomposed', search_analyzer: 'decomposed_search' },
      ancestor_ids: { type: 'keyword' },
      updated_at: { type: 'date' },
    },
  },
});

console.log('Elasticsearch indices created.');

process.exit(0);
