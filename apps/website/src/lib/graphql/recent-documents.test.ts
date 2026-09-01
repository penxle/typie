import { get } from 'svelte/store';
import { beforeEach, describe, expect, it } from 'vitest';
import { invalidateRecentDocuments, invalidateRecentDocumentsForSort, recentDocumentInvalidationVersions } from './recent-documents';

beforeEach(() => {
  recentDocumentInvalidationVersions.set(new Map());
});

describe('recent document invalidation', () => {
  it('marks both sort orders dirty when a site snapshot may be stale', () => {
    invalidateRecentDocuments('SITE');

    expect(get(recentDocumentInvalidationVersions).get('SITE')).toEqual({
      VIEWED_AT: 1,
      UPDATED_AT: 1,
    });
  });

  it('marks only the affected sort order dirty', () => {
    invalidateRecentDocumentsForSort('UPDATED_AT', 'SITE');

    expect(get(recentDocumentInvalidationVersions).get('SITE')).toEqual({
      VIEWED_AT: 0,
      UPDATED_AT: 1,
    });
  });
});
