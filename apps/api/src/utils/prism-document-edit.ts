import { logger } from '@typie/lib';
import { toolFailure } from '@typie/prism';
import { eq } from 'drizzle-orm';
import { db, DocumentSweeps } from '#/db/index.ts';
import { prism, PrismApiError } from '#/external/prism.ts';
import { readMergedGraph } from './changeset.ts';
import { publishBundle } from './document-bundle.ts';
import {
  AT_MESSAGE,
  changedOf,
  documentIdOf,
  documentPath,
  EDIT_TOO_LARGE_MESSAGE,
  EditDocumentInput,
  FULL_TOO_LARGE_MESSAGE,
  messageOf,
  NO_FILE_MESSAGE,
  OpenDocumentInput,
  opErrorMessage,
  OUTLINE_DEFAULT_LIMIT,
  OUTLINE_FULL_MAX_CHARS,
  OutlineDocumentInput,
  renderAffected,
  renderOutline,
  REWRITE_FAILED_MESSAGE,
  SAVE_TOO_LARGE_MESSAGE,
  SaveDocumentInput,
  TARGETS_MESSAGE,
  TOO_LARGE_MESSAGE,
  toRustOps,
} from './prism-document-edit-core.ts';
import { DOCUMENT_FORMAT_GUIDE, DOCUMENT_FORMAT_PATH } from './prism-document-format.ts';
import { ERROR_MESSAGE } from './prism-tool-messages.ts';
import { documentRefOf, NOT_FOUND_DOCUMENT } from './prism-workspace.ts';
import { ensurePrismActor, PRISM_DEVICE_ID, PRISM_USER_ID } from './system-actor.ts';
import { wasm as wasmFfi } from './wasm-ffi.ts';
import type { ToolFailure } from '@typie/prism';
import type { PrismToolContext, PrismToolHandler, PrismToolPreflight } from './prism-tools.ts';

const log = logger.getChild('prism-document-edit');

const sweepTombstonesOf = async (documentId: string): Promise<string[]> => {
  const rows = await db
    .select({ zombieDots: DocumentSweeps.zombieDots })
    .from(DocumentSweeps)
    .where(eq(DocumentSweeps.documentId, documentId));

  return [...new Set(rows.flatMap((row) => row.zombieDots))];
};

const fileErrorMessage = (err: PrismApiError, tooLargeMessage: string, fallbackMessage: string, documentId: string): string => {
  if (err.status === 409 && err.code === 'file-too-large') return tooLargeMessage;

  log.warn('prism agent file access failed: {documentId} {status} {code}', {
    documentId,
    status: err.status,
    code: err.code,
  });

  return fallbackMessage;
};

const agentFileOf = async (
  ctx: PrismToolContext,
  path: string,
  documentId: string,
  tooLargeMessage: string,
): Promise<string | ToolFailure> => {
  let xml: string | null;
  try {
    xml = await prism.getAgentFile(ctx.session.prismAgentId, path);
  } catch (err) {
    if (!(err instanceof PrismApiError)) throw err;
    return toolFailure('error', fileErrorMessage(err, tooLargeMessage, ERROR_MESSAGE, documentId));
  }
  if (xml === null) return toolFailure('error', NO_FILE_MESSAGE);

  return xml;
};

const openDocument: PrismToolHandler = async (ctx, input) => {
  const parsed = OpenDocumentInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const document = await documentRefOf(ctx, parsed.data.id);
  if (document === null) return toolFailure('error', NOT_FOUND_DOCUMENT);

  const started = performance.now();
  const graph = await readMergedGraph(document.documentId);
  const sweep = await sweepTombstonesOf(document.documentId);
  const wasmStarted = performance.now();
  const rendered = await wasmFfi.use((host) => host.to_xml(graph, sweep));
  const wasmMs = Math.round(performance.now() - wasmStarted);
  if (rendered.error) {
    log.info('open-document refused: {documentId} {sessionId} {detail}', {
      documentId: document.documentId,
      sessionId: ctx.session.id,
      detail: rendered.error.detail,
    });
    return toolFailure('error', messageOf(rendered.error));
  }

  const path = documentPath(document.documentId);
  try {
    await prism.writeAgentFiles(ctx.session.prismAgentId, [
      { path: DOCUMENT_FORMAT_PATH, content: DOCUMENT_FORMAT_GUIDE },
      { path, content: rendered.xml },
    ]);
  } catch (err) {
    if (!(err instanceof PrismApiError)) throw err;
    return toolFailure('error', fileErrorMessage(err, TOO_LARGE_MESSAGE, ERROR_MESSAGE, document.documentId));
  }

  log.info('open-document: {documentId} {sessionId} {bytes} {wasmMs} {totalMs}', {
    documentId: document.documentId,
    sessionId: ctx.session.id,
    bytes: Buffer.byteLength(rendered.xml),
    wasmMs,
    totalMs: Math.round(performance.now() - started),
  });

  return { ok: true, document, path };
};

