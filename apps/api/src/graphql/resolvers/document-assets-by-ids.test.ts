import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveDocumentAssetsByIds } from './document-assets-by-ids.ts';
import type { DocumentAssetAccess } from './document-assets-by-ids.ts';

test('resolver combines owner and materialized-reference access across asset types', async () => {
  const ownershipRequests: string[][] = [];
  const access: DocumentAssetAccess = {
    loadOwnedIds: async ({ ids }) => {
      ownershipRequests.push(ids);
      return ['IMG0OWNED'];
    },
    loadReferencedIds: async () => ['FILE0SHARED', 'DAN0ARCHIVED'],
  };

  const result = await resolveDocumentAssetsByIds({
    documentId: 'D0DOCUMENT',
    userId: 'U0VIEWER',
    requestedIds: ['IMG0OWNED', 'FILE0SHARED', 'EMBD0MISSING', 'DAN0ARCHIVED'],
    access,
  });

  assert.deepEqual(result, ['IMG0OWNED', 'FILE0SHARED', 'DAN0ARCHIVED']);
  assert.deepEqual(ownershipRequests, [['IMG0OWNED', 'FILE0SHARED', 'EMBD0MISSING']]);
});

test('owner resolves before materialization while an unrelated user asset remains hidden', async () => {
  const access: DocumentAssetAccess = {
    loadOwnedIds: async () => ['EMBD0OWNED'],
    loadReferencedIds: async () => [],
  };

  assert.deepEqual(
    await resolveDocumentAssetsByIds({
      documentId: 'D0DOCUMENT',
      userId: 'U0OWNER',
      requestedIds: ['EMBD0OWNED', 'IMG0OTHER'],
      access,
    }),
    ['EMBD0OWNED'],
  );
});

test('anonymous or collaborator access uses materialized references only', async () => {
  let ownershipLoads = 0;
  const access: DocumentAssetAccess = {
    loadOwnedIds: async () => {
      ownershipLoads += 1;
      return ['IMG0OTHER'];
    },
    loadReferencedIds: async () => ['FILE0SHARED'],
  };

  assert.deepEqual(
    await resolveDocumentAssetsByIds({
      documentId: 'D0DOCUMENT',
      userId: null,
      requestedIds: ['IMG0OTHER', 'FILE0SHARED'],
      access,
    }),
    ['FILE0SHARED'],
  );
  assert.equal(ownershipLoads, 0);
});

test('archived nodes are reference-only even for authenticated users', async () => {
  let requestedOwnedIds: string[] | null = null;
  const access: DocumentAssetAccess = {
    loadOwnedIds: async ({ ids }) => {
      requestedOwnedIds = ids;
      return ids;
    },
    loadReferencedIds: async () => [],
  };

  assert.deepEqual(
    await resolveDocumentAssetsByIds({
      documentId: 'D0DOCUMENT',
      userId: 'U0USER',
      requestedIds: ['DAN0ARCHIVED'],
      access,
    }),
    [],
  );
  assert.equal(requestedOwnedIds, null);
});

test('empty input returns before the access gateway is invoked', async () => {
  let calls = 0;
  const access: DocumentAssetAccess = {
    loadOwnedIds: async () => {
      calls += 1;
      return [];
    },
    loadReferencedIds: async () => {
      calls += 1;
      return [];
    },
  };

  assert.deepEqual(
    await resolveDocumentAssetsByIds({
      documentId: 'D0DOCUMENT',
      userId: 'U0USER',
      requestedIds: [],
      access,
    }),
    [],
  );
  assert.equal(calls, 0);
});

test('invalid input is rejected before the access gateway is invoked', async () => {
  let calls = 0;
  const access: DocumentAssetAccess = {
    loadOwnedIds: async () => {
      calls += 1;
      return [];
    },
    loadReferencedIds: async () => {
      calls += 1;
      return [];
    },
  };

  await assert.rejects(() =>
    resolveDocumentAssetsByIds({
      documentId: 'D0DOCUMENT',
      userId: 'U0USER',
      requestedIds: ['U0PROBE'],
      access,
    }),
  );
  assert.equal(calls, 0);
});
