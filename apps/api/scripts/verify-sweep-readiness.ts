#!/usr/bin/env node

// sweepDocument (#/utils/zombie-sweep.ts) statically imports #/mq/index.ts, which starts
// worker.run() at module-evaluation time unless SCRIPT is set (mq/bullmq.ts:71) — ESM evaluates
// imports before this file's body runs, so `process.env.SCRIPT = '1'` here would already be too
// late. Inject it before the process starts:
//
//   SCRIPT=1 doppler run --config prod_local -- node --experimental-strip-types \
//     scripts/verify-sweep-readiness.ts --http-canary <url> --ws-canary <url>
//
// This only runs once the edge/LB has already cut all public traffic — collect/consolidate/sweep
// workers and the API process stay up, only ingress is blocked. Steps 2-7 assume step 1
// confirmed the block; run this from inside the window, on the internal network, bypassing the
// LB.
//
// Pre-window canary (start BEFORE the block window opens, while traffic still flows, and leave
// it running — step 1 reads back whether the window's edge cut later force-closed it):
//
//   SCRIPT=1 doppler run --config prod_local -- node --experimental-strip-types \
//     scripts/verify-sweep-readiness.ts --keepalive-canary --ws-canary <url>
//
// Because the whole window is blocked, a document's stream tip cannot move, so the backfill's
// verified marker (recorded at `streamTip`-when-checked) still describes the current graph: any
// document whose marker equals its current tip is skipped from the final sweep and re-verification
// (carried-over verification, not an omission), and only unmarked/moved documents are re-checked.
// The heavy per-document work (final sweep, re-verification) runs across `--sweep-concurrency`
// worker threads, each with its own wasm instance.
//
// Exit 0: readiness confirmed — proceed with comment-anchor migration, then L3 deploy.
// Exit 1: block unconfirmed, or violating documents remain (see --violations output) — release
// the window without migrating or deploying L3 (no-op rollback).

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { setTimeout as sleep } from 'node:timers/promises';
import { parseArgs } from 'node:util';
import { Worker } from 'node:worker_threads';
import { asc, gt } from 'drizzle-orm';
import { WebSocket } from 'ws';
import { db, DocumentStates } from '#/db/index.ts';
import { getCollectedSeq, hasActivePresence, streamTip } from '#/utils/changeset.ts';
import { getSweepVerifiedMany, sweepVerifiedMatchesTip } from '#/utils/zombie-sweep.ts';
import type { SweepResult } from '#/utils/zombie-sweep.ts';

// Mirrors zombie-sweep.ts's own QUIESCENCE_MS (not exported) — the final sweep pass below is a
// no-op fence-defer until this holds anyway, but waiting here first avoids hammering it with
// deferred apply calls while the tip is still obviously too young.
const QUIESCENCE_MS = 10 * 60 * 1000;
const SIDECAR_CHECK_LIMIT = 1000;

const { values } = parseArgs({
  options: {
    'http-canary': { type: 'string' },
    'ws-canary': { type: 'string' },
    'keepalive-canary': { type: 'boolean', default: false },
    'canary-state': { type: 'string', default: '.sweep-readiness-canary-state.json' },
    'canary-timeout-ms': { type: 'string', default: '5000' },
    violations: { type: 'string', default: 'sweep-readiness-violations.json' },
    report: { type: 'string', default: 'sweep-readiness-report.json' },
    batch: { type: 'string', default: '500' },
    'poll-ms': { type: 'string', default: '15000' },
    'sweep-concurrency': { type: 'string', default: '4' },
    'check-concurrency': { type: 'string', default: '20' },
    'max-sweep-attempts': { type: 'string', default: '20' },
    'sweep-retry-ms': { type: 'string', default: '5000' },
  },
});

const httpCanaryUrl = values['http-canary'] ?? process.env.SWEEP_READINESS_HTTP_CANARY_URL;
const wsCanaryUrl = values['ws-canary'] ?? process.env.SWEEP_READINESS_WS_CANARY_URL;
const canaryStatePath = values['canary-state'] as string;
const canaryTimeoutMs = Number(values['canary-timeout-ms']);
const violationsPath = values.violations as string;
const reportPath = values.report as string;
const batchSize = Number(values.batch);
const pollMs = Number(values['poll-ms']);
// Redefined: the number of worker threads that share the residual final-sweep / re-verification.
const sweepConcurrency = Number(values['sweep-concurrency']);
const checkConcurrency = Number(values['check-concurrency']);
const maxSweepAttempts = Number(values['max-sweep-attempts']);
const sweepRetryMs = Number(values['sweep-retry-ms']);

