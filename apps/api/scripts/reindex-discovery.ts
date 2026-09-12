#!/usr/bin/env node

process.env.SCRIPT = '1';

const { and, asc, eq, gt } = await import('drizzle-orm');
const { PublicationState, SpaceState } = await import('@typie/lib/enums');
const { db, pg, Publications, Spaces } = await import('#/db/index.ts');
const { elasticsearch, esIndex } = await import('#/search.ts');
const { buildDiscoveryTagsQuery } = await import('#/utils/discovery-core.ts');
const { toTagIndexDocument } = await import('#/utils/discovery-index-core.ts');
const { indexPublications, indexSpaces, runBulk } = await import('#/utils/discovery-indexer.ts');

const apply = process.argv.includes('--yes');
const BATCH = 200;

const countRows = async () => {
  const [publications, spaces] = await Promise.all([
    db.$count(Publications, eq(Publications.state, PublicationState.PUBLISHED)),
    db.$count(Spaces, eq(Spaces.state, SpaceState.ACTIVE)),
  ]);
  return { publications, spaces };
};

const counts = await countRows();
const tags = await buildDiscoveryTagsQuery(db, {});
console.log(`published publications: ${counts.publications}, active spaces: ${counts.spaces}, discoverable tags: ${tags.length}`);
console.log(`target indices: ${esIndex.publications}, ${esIndex.spaces}, ${esIndex.tags}`);

if (apply) {
  for (const index of [esIndex.publications, esIndex.spaces, esIndex.tags]) {
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
  let indexedSpaces = 0;
  for (;;) {
    const rows = await db
      .select({ id: Spaces.id })
      .from(Spaces)
      .where(and(eq(Spaces.state, SpaceState.ACTIVE), gt(Spaces.id, after)))
      .orderBy(asc(Spaces.id))
      .limit(BATCH);
    if (rows.length === 0) break;
    await indexSpaces(
      rows.map(({ id }) => id),
      { cascade: false },
    );
    indexedSpaces += rows.length;
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    after = rows.at(-1)!.id;
    console.log(`spaces ${indexedSpaces}/${counts.spaces}`);
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
