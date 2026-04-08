import assert from 'node:assert/strict';
import test from 'node:test';
import { enqueueSearchSyncForEntityIds, filterVisibleSearchHits } from './search-index.ts';

test('filterVisibleSearchHits removes deleted document and folder hits while preserving order', async () => {
  const visibleHits = await filterVisibleSearchHits(
    [
      { kind: 'document', id: 'doc-active-1', payload: { label: 'first doc' } },
      { kind: 'folder', id: 'folder-deleted', payload: { label: 'deleted folder' } },
      { kind: 'document', id: 'doc-deleted', payload: { label: 'deleted doc' } },
      { kind: 'folder', id: 'folder-active', payload: { label: 'active folder' } },
    ],
    {
      findActiveDocumentIds: async (ids) => ids.filter((id) => id === 'doc-active-1'),
      findActiveFolderIds: async (ids) => ids.filter((id) => id === 'folder-active'),
    },
  );

  assert.deepEqual(visibleHits, [
    { kind: 'document', id: 'doc-active-1', payload: { label: 'first doc' } },
    { kind: 'folder', id: 'folder-active', payload: { label: 'active folder' } },
  ]);
});

test('enqueueSearchSyncForEntityIds schedules both document and folder search jobs', async () => {
  const jobs: { name: string; id: string }[] = [];

  await enqueueSearchSyncForEntityIds(['entity-document', 'entity-folder'], {
    findDocumentIdsByEntityIds: async (entityIds) => entityIds.filter((id) => id === 'entity-document').map(() => 'doc-1'),
    findFolderIdsByEntityIds: async (entityIds) => entityIds.filter((id) => id === 'entity-folder').map(() => 'folder-1'),
    enqueueDocumentIndexJob: async (id) => {
      jobs.push({ name: 'search:index:document', id });
    },
    enqueueFolderIndexJob: async (id) => {
      jobs.push({ name: 'search:index:folder', id });
    },
  });

  assert.deepEqual(jobs, [
    { name: 'search:index:document', id: 'doc-1' },
    { name: 'search:index:folder', id: 'folder-1' },
  ]);
});
