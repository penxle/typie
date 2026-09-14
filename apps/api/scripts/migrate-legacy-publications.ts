#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { parseArgs } from 'node:util';
import { Worker } from 'node:worker_threads';
import type { LegacyMigrationManifest, LegacyMigrationOutcome, LegacySpaceResolution } from '#/utils/legacy-publication.ts';
import type { LegacyClassification, LegacyPublicDocument, LegacySite } from '#/utils/legacy-publication-core.ts';

process.env.SCRIPT = '1';

const { redis } = await import('#/cache.ts');
const { pg } = await import('#/db/index.ts');
const { classifyLegacyDocument, isDemotedClassification } = await import('#/utils/legacy-publication-core.ts');
const { shardOf } = await import('#/utils/sweep-sharding.ts');
const { demoteLegacyEntities, ensureLegacySpaces, loadLegacyPublicDocuments, loadPublishedDocumentIds, restoreLegacyMigration } =
  await import('#/utils/legacy-publication.ts');

type LegacyWorkerTarget = { document: LegacyPublicDocument; spaceId: string | null };
type LegacyWorkerResult = { documentId: string; entityId: string; siteId: string; ms: number } & LegacyMigrationOutcome;
type LegacyWorkerMessage = { type: 'result'; results: LegacyWorkerResult[] } | { type: 'exhausted' } | { type: 'fatal'; message: string };

const { values } = parseArgs({
  options: {
    yes: { type: 'boolean', default: false },
    workers: { type: 'string' },
    batch: { type: 'string', default: '200' },
    out: { type: 'string', default: '.' },
    restore: { type: 'string' },
  },
});

const dryRun = !values.yes;
const mode = dryRun ? 'dry' : 'apply';
const outDir = values.out;
mkdirSync(outDir, { recursive: true });
const runStamp = new Date()
  .toISOString()
  .replaceAll(/[-:]/g, '')
  .replace(/\.\d+Z$/, 'Z');
const planPath = path.join(outDir, `legacy-migration-plan.${mode}.${runStamp}.json`);
const manifestPath = path.join(outDir, `legacy-migration-manifest.${mode}.${runStamp}.json`);
const reportPath = path.join(outDir, `legacy-migration-report.${mode}.${runStamp}.json`);

const invalidArgs: string[] = [];
const parseCount = (name: string, raw: string | undefined) => {
  const value = Number(raw);
  if (!Number.isSafeInteger(value) || value < 1) {
    invalidArgs.push(`invalid ${name}: ${raw}`);
    return 0;
  }
  return value;
};
const batchSize = parseCount('--batch', values.batch);
const workerCount = values.workers === undefined ? Math.max(1, os.availableParallelism() - 1) : parseCount('--workers', values.workers);

const percentile = (sorted: number[], ratio: number) =>
  sorted.length === 0 ? 0 : sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * ratio))];

