#!/usr/bin/env node

import { parentPort, workerData } from 'node:worker_threads';
import type { LegacyMigrationOutcome } from '#/utils/legacy-publication.ts';
import type { LegacyPublicDocument } from '#/utils/legacy-publication-core.ts';

process.env.SCRIPT = '1';

const { migrateLegacyDocument } = await import('#/utils/legacy-publication.ts');

type LegacyWorkerTarget = { document: LegacyPublicDocument; spaceId: string | null };
type LegacyWorkerResult = { documentId: string; entityId: string; siteId: string; ms: number } & LegacyMigrationOutcome;

type WorkerData = { workerIndex: number; dryRun: boolean; batch: number; targets: LegacyWorkerTarget[] };

const { workerIndex, dryRun, batch, targets } = workerData as WorkerData;

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const port = parentPort!;

const run = async (): Promise<void> => {
  let results: LegacyWorkerResult[] = [];
  for (const target of targets) {
    const started = performance.now();
    const outcome =
      dryRun || target.spaceId === null
        ? await migrateLegacyDocument(target.document, { dryRun: true })
        : await migrateLegacyDocument(target.document, { dryRun: false, spaceId: target.spaceId });
    results.push({
      documentId: target.document.documentId,
      entityId: target.document.entityId,
      siteId: target.document.siteId,
      ms: Math.round(performance.now() - started),
      ...outcome,
    });
    if (results.length >= batch) {
      port.postMessage({ type: 'result', results });
      results = [];
    }
  }
  if (results.length > 0) {
    port.postMessage({ type: 'result', results });
  }
  port.postMessage({ type: 'exhausted' });
};

port.on('message', (message: { done?: boolean }) => {
  if (message.done) {
    process.exit(0);
  }
});

try {
  await run();
} catch (err) {
  port.postMessage({ type: 'fatal', message: `worker ${workerIndex}: ${err instanceof Error ? err.message : String(err)}` });
}
