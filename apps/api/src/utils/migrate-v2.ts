import dayjs from 'dayjs';
import { eq } from 'drizzle-orm';
import { db, DocumentComments, DocumentCommentThreads, DocumentContents, DocumentStates, first, firstOrThrow } from '#/db/index.ts';
import { calculateBlobSizeFromAssetIds, countCharacters, extractAssetIdsFromPlainDoc, insertFreshV2Content } from '#/utils/entity.ts';
import {
  collectLegacyTextChars,
  collectPlainTextChars,
  convertLegacyDocumentJson,
  deriveExpectedTextFromPlain,
  firstTextDiff,
  plainStructureDiff,
  plainStructureEquals,
} from '#/utils/legacy-convert.ts';
import { wasm } from '#/utils/wasm.ts';
import { wasm as wasmFfi } from '#/utils/wasm-ffi.ts';
import type { LegacyDocumentJson } from '#/utils/legacy-convert.ts';

export type MigrateDocumentResult =
  | {
      status: 'migrated';
      documentId: string;
      applied: boolean;
      threadCount: number;
      commentCount: number;
      warnings: string[];
      timings?: Record<string, number>;
    }
  | { status: 'skipped'; documentId: string; reason: 'already-v2' }
  | { status: 'failed'; documentId: string; error: string };