if (invalidArgs.length > 0) {
  for (const message of invalidArgs) console.error(message);
  process.exitCode = 1;
} else if (values.restore) {
  if (existsSync(values.restore)) {
    const manifest = JSON.parse(readFileSync(values.restore, 'utf8')) as LegacyMigrationManifest;
    console.log(
      `RESTORE: spaces ${manifest.spaces.length}, publications ${manifest.publications.length}, demoted ${manifest.demoted.length}`,
    );
    const result = await restoreLegacyMigration(manifest);
    writeFileSync(path.join(outDir, `legacy-migration-restore-report.${runStamp}.json`), JSON.stringify(result, null, 2));
    console.log(`restored entities ${result.restoredEntityIds.length}, skipped ${result.skipped.length}`);
    process.exitCode = result.skipped.length > 0 ? 1 : 0;
  } else {
    console.error(`manifest not found: ${values.restore}`);
    process.exitCode = 1;
  }
} else {
  console.log(dryRun ? 'DRY RUN (apply with --yes)' : 'APPLY MODE');

  const documents = await loadLegacyPublicDocuments();
  const publishedDocumentIds = await loadPublishedDocumentIds(documents.map((document) => document.documentId));

  const sitesById = new Map<string, LegacySite>();
  for (const document of documents) {
    if (!sitesById.has(document.siteId)) {
      sitesById.set(document.siteId, {
        siteId: document.siteId,
        siteSlug: document.siteSlug,
        siteName: document.siteName,
        siteLogoId: document.siteLogoId,
        siteDateDisplay: document.siteDateDisplay,
      });
    }
  }

  const preliminary = new Map(
    documents.map((document) => [
      document.documentId,
      classifyLegacyDocument(document, { slugTakenSiteIds: new Set<string>(), publishedDocumentIds }),
    ]),
  );
  const sitesNeedingSpace = [...sitesById.values()].filter((site) =>
    documents.some((document) => document.siteId === site.siteId && preliminary.get(document.documentId) === 'migrated'),
  );
  const resolutions = await ensureLegacySpaces(sitesNeedingSpace, { dryRun });
  const slugTakenSiteIds = new Set([...resolutions].filter(([, resolution]) => resolution.kind === 'taken').map(([siteId]) => siteId));

  const plan = documents.map((document) => ({
    document,
    classification: classifyLegacyDocument(document, { slugTakenSiteIds, publishedDocumentIds }),
  }));
  writeFileSync(
    planPath,
    JSON.stringify(
      plan.map(({ document, classification }) => ({
        documentId: document.documentId,
        entityId: document.entityId,
        siteId: document.siteId,
        siteSlug: document.siteSlug,
        classification,
      })),
      null,
      2,
    ),
  );

  const countBy = (items: { classification: LegacyClassification }[]) => {
    const counts: Record<string, number> = {};
    for (const item of items) counts[item.classification] = (counts[item.classification] ?? 0) + 1;
    return counts;
  };
  const resolutionCounts: Record<LegacySpaceResolution['kind'], number> = { created: 0, reused: 0, planned: 0, taken: 0 };
  for (const resolution of resolutions.values()) resolutionCounts[resolution.kind] += 1;
  console.log(
    `documents ${documents.length}, sites ${sitesById.size}, plan ${JSON.stringify(countBy(plan))}, spaces ${JSON.stringify(resolutionCounts)}`,
  );

  const targets: LegacyWorkerTarget[] = plan
    .filter(({ classification }) => classification === 'migrated')
    .map(({ document }) => ({ document, spaceId: resolutions.get(document.siteId)?.spaceId ?? null }));
  const spaceIdByDocumentId = new Map(targets.map((target) => [target.document.documentId, target.spaceId ?? '']));

  const manifest: LegacyMigrationManifest = {
    spaces: [...resolutions]
      .filter(([, r]) => r.kind === 'created')
      .map(([siteId, r]) => ({ id: r.spaceId ?? '', siteId, slug: sitesById.get(siteId)?.siteSlug ?? '' })),
    publications: [],
    demoted: [],
  };
  const results: LegacyWorkerResult[] = [];
  let aborted = false;

  const flushManifest = () => writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
  flushManifest();

  console.log(`workers ${workerCount}, batch ${batchSize}, targets ${targets.length}`);

  await new Promise<void>((resolve, reject) => {
    let active = 0;
    const spawn = (shard: number): void => {
      const worker = new Worker(new URL('migrate-legacy-publications-worker.ts', import.meta.url), {
        workerData: {
          workerIndex: shard,
          dryRun,
          batch: batchSize,
          targets: targets.filter((target) => shardOf(target.document.documentId, workerCount) === shard),
        },
        execArgv: process.execArgv,
        env: { ...process.env, WASM_POOL_SIZE: '1', DB_POOL_MAX: '2' },
      });
      active += 1;

      worker.on('message', (message: LegacyWorkerMessage) => {
        if (message.type === 'fatal') {
          worker.postMessage({ done: true });
          reject(new Error(message.message));
          return;
        }
        if (message.type === 'exhausted') {
          worker.postMessage({ done: true });
          return;
        }
        for (const result of message.results) {
          results.push(result);
          if (result.outcome === 'migrated') {
            manifest.publications.push({
              id: result.publicationId,
              versionId: result.versionId,
              documentId: result.documentId,
              spaceId: spaceIdByDocumentId.get(result.documentId) ?? '',
              permalink: result.permalink,
              updatedAt: result.updatedAt,
            });
          }
        }
        if (!dryRun) flushManifest();
        process.stdout.write(`\rprocessed ${results.length}/${targets.length}   `);
      });
      worker.on('error', reject);
      worker.on('exit', () => {
        active -= 1;
        if (active === 0) resolve();
      });
    };
    for (let shard = 0; shard < workerCount; shard++) spawn(shard);
  }).catch((err) => {
    aborted = true;
    console.error('\nworker aborted, partial state written:', err);
  });
  process.stdout.write('\n');
  if (results.length !== targets.length) aborted = true;

  const runtimeEmpty = new Set(results.filter((result) => result.outcome === 'empty').map((result) => result.documentId));
  const demoteEntityIds = plan
    .filter(({ document, classification }) => isDemotedClassification(classification) || runtimeEmpty.has(document.documentId))
    .map(({ document }) => document.entityId);
  let demotedCount = 0;
  if (dryRun) demotedCount = demoteEntityIds.length;
  if (!dryRun && !aborted) {
    manifest.demoted = demoteEntityIds.map((entityId) => ({ entityId, from: 'PUBLIC' as const }));
    flushManifest();
    try {
      const demoted = await demoteLegacyEntities(demoteEntityIds);
      manifest.demoted = demoted.map((entityId) => ({ entityId, from: 'PUBLIC' as const }));
      demotedCount = demoted.length;
    } catch (err) {
      aborted = true;
      demotedCount = 0;
      console.error('\ndemotion aborted, manifest keeps the intended entity ids:', err);
    }
    flushManifest();
  }

  const failed = results.filter((result) => result.outcome === 'failed');
  const outcomeCounts: Record<string, number> = {};
  for (const result of results) outcomeCounts[result.outcome] = (outcomeCounts[result.outcome] ?? 0) + 1;
  const sortedMs = results.map((result) => result.ms).toSorted((a, b) => a - b);
  const graphBytes = results.reduce((sum, result) => sum + ('graphBytes' in result ? result.graphBytes : 0), 0);
  const report = {
    mode,
    aborted,
    documents: documents.length,
    plan: countBy(plan),
    spaces: resolutionCounts,
    slugTakenSites: [...slugTakenSiteIds].map((siteId) => ({ siteId, slug: sitesById.get(siteId)?.siteSlug })),
    outcomes: outcomeCounts,
    demoted: demotedCount,
    failed: failed.map((result) => ({ documentId: result.documentId, message: result.outcome === 'failed' ? result.message : '' })),
    timing: {
      p50: percentile(sortedMs, 0.5),
      p95: percentile(sortedMs, 0.95),
      max: sortedMs.at(-1) ?? 0,
      totalMs: sortedMs.reduce((a, b) => a + b, 0),
    },
    graphBytes,
  };
  writeFileSync(reportPath, JSON.stringify(report, null, 2));
  console.log(
    JSON.stringify(
      { outcomes: report.outcomes, demoted: report.demoted, failed: failed.length, timing: report.timing, graphBytes },
      null,
      2,
    ),
  );
  console.log(`plan ${planPath}\nmanifest ${manifestPath}\nreport ${reportPath}`);

  process.exitCode = aborted || failed.length > 0 ? 1 : 0;
}

await pg.end();
redis.disconnect();
