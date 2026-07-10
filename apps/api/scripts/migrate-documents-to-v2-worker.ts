#!/usr/bin/env node

import { parentPort, workerData } from 'node:worker_threads';
import { migrateDocumentToV2 } from '#/utils/migrate-v2.ts';
import type { MigrateDocumentResult } from '#/utils/migrate-v2.ts';

process.env.SCRIPT = '1';

const { dryRun } = workerData as { dryRun: boolean };

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const port = parentPort!;

port.on('message', async (message: { ids: string[] } | { done: true }) => {
  if ('done' in message) {
    process.exit(0);
  }

  const results: MigrateDocumentResult[] = [];
  for (const id of message.ids) {
    results.push(await migrateDocumentToV2(id, { dryRun }));
  }
  port.postMessage(results);
});

port.postMessage([]);