type CanaryState = {
  url: string;
  openedAt: string;
  closedAt: string | null;
  closeCode?: number;
  closeReason?: string;
  error?: string;
};

const elapsed = (startMs: number): string => `${Math.round((Date.now() - startMs) / 1000)}s`;

const mapWithConcurrency = async <T, R>(items: readonly T[], concurrency: number, fn: (item: T) => Promise<R>): Promise<R[]> => {
  const results: R[] = Array.from({ length: items.length });
  let cursor = 0;
  const worker = async (): Promise<void> => {
    for (;;) {
      const index = cursor;
      cursor += 1;
      if (index >= items.length) return;
      results[index] = await fn(items[index]);
    }
  };
  await Promise.all(Array.from({ length: Math.max(1, Math.min(concurrency, items.length)) }, worker));
  return results;
};

const seqEqual = (a: string | null, b: string | null): boolean => a === b;

if (values['keepalive-canary']) {
  await runKeepaliveCanary();
} else {
  await main();
}

async function runKeepaliveCanary(): Promise<void> {
  if (!wsCanaryUrl) {
    console.error('Usage: --keepalive-canary requires --ws-canary <url> (or SWEEP_READINESS_WS_CANARY_URL)');
    process.exit(1);
  }

  const state: CanaryState = { url: wsCanaryUrl, openedAt: new Date().toISOString(), closedAt: null };
  writeFileSync(canaryStatePath, JSON.stringify(state, null, 2));
  console.log(`pre-window canary opening ${wsCanaryUrl} — leave this process running until the block window opens and cuts it`);

  const ws = new WebSocket(wsCanaryUrl);
  ws.on('open', () => console.log('pre-window canary open — waiting for the block window to close it'));
  ws.on('close', (code, reason) => finishKeepalive(state, { closeCode: code, closeReason: reason.toString() }));
  ws.on('error', (err) => finishKeepalive(state, { error: err.message }));
}

function finishKeepalive(state: CanaryState, extra: Partial<CanaryState>): void {
  state.closedAt = new Date().toISOString();
  Object.assign(state, extra);
  writeFileSync(canaryStatePath, JSON.stringify(state, null, 2));
  console.log(`pre-window canary ended — state written to ${canaryStatePath}`);
  process.exit(0);
}

async function checkHttpCanaryRejected(url: string): Promise<boolean> {
  try {
    await fetch(url, { signal: AbortSignal.timeout(canaryTimeoutMs) });
    // Any completed HTTP response — success or error status — means the edge let the request
    // through; a true block drops/refuses the connection, which throws instead.
    return false;
  } catch {
    return true;
  }
}

function checkWsCanaryRejected(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    const ws = new WebSocket(url);
    // No response within the timeout counts as rejected, same as the HTTP canary's catch: a
    // packet-drop-style block (the common real implementation) never fires 'error', it just lets
    // the TCP connect hang past any JS timer, so treating a timeout as "reachable" would make
    // this gate fail forever under that block style.
    const timer = setTimeout(() => {
      ws.terminate();
      resolve(true);
    }, canaryTimeoutMs);
    ws.once('open', () => {
      clearTimeout(timer);
      ws.terminate();
      resolve(false);
    });
    ws.once('error', () => {
      clearTimeout(timer);
      ws.terminate();
      resolve(true);
    });
    ws.once('unexpected-response', () => {
      clearTimeout(timer);
      ws.terminate();
      resolve(true);
    });
  });
}

function checkPreWindowCanaryClosed(): { ok: boolean; detail: string } {
  if (!existsSync(canaryStatePath)) {
    return { ok: false, detail: `no state file at ${canaryStatePath} — run with --keepalive-canary before the window opens` };
  }
  const state = JSON.parse(readFileSync(canaryStatePath, 'utf8')) as CanaryState;
  if (state.url !== wsCanaryUrl) {
    return { ok: false, detail: `state file targets a different URL (${state.url})` };
  }
  if (!state.closedAt) {
    return { ok: false, detail: `socket opened at ${state.openedAt} has not closed yet` };
  }
  return { ok: true, detail: `closed at ${state.closedAt} (opened ${state.openedAt})` };
}