export type SaveTarget = {
  document: NonNullable<Awaited<ReturnType<typeof documentRefOf>>>;
  path: string;
  summary: string;
  xml: string;
};

export const saveTargetOf = async (ctx: PrismToolContext, input: unknown): Promise<SaveTarget | ToolFailure> => {
  const parsed = SaveDocumentInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const documentId = documentIdOf(parsed.data.path);
  const document = documentId === null ? null : await documentRefOf(ctx, documentId);
  if (document === null) return toolFailure('error', NOT_FOUND_DOCUMENT);

  const xml = await agentFileOf(ctx, parsed.data.path, document.documentId, SAVE_TOO_LARGE_MESSAGE);
  if (typeof xml !== 'string') return xml;

  return { document, path: parsed.data.path, summary: parsed.data.summary, xml };
};

const saveDocument: PrismToolHandler = async (ctx, input) => {
  const target = await saveTargetOf(ctx, input);
  if ('ok' in target) return target;
  const { document, path, xml } = target;

  const started = performance.now();
  const graph = await readMergedGraph(document.documentId);
  const sweep = await sweepTombstonesOf(document.documentId);
  const wasmStarted = performance.now();
  const result = await wasmFfi.use((host) => host.edit_from_xml(graph, sweep, xml));
  const wasmMs = Math.round(performance.now() - wasmStarted);
  if (result.error) {
    log.info('save-document refused: {documentId} {sessionId} {detail}', {
      documentId: document.documentId,
      sessionId: ctx.session.id,
      detail: result.error.detail,
    });
    return toolFailure('error', messageOf(result.error));
  }

  const changed = changedOf(result);
  const unchanged = result.bundle.length === 0;
  if (!unchanged) {
    if (result.xml === '') {
      log.error('save-document produced no file: {documentId} {sessionId}', {
        documentId: document.documentId,
        sessionId: ctx.session.id,
      });
      return toolFailure('error', REWRITE_FAILED_MESSAGE);
    }

    await ensurePrismActor();
    try {
      await prism.writeAgentFiles(ctx.session.prismAgentId, [{ path, content: result.xml }]);
    } catch (err) {
      if (!(err instanceof PrismApiError)) throw err;
      return toolFailure('error', fileErrorMessage(err, SAVE_TOO_LARGE_MESSAGE, REWRITE_FAILED_MESSAGE, document.documentId));
    }

    await publishBundle(document.documentId, result.bundle, PRISM_USER_ID, PRISM_DEVICE_ID);
  }

  log.info('save-document: {documentId} {sessionId} {bytes} {wasmMs} {totalMs} {unchanged} {*}', {
    documentId: document.documentId,
    sessionId: ctx.session.id,
    bytes: Buffer.byteLength(xml),
    wasmMs,
    totalMs: Math.round(performance.now() - started),
    unchanged,
    changed,
  });

  return { ok: true, unchanged, document, path, changed };
};

const outlineDocument: PrismToolHandler = async (ctx, input) => {
  const parsed = OutlineDocumentInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const documentId = documentIdOf(parsed.data.path);
  const document = documentId === null ? null : await documentRefOf(ctx, documentId);
  if (document === null) return toolFailure('error', NOT_FOUND_DOCUMENT);

  const xml = await agentFileOf(ctx, parsed.data.path, document.documentId, TOO_LARGE_MESSAGE);
  if (typeof xml !== 'string') return xml;

  const { under = 'root', depth = 1, offset = 0, limit = OUTLINE_DEFAULT_LIMIT, full = false } = parsed.data;
  const started = performance.now();
  const outline = await wasmFfi.use((host) => host.outline_xml(xml, under, depth, offset, limit, full));
  const wasmMs = Math.round(performance.now() - started);
  if (outline.error) {
    log.info('outline-document refused: {documentId} {sessionId} {detail}', {
      documentId: document.documentId,
      sessionId: ctx.session.id,
      detail: outline.error.detail,
    });
    return toolFailure('error', messageOf(outline.error));
  }

  if (outline.xml !== undefined && outline.xml.length > OUTLINE_FULL_MAX_CHARS) {
    log.info('outline-document full refused: {documentId} {sessionId} {chars}', {
      documentId: document.documentId,
      sessionId: ctx.session.id,
      chars: outline.xml.length,
    });
    return toolFailure('error', FULL_TOO_LARGE_MESSAGE);
  }

  log.info('outline-document: {documentId} {sessionId} {rows} {total} {full} {wasmMs}', {
    documentId: document.documentId,
    sessionId: ctx.session.id,
    rows: outline.rows.length,
    total: outline.total,
    full,
    wasmMs,
  });

  return { ok: true, document, path: parsed.data.path, text: renderOutline(outline, offset) };
};

