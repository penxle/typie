#!/usr/bin/env node

// Migrates every DocumentCommentThreads.selection (the only durable StablePosition
// store) from v1 to the v2 wire format, run inside the L3 maintenance window after
// readiness passes and before the L3 deploy (internal network):
//
//   SCRIPT=1 doppler run --config prod_local -- node --experimental-strip-types \
//     scripts/migrate-comment-anchors.ts [--yes] [--batch <n>] [--backup <path>]
//   SCRIPT=1 doppler run --config prod_local -- node --experimental-strip-types \
//     scripts/migrate-comment-anchors.ts --restore <backup>
//
// dry-run is the default; without --yes nothing is written. The migration is
// atomic: if any row is unrecognized or fails to resolve it aborts (exit 1) and
// writes nothing, so the operator resolves the surfaced rows before applying.

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { parseArgs } from 'node:util';
import { asc, eq } from 'drizzle-orm';
import { db, DocumentCommentThreads } from '#/db/index.ts';
import { readMergedGraph } from '#/utils/changeset.ts';
import { isV2Selection, normalizeStableSelection, normalizeStableSelectionForMigration } from '#/utils/comment-selection.ts';
import { wasm } from '#/utils/wasm-ffi.ts';

const { values } = parseArgs({
  options: {
    yes: { type: 'boolean', default: false },
    batch: { type: 'string', default: '200' },
    backup: { type: 'string', default: 'comment-anchor-backup.json' },
    restore: { type: 'string' },
  },
});

const batchSize = Number(values.batch);

type BackupEntry = { threadId: string; selection: unknown };

const loadAllThreads = async (): Promise<{ id: string; documentId: string; selection: unknown }[]> => {
  const rows: { id: string; documentId: string; selection: unknown }[] = [];
  for (;;) {
    const page = await db
      .select({ id: DocumentCommentThreads.id, documentId: DocumentCommentThreads.documentId, selection: DocumentCommentThreads.selection })
      .from(DocumentCommentThreads)
      .orderBy(asc(DocumentCommentThreads.documentId), asc(DocumentCommentThreads.id))
      .limit(batchSize)
      .offset(rows.length);
    if (page.length === 0) {
      break;
    }
    rows.push(...page);
    if (page.length < batchSize) {
      break;
    }
  }
  return rows;
};

// ── Restore mode ────────────────────────────────────────────────────────────
if (values.restore) {
  const path = values.restore;
  if (!existsSync(path)) {
    console.error(`backup not found: ${path}`);
    process.exit(1);
  }
  const backup: BackupEntry[] = JSON.parse(readFileSync(path, 'utf8'));
  console.log(`RESTORE: ${backup.length} threads from ${path}`);

  for (const { threadId, selection } of backup) {
    await db.update(DocumentCommentThreads).set({ selection }).where(eq(DocumentCommentThreads.id, threadId));
  }

  // Every restored row must read back as v1 (not the v2 envelope).
  const stillV2: string[] = [];
  for (const { threadId } of backup) {
    const [row] = await db
      .select({ selection: DocumentCommentThreads.selection })
      .from(DocumentCommentThreads)
      .where(eq(DocumentCommentThreads.id, threadId));
    if (row && isV2Selection(row.selection)) {
      stillV2.push(threadId);
    }
  }
  if (stillV2.length > 0) {
    console.error(`RESTORE FAILED: ${stillV2.length} threads still v2: ${stillV2.join(', ')}`);
    process.exit(1);
  }
  console.log(`RESTORE OK: ${backup.length} threads back to v1`);
  process.exit(0);
}

// ── Forward migration ─────────────────────────────────────────────────────────
const dryRun = !values.yes;
console.log(dryRun ? 'DRY RUN (apply with --yes)' : 'APPLY MODE');

const threads = await loadAllThreads();
console.log(`scanning ${threads.length} comment threads`);