export const migrateDocumentToV2 = async (
  documentId: string,
  options?: { dryRun?: boolean; skipExistingCheck?: boolean; profile?: boolean; snapshot?: Uint8Array | null },
): Promise<MigrateDocumentResult> => {
  let stage = 'load';
  const timings: Record<string, number> = {};
  let stageStart = performance.now();
  const mark = (next: string) => {
    timings[stage] = (timings[stage] ?? 0) + performance.now() - stageStart;
    stage = next;
    stageStart = performance.now();
  };
  try {
    if (!options?.skipExistingCheck) {
      const existing = await db
        .select({ documentId: DocumentStates.documentId })
        .from(DocumentStates)
        .where(eq(DocumentStates.documentId, documentId))
        .then(first);
      if (existing) return { status: 'skipped', documentId, reason: 'already-v2' };
    }

    let snapshot = options?.snapshot;
    if (snapshot === undefined) {
      const content = await db
        .select({ snapshot: DocumentContents.snapshot })
        .from(DocumentContents)
        .where(eq(DocumentContents.documentId, documentId))
        .then(first);
      snapshot = content?.snapshot ?? null;
    }
    if (snapshot === null) return { status: 'failed', documentId, error: 'no-legacy-content' };

    mark('snapshot-to-json');
    const legacyJson = (await wasm.snapshotToJson(snapshot)) as LegacyDocumentJson;
    mark('convert');
    const { plain, remarkAnchors, warnings } = convertLegacyDocumentJson(legacyJson);

    const { graph, anchors, heads, text, roundtrip } = await wasmFfi.use((host) => {
      mark('verify_plain');
      host.verify_plain(plain);
      mark('to_graph_with_anchors');
      const result = host.to_graph_with_anchors(plain, { paths: remarkAnchors.map((anchor) => anchor.path) });
      mark('heads');
      const headsBytes = host.heads(result.graph);
      mark('extract_text');
      const extracted = host.extract_text(plain);
      mark('to_plain-roundtrip');
      return {
        graph: result.graph,
        anchors: result.anchors,
        heads: headsBytes,
        text: extracted,
        roundtrip: host.to_plain(result.graph),
      };
    });
    mark('gates');

    if (!plainStructureEquals(plain, roundtrip)) {
      return { status: 'failed', documentId, error: `structure-mismatch: ${plainStructureDiff(plain, roundtrip).join(' | ')}` };
    }

    const expectedText = deriveExpectedTextFromPlain(plain);
    if (text !== expectedText) {
      return { status: 'failed', documentId, error: `text-mismatch: extract_text != expected, ${firstTextDiff(text, expectedText)}` };
    }

    const plainChars = collectPlainTextChars(plain);
    const legacyChars = collectLegacyTextChars(legacyJson);
    if (plainChars !== legacyChars) {
      return {
        status: 'failed',
        documentId,
        error: `text-mismatch: converter dropped or duplicated characters, ${firstTextDiff(plainChars, legacyChars)}`,
      };
    }

    if (anchors.length !== remarkAnchors.length) {
      return { status: 'failed', documentId, error: 'anchor-count-mismatch' };
    }

    const legacyAssets = collectLegacyAssetIds(legacyJson);
    const v2Assets = extractAssetIdsFromPlainDoc(plain);
    const v2AssetKey = assetKey(v2Assets);
    if (assetKey(legacyAssets) !== v2AssetKey) {
      return { status: 'failed', documentId, error: `asset-mismatch: legacy=${assetKey(legacyAssets)} v2=${v2AssetKey}` };
    }

    const commentCount = remarkAnchors.reduce((sum, anchor) => sum + anchor.remarks.length, 0);

    if (options?.dryRun) {
      mark('done');
      return {
        status: 'migrated',
        documentId,
        applied: false,
        threadCount: remarkAnchors.length,
        commentCount,
        warnings,
        ...(options.profile && { timings }),
      };
    }

    const characterCount = countCharacters(text);
    const blobSize = await calculateBlobSizeFromAssetIds(v2Assets.imageIds, v2Assets.fileIds);

    mark('transaction');
    await db.transaction(async (tx) => {
      await insertFreshV2Content(tx, documentId, { plain, graph, heads, text, characterCount, blobSize });

      for (const [index, anchor] of remarkAnchors.entries()) {
        const head = anchor.remarks[0];
        const tail = anchor.remarks.at(-1) ?? head;

        const thread = await tx
          .insert(DocumentCommentThreads)
          .values({
            documentId,
            userId: head.user_id,
            selection: anchors[index],
            createdAt: dayjs(head.created_at),
            updatedAt: dayjs(tail.created_at),
          })
          .returning({ id: DocumentCommentThreads.id })
          .then(firstOrThrow);

        for (const remark of anchor.remarks) {
          await tx.insert(DocumentComments).values({
            threadId: thread.id,
            userId: remark.user_id,
            content: remark.text,
            createdAt: dayjs(remark.created_at),
            updatedAt: dayjs(remark.created_at),
          });
        }
      }
    });

    mark('done');
    return {
      status: 'migrated',
      documentId,
      applied: true,
      threadCount: remarkAnchors.length,
      commentCount,
      warnings,
      ...(options?.profile && { timings }),
    };
  } catch (err) {
    const detail = err instanceof Error ? [err.message, ...(err.stack?.split('\n').slice(1, 3) ?? [])].join(' ').trim() : String(err);
    return { status: 'failed', documentId, error: `[${stage}] ${detail}` };
  }
};

const collectLegacyAssetIds = (json: LegacyDocumentJson) => {
  const imageIds: string[] = [];
  const fileIds: string[] = [];
  const embedIds: string[] = [];
  const archivedIds: string[] = [];
  const buckets: Record<string, string[]> = { image: imageIds, file: fileIds, embed: embedIds, archived: archivedIds };

  const walk = (nodeId: string) => {
    const entry = json.nodes[nodeId];
    if (!entry) return;
    const bucket = buckets[entry.type];
    if (bucket && typeof entry.id === 'string') bucket.push(entry.id);
    for (const childId of entry.children ?? []) walk(childId);
  };
  walk('0'.repeat(32));

  return { imageIds, fileIds, embedIds, archivedIds };
};

const assetKey = (assets: { imageIds: string[]; fileIds: string[]; embedIds: string[]; archivedIds: string[] }): string =>
  [assets.imageIds, assets.fileIds, assets.embedIds, assets.archivedIds]
    .map((ids) => ids.toSorted((a, b) => a.localeCompare(b)).join(','))
    .join('|');