const editDocument: PrismToolHandler = async (ctx, input) => {
  const parsed = EditDocumentInput.safeParse(input);
  if (!parsed.success) {
    const at = parsed.error.issues.some((issue) => issue.message === AT_MESSAGE || issue.path.includes('at'));
    if (at) return toolFailure('error', AT_MESSAGE);
    const targets = parsed.error.issues.some((issue) => issue.message === TARGETS_MESSAGE);
    return toolFailure('error', targets ? TARGETS_MESSAGE : ERROR_MESSAGE);
  }

  const documentId = documentIdOf(parsed.data.path);
  const document = documentId === null ? null : await documentRefOf(ctx, documentId);
  if (document === null) return toolFailure('error', NOT_FOUND_DOCUMENT);

  const xml = await agentFileOf(ctx, parsed.data.path, document.documentId, TOO_LARGE_MESSAGE);
  if (typeof xml !== 'string') return xml;

  const started = performance.now();
  const ops = JSON.stringify(toRustOps(parsed.data.ops));
  const result = await wasmFfi.use((host) => host.edit_xml(xml, ops));
  const wasmMs = Math.round(performance.now() - started);
  if (result.error) {
    log.info('edit-document refused: {documentId} {sessionId} {op} {address} {detail}', {
      documentId: document.documentId,
      sessionId: ctx.session.id,
      op: result.error.op ?? null,
      address: result.error.address ?? null,
      detail: result.error.info.detail,
    });
    return toolFailure('error', opErrorMessage(result.error));
  }

  try {
    await prism.writeAgentFiles(ctx.session.prismAgentId, [{ path: parsed.data.path, content: result.xml }]);
  } catch (err) {
    if (!(err instanceof PrismApiError)) throw err;
    return toolFailure('error', fileErrorMessage(err, EDIT_TOO_LARGE_MESSAGE, ERROR_MESSAGE, document.documentId));
  }

  log.info('edit-document: {documentId} {sessionId} {ops} {bytes} {wasmMs} {totalMs}', {
    documentId: document.documentId,
    sessionId: ctx.session.id,
    ops: parsed.data.ops.length,
    bytes: Buffer.byteLength(result.xml),
    wasmMs,
    totalMs: Math.round(performance.now() - started),
  });

  return {
    ok: true,
    document,
    path: parsed.data.path,
    applied: parsed.data.ops.length,
    text: renderAffected(parsed.data.ops.length, result.affected),
  };
};

export const documentEditTools: Record<string, PrismToolHandler> = {
  'open-document': openDocument,
  'outline-document': outlineDocument,
  'edit-document': editDocument,
  'save-document': saveDocument,
};

const verifyFailureOf = async (target: SaveTarget, sessionId: string): Promise<ToolFailure | null> => {
  const started = performance.now();
  const verdict = await wasmFfi.use((host) => host.verify_xml(target.xml));
  const wasmMs = Math.round(performance.now() - started);
  const bytes = Buffer.byteLength(target.xml);

  if (verdict.error) {
    log.info('save-document preverify refused: {documentId} {sessionId} {bytes} {wasmMs} {detail}', {
      documentId: target.document.documentId,
      sessionId,
      bytes,
      wasmMs,
      detail: verdict.error.detail,
    });
    return toolFailure('error', messageOf(verdict.error));
  }

  log.info('save-document preverified: {documentId} {sessionId} {bytes} {wasmMs}', {
    documentId: target.document.documentId,
    sessionId,
    bytes,
    wasmMs,
  });

  return null;
};

const preverifySaveDocument: PrismToolPreflight = async (ctx, input) => {
  const target = await saveTargetOf(ctx, input);
  return 'ok' in target ? target : await verifyFailureOf(target, ctx.session.id);
};

export const documentEditPreflights: Record<string, PrismToolPreflight> = {
  'save-document': preverifySaveDocument,
};
