#!/usr/bin/env node

import { setTimeout as sleep } from 'node:timers/promises';
import { parentPort, workerData } from 'node:worker_threads';
import { getCollectedSeq, readStreamBatch, streamTip } from '#/utils/changeset.ts';
import { sweepDocument } from '#/utils/zombie-sweep.ts';
import type { SweepResult } from '#/utils/zombie-sweep.ts';

process.env.SCRIPT = '1';

type WorkerData = {
  phase: 'final-sweep' | 'reverify';
  maxSweepAttempts: number;
  sweepRetryMs: number;
  sidecarCheckLimit: number;
};

const { phase, maxSweepAttempts, sweepRetryMs, sidecarCheckLimit } = workerData as WorkerData;

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const port = parentPort!;

type FinalSweepResult = { documentId: string; result: SweepResult | null };
type ReverifyResult = { documentId: string; drained: boolean; zombieCount: number; unconfirmedSidecar: number };

const finalSweepOne = async (documentId: string): Promise<FinalSweepResult> => {
  let last: SweepResult | null = null;
  for (let attempt = 1; attempt <= maxSweepAttempts; attempt++) {
    last = await sweepDocument(documentId, { dryRun: false });
    if (!last.deferred) break;
    if (attempt < maxSweepAttempts) await sleep(sweepRetryMs);
  }
  return { documentId, result: last };
};

const reverifyOne = async (documentId: string): Promise<ReverifyResult> => {
  const [collected, tip, dry] = await Promise.all([
    getCollectedSeq(documentId),
    streamTip(documentId),
    sweepDocument(documentId, { dryRun: true }),
  ]);
  const pending = await readStreamBatch(documentId, collected, sidecarCheckLimit);
  const unconfirmedSidecar = pending.filter((entry) => entry.zombieDots && entry.zombieDots.length > 0).length;
  return { documentId, drained: collected === tip, zombieCount: dry.zombieDots.length, unconfirmedSidecar };
};

port.on('message', async (message: { done: true } | { index: number; documentId: string }) => {
  if ('done' in message) {
    process.exit(0);
  }
  try {
    const result = phase === 'final-sweep' ? await finalSweepOne(message.documentId) : await reverifyOne(message.documentId);
    port.postMessage({ index: message.index, result });
  } catch (err) {
    port.postMessage({ fatal: err instanceof Error ? err.message : String(err) });
  }
});

port.postMessage(null);
