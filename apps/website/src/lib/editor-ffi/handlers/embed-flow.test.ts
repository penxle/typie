import { describe, expect, it } from 'vitest';
import type { Message } from '@typie/editor-ffi/browser';

const createSetEmbedAttrsMessage = (nodeId: string, embedId: string): Message => ({
  type: 'node',
  op: {
    type: 'set_attrs',
    id: nodeId,
    attrs: {
      type: 'embed',
      id: embedId,
    },
  },
});

const createDeleteNodeMessage = (nodeId: string): Message => ({
  type: 'node',
  op: {
    type: 'delete',
    id: nodeId,
  },
});

const normalizeUrl = (raw: string): string => (/^[^:]+:\/\//.test(raw) ? raw : `https://${raw}`);

describe('embed flow messages', () => {
  it('creates a set_attrs message with embed type and id', () => {
    expect(createSetEmbedAttrsMessage('node-1', 'embed-1')).toEqual({
      type: 'node',
      op: {
        type: 'set_attrs',
        id: 'node-1',
        attrs: { type: 'embed', id: 'embed-1' },
      },
    });
  });

  it('creates a delete message for a node', () => {
    expect(createDeleteNodeMessage('node-1')).toEqual({
      type: 'node',
      op: {
        type: 'delete',
        id: 'node-1',
      },
    });
  });
});

describe('URL normalization', () => {
  it('adds https:// prefix when scheme is missing', () => {
    expect(normalizeUrl('example.com')).toBe('https://example.com');
  });

  it('preserves https:// URLs as-is', () => {
    expect(normalizeUrl('https://example.com')).toBe('https://example.com');
  });

  it('preserves http:// URLs as-is', () => {
    expect(normalizeUrl('http://example.com')).toBe('http://example.com');
  });

  it('preserves URLs with other schemes as-is', () => {
    expect(normalizeUrl('ftp://example.com')).toBe('ftp://example.com');
  });
});