type Migratable = { threadId: string; original: unknown; v2: unknown; degraded: boolean };
type Blocker = { threadId: string; documentId: string; reason: 'unrecognized' | 'unresolved'; message?: string };

const migratable: Migratable[] = [];
const blockers: Blocker[] = [];
let alreadyV2 = 0;

// One document's graph at a time — threads are ordered by document, so a single
// cached graph covers each document's whole run of comments.
let currentDocId = '';
let currentGraph: Uint8Array | null = null;

for (const thread of threads) {
  if (isV2Selection(thread.selection)) {
    alreadyV2 += 1;
    continue;
  }

  const normalized = normalizeStableSelectionForMigration(thread.selection);
  if (normalized === null) {
    blockers.push({ threadId: thread.id, documentId: thread.documentId, reason: 'unrecognized' });
    continue;
  }

  try {
    if (thread.documentId !== currentDocId || !currentGraph) {
      currentGraph = await readMergedGraph(thread.documentId);
      currentDocId = thread.documentId;
    }
    const graph = currentGraph;
    const resolved = await wasm.use((host) => host.resolve_v1_selection(graph, JSON.stringify(normalized)));
    // The wasm boundary emits a None child as `undefined`; the JSON round-trip
    // strips those keys (structuredClone would keep them), so normalize can
    // restore the explicit null the persisted DTO (and kotlinx decoders) require.
    // eslint-disable-next-line unicorn/prefer-structured-clone
    const v2 = normalizeStableSelection(JSON.parse(JSON.stringify(resolved.selection)));
    migratable.push({ threadId: thread.id, original: thread.selection, v2, degraded: resolved.degraded });
  } catch (err) {
    blockers.push({
      threadId: thread.id,
      documentId: thread.documentId,
      reason: 'unresolved',
      message: err instanceof Error ? err.message : String(err),
    });
  }
}

const degradedCount = migratable.filter((m) => m.degraded).length;
console.log(`already v2: ${alreadyV2}, migratable: ${migratable.length} (degraded: ${degradedCount}), blockers: ${blockers.length}`);

// Atomic refusal: any blocker aborts the whole migration before writing.
if (blockers.length > 0) {
  const blockerPath = 'comment-anchor-blockers.json';
  writeFileSync(blockerPath, JSON.stringify(blockers, null, 2));
  console.error(`ABORTING: ${blockers.length} unmigratable rows written to ${blockerPath} (resolve them and rerun)`);
  process.exit(1);
}

if (dryRun) {
  writeFileSync('comment-anchor-preview.json', JSON.stringify(migratable, null, 2));
  console.log(`dry run: ${migratable.length} threads ready (preview: comment-anchor-preview.json)`);
  process.exit(0);
}

// Back up originals before mutating so --restore can roll back.
const backup: BackupEntry[] = migratable.map((m) => ({ threadId: m.threadId, selection: m.original }));
writeFileSync(values.backup, JSON.stringify(backup, null, 2));
console.log(`backed up ${backup.length} originals to ${values.backup}`);

for (const { threadId, v2 } of migratable) {
  await db.update(DocumentCommentThreads).set({ selection: v2 }).where(eq(DocumentCommentThreads.id, threadId));
}

// Post-verify: every migrated row must read back as the v2 envelope.
const notV2: string[] = [];
for (const { threadId } of migratable) {
  const [row] = await db
    .select({ selection: DocumentCommentThreads.selection })
    .from(DocumentCommentThreads)
    .where(eq(DocumentCommentThreads.id, threadId));
  if (!row || !isV2Selection(row.selection)) {
    notV2.push(threadId);
  }
}
if (notV2.length > 0) {
  console.error(`POST-VERIFY FAILED: ${notV2.length} threads not v2: ${notV2.join(', ')} (restore with --restore ${values.backup})`);
  process.exit(1);
}

console.log(`APPLIED: ${migratable.length} threads migrated to v2 and verified`);
process.exit(0);
