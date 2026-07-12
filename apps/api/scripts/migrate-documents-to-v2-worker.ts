#!/usr/bin/env node

import { parentPort, workerData } from 'node:worker_threads';
import { inArray } from 'drizzle-orm';
import { db, DocumentContents } from '#/db/index.ts';
import { migrateDocumentToV2 } from '#/utils/migrate-v2.ts';
import type { MigrateDocumentResult } from '#/utils/migrate-v2.ts';

process.env.SCRIPT = '1';

const { dryRun, profile, skipExistingCheck } = workerData as { dryRun: boolean; profile: boolean; skipExistingCheck: boolean };

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const port = parentPort!;

port.on('message', async (message: { ids: string[] } | { done: true }) => {
  if ('done' in message) {
    process.exit(0);
  }

  const rows = await db
    .select({ documentId: DocumentContents.documentId, snapshot: DocumentContents.snapshot })
    .from(DocumentContents)
    .where(inArray(DocumentContents.documentId, message.ids));
  const snapshots = new Map(rows.map((row) => [row.documentId, row.snapshot]));

  const results: MigrateDocumentResult[] = [];
  for (const id of message.ids) {
    results.push(await migrateDocumentToV2(id, { dryRun, profile, skipExistingCheck, snapshot: snapshots.get(id) ?? null }));
  }
  port.postMessage(results);
});

port.postMessage([]);