async function confirmBlock(): Promise<void> {
  if (!httpCanaryUrl || !wsCanaryUrl) {
    console.error('--http-canary and --ws-canary (or SWEEP_READINESS_HTTP_CANARY_URL / SWEEP_READINESS_WS_CANARY_URL) are required.');
    process.exit(1);
  }

  const [httpRejected, wsRejected] = await Promise.all([checkHttpCanaryRejected(httpCanaryUrl), checkWsCanaryRejected(wsCanaryUrl)]);
  const preWindow = checkPreWindowCanaryClosed();

  console.log(`  http canary (${httpCanaryUrl}): ${httpRejected ? 'rejected (ok)' : 'REACHABLE — block not confirmed'}`);
  console.log(`  ws canary (${wsCanaryUrl}): ${wsRejected ? 'rejected (ok)' : 'REACHABLE — block not confirmed'}`);
  console.log(`  pre-window canary: ${preWindow.ok ? preWindow.detail : `NOT CONFIRMED — ${preWindow.detail}`}`);

  if (!httpRejected || !wsRejected || !preWindow.ok) {
    console.error('Block not confirmed — aborting before any further checks (they would be meaningless without it).');
    process.exit(1);
  }

  console.log('block confirmed.');
}

async function loadDocumentIds(): Promise<string[]> {
  const ids: string[] = [];
  let cursor = '';
  for (;;) {
    const rows = await db
      .select({ documentId: DocumentStates.documentId })
      .from(DocumentStates)
      .where(cursor ? gt(DocumentStates.documentId, cursor) : undefined)
      .orderBy(asc(DocumentStates.documentId))
      .limit(batchSize);
    if (rows.length === 0) break;
    for (const { documentId } of rows) {
      ids.push(documentId);
      cursor = documentId;
    }
  }
  return ids;
}

type VerifiedPartition = { skipped: string[]; residual: string[] };

// Skip a document whose verified marker still equals its current (block-frozen) tip: an
// unmoved tip is an unchanged graph, so the backfill's zombie-free judgement carries over
// and only unmarked/moved documents need the wasm-heavy re-check.
async function partitionByVerified(documentIds: string[]): Promise<VerifiedPartition> {
  const skipped: string[] = [];
  const residual: string[] = [];
  const WINDOW = 1000;
  for (let start = 0; start < documentIds.length; start += WINDOW) {
    const slice = documentIds.slice(start, start + WINDOW);
    const [markers, tips] = await Promise.all([
      getSweepVerifiedMany(slice),
      mapWithConcurrency(slice, checkConcurrency, (id) => streamTip(id)),
    ]);
    for (const [i, element] of slice.entries()) {
      if (sweepVerifiedMatchesTip(markers[i], tips[i])) skipped.push(element);
      else residual.push(element);
    }
  }
  return { skipped, residual };
}

async function waitForPresenceDecay(documentIds: string[]): Promise<void> {
  const start = Date.now();
  for (;;) {
    const active = await mapWithConcurrency(documentIds, checkConcurrency, async (documentId) =>
      (await hasActivePresence(documentId)) ? documentId : null,
    );
    const pending = active.filter((id): id is string => id !== null);
    if (pending.length === 0) {
      console.log(`  presence decayed for all ${documentIds.length} documents (${elapsed(start)})`);
      return;
    }
    console.log(`  waiting on presence lease decay: ${pending.length}/${documentIds.length} still active (${elapsed(start)})`);
    await sleep(pollMs);
  }
}

