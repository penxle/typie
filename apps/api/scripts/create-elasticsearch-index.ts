#!/usr/bin/env bun

import { elastic } from '@/search';

await elastic.indices.delete({
  index: 'posts',
  ignore_unavailable: true,
});

await elastic.indices.create({
  index: 'posts',
  mappings: {
    properties: {
      id: { type: 'keyword' },
      siteId: { type: 'keyword' },
      title: {
        type: 'text',
        analyzer: 'nori',
        fields: {
          keyword: { type: 'keyword' },
        },
      },
      subtitle: {
        type: 'text',
        analyzer: 'nori',
        fields: {
          keyword: { type: 'keyword' },
        },
      },
      text: {
        type: 'text',
        analyzer: 'nori',
      },
      updatedAt: { type: 'long' },
    },
  },
  settings: {
    number_of_shards: 1,
    number_of_replicas: 1,
    analysis: {
      analyzer: {
        nori: {
          type: 'custom',
          tokenizer: 'nori_tokenizer',
          filter: ['lowercase'],
        },
      },
      tokenizer: {
        nori_tokenizer: {
          type: 'nori_tokenizer',
          decompound_mode: 'discard',
        },
      },
    },
  },
});

console.log('Elasticsearch index recreated.');

process.exit(0);
