#!/usr/bin/env node

process.env.SCRIPT = '1';

const { and, asc, eq, gt } = await import('drizzle-orm');
const { PublicationState, SiteState } = await import('@typie/lib/enums');
const { db, pg, Publications, Sites } = await import('#/db/index.ts');
const { elasticsearch, esIndex } = await import('#/search.ts');
const { buildDiscoveryTagsQuery } = await import('#/utils/discovery-core.ts');
const { toTagIndexDocument } = await import('#/utils/discovery-index-core.ts');
const { indexPublications, indexSites, runBulk } = await import('#/utils/discovery-indexer.ts');

const apply = process.argv.includes('--yes');
const BATCH = 200;

const countRows = async () => {
  const [publications, sites] = await Promise.all([
    db.$count(Publications, eq(Publications.state, PublicationState.PUBLISHED)),
    db.$count(Sites, eq(Sites.state, SiteState.ACTIVE)),
  ]);
  return { publications, sites };
};

const counts = await countRows();
const tags = await buildDiscoveryTagsQuery(db, {});
console.log(`published publications: ${counts.publications}, active sites: ${counts.sites}, discoverable tags: ${tags.length}`);
console.log(`target indices: ${esIndex.publications}, ${esIndex.sites}, ${esIndex.tags}`);

if (apply) {
  for (const index of [esIndex.publications, esIndex.sites, esIndex.tags]) {
    const wiped = await elasticsearch.deleteByQuery({ index, conflicts: 'proceed', refresh: true, query: { match_all: {} } });
    if (wiped.timed_out || (wiped.failures?.length ?? 0) > 0) {
      throw new Error(`wipe failed for ${index}: ${JSON.stringify({ timed_out: wiped.timed_out, failures: wiped.failures })}`);
    }
    console.log(`wiped ${index}: deleted ${wiped.deleted}/${wiped.total}, version conflicts ${wiped.version_conflicts}`);
  }

  let after = '';
  let indexedPublications = 0;
  for (;;) {
    const rows = await db
      .select({ id: Publications.id })
      .from(Publications)
      .where(and(eq(Publications.state, PublicationState.PUBLISHED), gt(Publications.id, after)))
      .orderBy(asc(Publications.id))
      .limit(BATCH);
    if (rows.length === 0) break;
    await indexPublications(
      rows.map(({ id }) => id),
      { syncTags: false },
    );
    indexedPublications += rows.length;
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    after = rows.at(-1)!.id;
    console.log(`publications ${indexedPublications}/${counts.publications}`);
  }

  after = '';
  let indexedSites = 0;
  for (;;) {
    const rows = await db
      .select({ id: Sites.id })
      .from(Sites)
      .where(and(eq(Sites.state, SiteState.ACTIVE), gt(Sites.id, after)))
      .orderBy(asc(Sites.id))
      .limit(BATCH);
    if (rows.length === 0) break;
    await indexSites(
      rows.map(({ id }) => id),
      { cascade: false },
    );
    indexedSites += rows.length;
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    after = rows.at(-1)!.id;
    console.log(`sites ${indexedSites}/${counts.sites}`);
  }

  for (let offset = 0; offset < tags.length; offset += BATCH) {
    const chunk = tags.slice(offset, offset + BATCH);
    await runBulk(chunk.flatMap((tag) => [{ index: { _index: esIndex.tags, _id: tag.name } }, toTagIndexDocument(tag)]));
  }
  console.log(`tags ${tags.length}`);
  console.log('discovery indices rebuilt');
} else {
  console.log('dry run — pass --yes to wipe and rebuild the discovery indices');
}

process.exitCode = 0;
await elasticsearch.close();
await pg.end();