async function drainCollect(documentIds: string[], label: string): Promise<void> {
  const start = Date.now();
  for (;;) {
    const pending = await mapWithConcurrency(documentIds, checkConcurrency, async (documentId) => {
      const [collected, tip] = await Promise.all([getCollectedSeq(documentId), streamTip(documentId)]);
      return seqEqual(collected, tip) ? null : documentId;
    });
    const unresolved = pending.filter((id): id is string => id !== null);
    if (unresolved.length === 0) {
      console.log(`  [${label}] collected caught up to tip for all ${documentIds.length} documents (${elapsed(start)})`);
      return;
    }
    // collect folds at most 5 stream entries per run (readStreamBatch cap in changeset.ts), so a
    // deep backlog needs several passes of the already-running collect worker/cron — just poll.
    console.log(`  [${label}] waiting on collect: ${unresolved.length}/${documentIds.length} pending (${elapsed(start)})`);
    await sleep(pollMs);
  }
}

async function waitForQuiescence(documentIds: string[]): Promise<void> {
  const start = Date.now();
  for (;;) {
    const remainders = await mapWithConcurrency(documentIds, checkConcurrency, async (documentId) => {
      const tip = await streamTip(documentId);
      if (tip === null) return null;
      const age = Date.now() - Number(tip.split('-')[0]);
      return age >= QUIESCENCE_MS ? null : QUIESCENCE_MS - age;
    });
    const pending = remainders.filter((r): r is number => r !== null);
    if (pending.length === 0) {
      console.log(`  all ${documentIds.length} documents past ${QUIESCENCE_MS / 60_000}min tip quiescence (${elapsed(start)})`);
      return;
    }
    const maxRemaining = Math.max(...pending);
    console.log(
      `  waiting on tip quiescence: ${pending.length}/${documentIds.length} pending, next ready in ~${Math.round(maxRemaining / 1000)}s (${elapsed(start)})`,
    );
    await sleep(Math.min(pollMs, maxRemaining + 1000));
  }
}

type FinalSweepResult = { documentId: string; result: SweepResult | null };
type ReverifyRow = { documentId: string; drained: boolean; zombieCount: number; unconfirmedSidecar: number };

async function runWorkerPass<R>(documentIds: string[], phase: 'final-sweep' | 'reverify'): Promise<R[]> {
  if (documentIds.length === 0) return [];

  const results: R[] = Array.from({ length: documentIds.length });
  let cursor = 0;
  let processed = 0;

  await new Promise<void>((resolve, reject) => {
    let active = 0;

    const spawn = (): void => {
      const worker = new Worker(new URL('verify-sweep-readiness-worker.ts', import.meta.url), {
        workerData: { phase, maxSweepAttempts, sweepRetryMs, sidecarCheckLimit: SIDECAR_CHECK_LIMIT },
        execArgv: process.execArgv,
        env: { ...process.env, WASM_POOL_SIZE: '1', DB_POOL_MAX: '2' },
      });
      active += 1;

      worker.on('message', (message: null | { index: number; result: R } | { fatal: string }) => {
        if (message && 'fatal' in message) {
          reject(new Error(`readiness worker (${phase}): ${message.fatal}`));
          return;
        }
        if (message) {
          results[message.index] = message.result;
          processed += 1;
          if (processed % 100 === 0 || processed === documentIds.length) {
            console.log(`  [${phase}] ${processed}/${documentIds.length} processed`);
          }
        }
        if (cursor < documentIds.length) {
          const index = cursor;
          cursor += 1;
          worker.postMessage({ index, documentId: documentIds[index] });
        } else {
          worker.postMessage({ done: true });
        }
      });

      worker.on('error', reject);
      worker.on('exit', () => {
        active -= 1;
        if (active === 0) resolve();
      });
    };

    for (let i = 0; i < Math.min(sweepConcurrency, documentIds.length); i++) spawn();
  }).catch((err: unknown) => {
    // Fail closed: a worker crash mid-pass cannot become a PASS, so record the abort and stop
    // before any migration/L3 deploy (mirrors the aborted-flag + partial-report exit convention).
    const summary = {
      finishedAt: new Date().toISOString(),
      phase,
      aborted: true,
      pass: false,
      error: err instanceof Error ? err.message : String(err),
    };
    writeFileSync(reportPath, JSON.stringify(summary, null, 2));
    console.error(`\n[${phase}] 워커 비정상 종료 — fail-closed (migration/L3 중단):`, err);
    process.exit(1);
  });

  return results;
}

type FinalSweepOutcome = { unresolved: string[]; applied: number; zombiesSwept: number };

