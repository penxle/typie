#!/usr/bin/env node

import { parentPort } from 'node:worker_threads';
import { backfillFont } from '#/utils/backfill-fonts.ts';
import type { BackfillResult, BackfillTarget } from '#/utils/backfill-fonts.ts';

process.env.SCRIPT = '1';

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const port = parentPort!;

port.on('message', async (message: { target: BackfillTarget } | { done: true }) => {
  if ('done' in message) {
    process.exit(0);
  }

  const { row } = message.target;
  const result: BackfillResult = { id: row.id, path: row.path, error: null };
  try {
    await backfillFont(message.target);
  } catch (err) {
    result.error = err instanceof Error ? err.message : String(err);
  }
  port.postMessage(result);
});

port.postMessage(null);
