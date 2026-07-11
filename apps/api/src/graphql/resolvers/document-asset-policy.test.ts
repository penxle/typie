import assert from 'node:assert/strict';
import test from 'node:test';
import { selectAuthorizedDocumentAssetIds, validateAndCanonicalizeDocumentAssetIds } from './document-asset-policy.ts';

test('owner can resolve an asset before document materialization', () => {
  assert.deepEqual(
    selectAuthorizedDocumentAssetIds({
      canonicalIds: ['IMG0OWNER'],
      ownedIds: ['IMG0OWNER'],
      referencedIds: [],
    }),
    ['IMG0OWNER'],
  );
});

test('collaborator can resolve an asset after it is materialized in the document', () => {
  assert.deepEqual(
    selectAuthorizedDocumentAssetIds({
      canonicalIds: ['FILE0SHARED'],
      ownedIds: [],
      referencedIds: ['FILE0SHARED'],
    }),
    ['FILE0SHARED'],
  );
});

test('unowned and unreferenced asset ids are rejected without revealing a reason', () => {
  assert.deepEqual(
    selectAuthorizedDocumentAssetIds({
      canonicalIds: ['EMBD0OTHER'],
      ownedIds: [],
      referencedIds: [],
    }),
    [],
  );
});

test('requested ids are canonicalized while preserving request order', () => {
  assert.deepEqual(validateAndCanonicalizeDocumentAssetIds(['FILE0B', 'IMG0A', 'FILE0B']), ['FILE0B', 'IMG0A']);
});

test('oversized requests are rejected before authorization work', () => {
  assert.throws(() => validateAndCanonicalizeDocumentAssetIds(Array.from({ length: 51 }, (_, index) => `IMG0${index}`)));
});

test('unsupported and malformed ids are rejected', () => {
  for (const id of ['U0USER', 'IMG-not-an-id']) {
    assert.throws(() => validateAndCanonicalizeDocumentAssetIds([id]));
  }
});
