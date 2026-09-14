import { SiteState, SpaceState } from '@typie/lib/enums';
import { db } from '#/db/index.ts';
import { elasticsearch, esIndex } from '#/search.ts';
import { buildDiscoveryTagCountsQuery } from './discovery-core.ts';
import {
  buildPublicationIndexRowsQuery,
  buildPublicationTagNamesQuery,
  buildSpaceIndexRowsQuery,
  buildSpaceTagNamesQuery,
  collectTagNames,
  toPublicationIndexDocument,
  toSpaceIndexDocument,
  toTagIndexDocument,
} from './discovery-index-core.ts';
import { buildLatestVersionMetadataQuery } from './publication-core.ts';

type BulkOperation = Record<string, unknown>;

export const runBulk = async (operations: BulkOperation[]) => {
  if (operations.length === 0) return;
  const result = await elasticsearch.bulk({ operations });
  if (result.errors) {
    const failed = result.items.find((item) => Object.values(item)[0]?.error);
    throw new Error(`discovery bulk failed: ${JSON.stringify(failed)}`);
  }
};

const unique = (ids: string[]) => [...new Set(ids)];

export const syncTagCounts = async (names: string[]) => {
  const targets = unique(names);
  if (targets.length === 0) return;

  const rows = await buildDiscoveryTagCountsQuery(db, { names: targets });
  const counts = new Map(rows.map((row) => [row.name, row.count]));
  const operations: BulkOperation[] = [];
  for (const name of targets) {
    const count = counts.get(name) ?? 0;
    if (count > 0) {
      operations.push({ index: { _index: esIndex.tags, _id: name } }, toTagIndexDocument({ name, count }));
    } else {
      operations.push({ delete: { _index: esIndex.tags, _id: name } });
    }
  }
  await runBulk(operations);
};

export const indexPublications = async (publicationIds: string[], options: { syncTags?: boolean } = {}) => {
  const ids = unique(publicationIds);
  if (ids.length === 0) return;

  const [rows, versions, tags, previous] = await Promise.all([
    buildPublicationIndexRowsQuery(db, { publicationIds: ids }),
    buildLatestVersionMetadataQuery(db, { publicationIds: ids }),
    buildPublicationTagNamesQuery(db, { publicationIds: ids }),
    elasticsearch.mget<{ tags?: string[] }>({ index: esIndex.publications, ids, _source: ['tags'] }),
  ]);

  const rowById = new Map(rows.map((row) => [row.id, row]));
  const versionById = new Map(versions.map((version) => [version.publicationId, version]));
  const tagNamesById = new Map<string, string[]>();
  for (const tag of tags) {
    tagNamesById.set(tag.publicationId, [...(tagNamesById.get(tag.publicationId) ?? []), tag.name]);
  }

  const operations: BulkOperation[] = [];
  const touchedTagNames: string[] = [];
  for (const id of ids) {
    const row = rowById.get(id);
    const tagNames = tagNamesById.get(id) ?? [];
    const document = row ? toPublicationIndexDocument({ row, version: versionById.get(id), tagNames }) : null;
    if (document) {
      operations.push({ index: { _index: esIndex.publications, _id: id } }, document);
    } else {
      operations.push({ delete: { _index: esIndex.publications, _id: id } });
    }
    touchedTagNames.push(...tagNames);
  }
  for (const doc of previous.docs) {
    if ('found' in doc && doc.found && doc._source?.tags) touchedTagNames.push(...doc._source.tags);
  }

  await runBulk(operations);

  if (options.syncTags !== false) {
    await syncTagCounts(collectTagNames(touchedTagNames));
  }
};

export const indexSpaces = async (spaceIds: string[], options: { cascade?: boolean } = {}) => {
  const ids = unique(spaceIds);
  if (ids.length === 0) return;

  const rows = await buildSpaceIndexRowsQuery(db, { spaceIds: ids });
  const rowById = new Map(rows.map((row) => [row.id, row]));

  const operations: BulkOperation[] = [];
  for (const id of ids) {
    const row = rowById.get(id);
    const document = row ? toSpaceIndexDocument(row) : null;
    if (document) {
      operations.push({ index: { _index: esIndex.spaces, _id: id } }, document);
    } else {
      operations.push({ delete: { _index: esIndex.spaces, _id: id } });
    }
  }
  await runBulk(operations);

  if (options.cascade === false) return;

  for (const id of ids) {
    const row = rowById.get(id);
    const alive = row !== undefined && row.state === SpaceState.ACTIVE && row.siteState === SiteState.ACTIVE;
    if (alive) {
      const updated = await elasticsearch.updateByQuery({
        index: esIndex.publications,
        conflicts: 'proceed',
        query: { term: { space_id: id } },
        script: {
          source: 'ctx._source.discoverable = params.discoverable',
          params: { discoverable: row.allowIndexing && row.allowDiscovery },
        },
      });
      if (updated.timed_out || (updated.failures?.length ?? 0) > 0) {
        throw new Error(
          `discovery space cascade failed for ${id}: ${JSON.stringify({ timed_out: updated.timed_out, failures: updated.failures })}`,
        );
      }
    } else {
      const deleted = await elasticsearch.deleteByQuery({
        index: esIndex.publications,
        conflicts: 'proceed',
        query: { term: { space_id: id } },
      });
      if (deleted.timed_out || (deleted.failures?.length ?? 0) > 0) {
        throw new Error(
          `discovery space cascade failed for ${id}: ${JSON.stringify({ timed_out: deleted.timed_out, failures: deleted.failures })}`,
        );
      }
    }
  }

  const names = await buildSpaceTagNamesQuery(db, { spaceIds: ids });
  await syncTagCounts(names.map((row) => row.name));
};