async function finalSweepPass(documentIds: string[]): Promise<FinalSweepOutcome> {
  const rows = await runWorkerPass<FinalSweepResult>(documentIds, 'final-sweep');

  const unresolved = rows.filter((r) => r.result === null || r.result.deferred).map((r) => r.documentId);
  const applied = rows.filter((r) => r.result?.applied).length;
  const zombiesSwept = rows.reduce((sum, r) => sum + (r.result?.zombieDots.length ?? 0), 0);

  console.log(`  final sweep: applied=${applied} zombiesSwept=${zombiesSwept} unresolved=${unresolved.length}`);
  return { unresolved, applied, zombiesSwept };
}

async function reverify(documentIds: string[]): Promise<ReverifyRow[]> {
  return runWorkerPass<ReverifyRow>(documentIds, 'reverify');
}

type Violation = { documentId: string; reasons: string[] };

function writeReport(
  documentIds: string[],
  skipped: string[],
  residual: string[],
  finalSweep: FinalSweepOutcome,
  reverifyRows: ReverifyRow[],
): void {
  const violationsByDoc = new Map<string, string[]>();
  const addViolation = (documentId: string, reason: string): void => {
    const reasons = violationsByDoc.get(documentId) ?? [];
    reasons.push(reason);
    violationsByDoc.set(documentId, reasons);
  };

  for (const documentId of finalSweep.unresolved) {
    addViolation(documentId, 'final-sweep-unresolved');
  }
  for (const row of reverifyRows) {
    if (!row.drained) addViolation(row.documentId, 'collected-not-equal-tip');
    if (row.zombieCount > 0) addViolation(row.documentId, 'dry-run-zombies-remaining');
    if (row.unconfirmedSidecar > 0) addViolation(row.documentId, 'unconfirmed-sweep-sidecar');
  }

  const violations: Violation[] = [];
  for (const [documentId, reasons] of violationsByDoc) {
    violations.push({ documentId, reasons });
  }
  writeFileSync(violationsPath, JSON.stringify(violations, null, 2));

  const summary = {
    finishedAt: new Date().toISOString(),
    documentCount: documentIds.length,
    skippedCount: skipped.length,
    residualCount: residual.length,
    finalSweep: { applied: finalSweep.applied, zombiesSwept: finalSweep.zombiesSwept, unresolvedCount: finalSweep.unresolved.length },
    violationCount: violations.length,
    pass: violations.length === 0,
  };
  writeFileSync(reportPath, JSON.stringify(summary, null, 2));
  console.log(JSON.stringify(summary, null, 2));

  if (violations.length > 0) {
    console.error(
      `FAIL — ${violations.length} document(s) violate readiness. See ${violationsPath}. Release the window without migrating or deploying L3.`,
    );
    process.exit(1);
  }

  console.log(
    `PASS — ${documentIds.length} documents clean (${skipped.length} 이월 검증 / ${residual.length} 실검사). Proceed with comment-anchor migration, then L3 deploy.`,
  );
  process.exit(0);
}

async function main(): Promise<void> {
  console.log('=== Step 1: block confirmation ===');
  await confirmBlock();

  console.log('=== Loading document set ===');
  const documentIds = await loadDocumentIds();
  console.log(`  documents: ${documentIds.length}`);

  console.log('=== Verified-marker skip filter ===');
  const { skipped, residual } = await partitionByVerified(documentIds);
  console.log(`  skip (이월된 검증) ${skipped.length} / 실검사 ${residual.length}`);

  console.log('=== Step 2: presence lease decay ===');
  await waitForPresenceDecay(documentIds);

  console.log('=== Step 3: collect drain (1st) ===');
  await drainCollect(documentIds, 'drain-1');

  console.log('=== Step 4: tip quiescence wait + final sweep ===');
  await waitForQuiescence(documentIds);
  const finalSweep = await finalSweepPass(residual);

  console.log('=== Step 5: collect drain (2nd) ===');
  await drainCollect(documentIds, 'drain-2');

  console.log('=== Step 6: full re-verification ===');
  const reverifyRows = await reverify(residual);

  console.log('=== Step 7: report ===');
  writeReport(documentIds, skipped, residual, finalSweep, reverifyRows);
}
