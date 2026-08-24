import { logger } from '@typie/lib';
import { ENTITY_ICON_COLORS, ENTITY_ICON_NAMES, NOTE_COLORS, NOTE_DEFAULT_COLOR } from '@typie/lib/catalogs';
import { DocumentCommentState, DocumentCommentThreadState, EntityState, EntityType, NoteState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { toolFailure } from '@typie/prism';
import dayjs from 'dayjs';
import { and, asc, desc, eq, gt, inArray, isNotNull, isNull, or } from 'drizzle-orm';
import { alias } from 'drizzle-orm/pg-core';
import { z } from 'zod';
import {
  db,
  DocumentComments,
  DocumentCommentThreads,
  Documents,
  Entities,
  EntityGoals,
  first,
  Folders,
  NoteEntities,
  Notes,
  Users,
} from '#/db/index.ts';
import { env } from '#/env.ts';
import { elasticsearch, esIndex } from '#/search.ts';
import {
  createDocumentCore,
  createFolderCore,
  deleteEntitiesCore,
  duplicateDocumentCore,
  moveEntitiesCore,
  recoverEntityCore,
  renameFolderCore,
  updateDocumentsOptionCore,
  updateEntityIconCore,
  updateFolderOptionCore,
} from './entity-actions.ts';
import { deleteEntityGoalCore, deleteUserGoalCore, upsertEntityGoalCore, upsertUserGoalCore } from './goal-actions.ts';
import { addNoteEntityCore, createNoteCore, deleteNoteCore, removeNoteEntityCore, updateNoteCore } from './note-actions.ts';
import { assertActiveSubscription } from './plan.ts';
import { snapshotManuscript } from './prism-manuscript.ts';
import { ERROR_MESSAGE } from './prism-tool-messages.ts';
import {
  COMMENT_PAGE_SIZE,
  CreateDocumentInput,
  CreateFolderInput,
  CreateNoteInput,
  DeleteEntitiesInput,
  DeleteGoalInput,
  DeleteNoteInput,
  DuplicateDocumentInput,
  entityRefKind,
  entityUrl,
  kstDate,
  kstDueDate,
  ListEntitiesInput,
  MoveEntitiesInput,
  NoteLinkInput,
  notePreview,
  pageOf,
  ReadCommentsInput,
  ReadDocumentInput,
  ReadNoteInput,
  ReadSharingInput,
  RecoverEntityInput,
  RenameFolderInput,
  SearchEntitiesInput,
  SetGoalInput,
  snippetOf,
  TRASH_PAGE_SIZE,
  UpdateIconInput,
  UpdateNoteInput,
  UpdateSharingInput,
  validIcon,
  windowOf,
  withinDays,
} from './prism-workspace-core.ts';
import { decompose } from './text.ts';
import { currentUserGoal, dailyCharacterChanges, dailyGoalHistory } from './user-stats.ts';
import type { Dayjs } from 'dayjs';
import type { SQL } from 'drizzle-orm';
import type { PrismToolContext, PrismToolHandler } from './prism-tools.ts';
import type { EntityRefKind } from './prism-workspace-core.ts';

const log = logger.getChild('prism-workspace');

const searchEntities = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = SearchEntitiesInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const query = parsed.data.query.trim();
  const decomposed = decompose(query);
  const result = await elasticsearch.search({
    index: [esIndex.documents, esIndex.folders],
    size: 10,
    _source: false,
    query: {
      bool: {
        should: [
          { match: { title: { query, boost: 3 } } },
          { match: { subtitle: { query, boost: 2 } } },
          { match: { text: { query } } },
          { match: { name: { query, boost: 2 } } },
          ...(decomposed
            ? [
                { match: { title_decomposed: { query: decomposed, boost: 1.5 } } },
                { match: { subtitle_decomposed: { query: decomposed, boost: 1 } } },
                { match: { name_decomposed: { query: decomposed } } },
              ]
            : []),
        ],
        filter: [{ term: { site_id: ctx.siteId } }],
        minimum_should_match: 1,
      },
    },
    highlight: { fields: { text: { fragment_size: 200, number_of_fragments: 1 } }, pre_tags: ['<em>'], post_tags: ['</em>'] },
  });

  const hitIds = result.hits.hits.map((hit) => ({ index: hit._index, id: hit._id ?? '', snippet: snippetOf(hit.highlight?.text?.[0]) }));
  const documentIds = hitIds.filter((hit) => hit.index === esIndex.documents).map((hit) => hit.id);
  const folderIds = hitIds.filter((hit) => hit.index === esIndex.folders).map((hit) => hit.id);

  const documents =
    documentIds.length === 0
      ? []
      : await db
          .select({ id: Documents.id, entityId: Documents.entityId, title: Documents.title, subtitle: Documents.subtitle })
          .from(Documents)
          .innerJoin(Entities, eq(Documents.entityId, Entities.id))
          .where(and(inArray(Documents.id, documentIds), eq(Entities.state, EntityState.ACTIVE)));
  const folders =
    folderIds.length === 0
      ? []
      : await db
          .select({ id: Folders.id, entityId: Folders.entityId, name: Folders.name })
          .from(Folders)
          .innerJoin(Entities, eq(Folders.entityId, Entities.id))
          .where(and(inArray(Folders.id, folderIds), eq(Entities.state, EntityState.ACTIVE)));

  const snippets = new Map(hitIds.map((hit) => [hit.id, hit.snippet]));
  const documentById = new Map(documents.map((doc) => [doc.id, doc]));
  const folderById = new Map(folders.map((folder) => [folder.id, folder]));

  return {
    ok: true,
    documents: documentIds.flatMap((id) => {
      const doc = documentById.get(id);
      return doc ? [{ ...doc, snippet: snippets.get(id) ?? null }] : [];
    }),
    folders: folderIds.flatMap((id) => {
      const folder = folderById.get(id);
      return folder ? [folder] : [];
    }),
  };
};

type EntityItem =
  | { kind: 'document'; id: string; entityId: string; title: string | null; subtitle: string | null }
  | { kind: 'folder'; id: string; entityId: string; name: string };

const listEntities = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = ListEntitiesInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  let parentEntityId: string | null = null;
  if (parsed.data.folderId !== undefined) {
    const parent = await db
      .select({ entityId: Folders.entityId })
      .from(Folders)
      .innerJoin(Entities, eq(Folders.entityId, Entities.id))
      .where(and(eq(Folders.id, parsed.data.folderId), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)))
      .then(first);
    if (!parent) return toolFailure('error', '그 폴더를 찾지 못했어요 — list-entities를 인자 없이 불러 스페이스 첫 깊이부터 확인하세요.');
    parentEntityId = parent.entityId;
  }

  const rows = await db
    .select({ entityId: Entities.id, type: Entities.type, order: Entities.order })
    .from(Entities)
    .where(
      and(
        eq(Entities.siteId, ctx.siteId),
        eq(Entities.state, EntityState.ACTIVE),
        parentEntityId === null ? isNull(Entities.parentId) : eq(Entities.parentId, parentEntityId),
      ),
    )
    .orderBy(asc(Entities.order));

  const entityIds = rows.map((row) => row.entityId);
  const documents =
    entityIds.length === 0
      ? []
      : await db
          .select({ entityId: Documents.entityId, id: Documents.id, title: Documents.title, subtitle: Documents.subtitle })
          .from(Documents)
          .where(inArray(Documents.entityId, entityIds));
  const folders =
    entityIds.length === 0
      ? []
      : await db
          .select({ entityId: Folders.entityId, id: Folders.id, name: Folders.name })
          .from(Folders)
          .where(inArray(Folders.entityId, entityIds));

  const documentByEntity = new Map(documents.map((doc) => [doc.entityId, doc]));
  const folderByEntity = new Map(folders.map((folder) => [folder.entityId, folder]));

  return {
    ok: true,
    items: rows.flatMap((row): EntityItem[] => {
      if (row.type === EntityType.DOCUMENT) {
        const doc = documentByEntity.get(row.entityId);
        return doc ? [{ kind: 'document', id: doc.id, entityId: row.entityId, title: doc.title, subtitle: doc.subtitle }] : [];
      }
      const folder = folderByEntity.get(row.entityId);
      return folder ? [{ kind: 'folder', id: folder.id, entityId: row.entityId, name: folder.name }] : [];
    }),
  };
};

const readDocument = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = ReadDocumentInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const owned = await db
    .select({ id: Documents.id })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(and(eq(Documents.id, parsed.data.documentId), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)))
    .then(first);
  if (!owned) return toolFailure('error', '그 문서를 찾지 못했어요 — search-entities나 list-entities로 문서를 다시 찾아보세요.');

  try {
    const manuscript = await snapshotManuscript(parsed.data.documentId);
    const window = windowOf(manuscript.content, parsed.data.offset, parsed.data.length);
    if (window.range.offset >= window.range.total) {
      return toolFailure('error', `본문 끝을 넘는 위치예요 — 본문은 ${window.range.total}자까지예요. offset을 그보다 작게 주세요.`);
    }

    return {
      ok: true,
      title: manuscript.title,
      subtitle: manuscript.subtitle,
      characterCount: manuscript.characterCount,
      content: window.content,
      range: window.range,
    };
  } catch (err) {
    if (err instanceof TypieError && err.code === 'prism_manuscript_empty')
      return toolFailure('error', '이 문서는 아직 읽을 내용이 없어요.');
    log.warn('read-document snapshot failed: {documentId} {*}', { documentId: parsed.data.documentId, error: err });
    return toolFailure('error', ERROR_MESSAGE);
  }
};

type NoteAttachment = { entityId: string; kind: 'document' | 'folder'; title: string | null };

const noteAttachments = async (siteId: string, noteIds: string[]): Promise<Map<string, NoteAttachment[]>> => {
  if (noteIds.length === 0) return new Map();

  const rows = await db
    .select({ noteId: NoteEntities.noteId, entityId: Entities.id, type: Entities.type, title: Documents.title, name: Folders.name })
    .from(NoteEntities)
    .innerJoin(Entities, eq(NoteEntities.entityId, Entities.id))
    .leftJoin(Documents, eq(Documents.entityId, Entities.id))
    .leftJoin(Folders, eq(Folders.entityId, Entities.id))
    .where(and(inArray(NoteEntities.noteId, noteIds), eq(Entities.siteId, siteId), eq(Entities.state, EntityState.ACTIVE)));

  const map = new Map<string, NoteAttachment[]>();
  for (const row of rows) {
    const list = map.get(row.noteId) ?? [];
    list.push({
      entityId: row.entityId,
      kind: row.type === EntityType.DOCUMENT ? 'document' : 'folder',
      title: row.type === EntityType.DOCUMENT ? row.title : row.name,
    });
    map.set(row.noteId, list);
  }

  return map;
};

const listNotes = async (ctx: PrismToolContext, input: unknown) => {
  if (!z.object({}).safeParse(input).success) return toolFailure('error', ERROR_MESSAGE);

  const notes = await db
    .select({ id: Notes.id, content: Notes.content, color: Notes.color, status: Notes.status, updatedAt: Notes.updatedAt })
    .from(Notes)
    .where(and(eq(Notes.siteId, ctx.siteId), eq(Notes.state, NoteState.ACTIVE)))
    .orderBy(asc(Notes.order));

  const attachments = await noteAttachments(
    ctx.siteId,
    notes.map((note) => note.id),
  );

  return {
    ok: true,
    notes: notes.map((note) => ({
      id: note.id,
      preview: notePreview(note.content),
      color: note.color,
      status: note.status,
      attachments: attachments.get(note.id) ?? [],
      updatedAt: note.updatedAt.toISOString(),
    })),
  };
};

const readNote = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = ReadNoteInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const note = await db
    .select({ id: Notes.id, content: Notes.content, color: Notes.color, status: Notes.status, updatedAt: Notes.updatedAt })
    .from(Notes)
    .where(and(eq(Notes.id, parsed.data.noteId), eq(Notes.siteId, ctx.siteId), eq(Notes.state, NoteState.ACTIVE)))
    .then(first);
  if (!note) return toolFailure('error', '그 노트를 찾지 못했어요 — list-notes로 노트를 다시 확인하세요.');

  const attachments = await noteAttachments(ctx.siteId, [note.id]);

  return {
    ok: true,
    id: note.id,
    content: note.content,
    color: note.color,
    status: note.status,
    attachments: attachments.get(note.id) ?? [],
    updatedAt: note.updatedAt.toISOString(),
  };
};

const readStats = async (ctx: PrismToolContext, input: unknown) => {
  if (!z.object({}).safeParse(input).success) return toolFailure('error', ERROR_MESSAGE);

  const rows = await dailyCharacterChanges(ctx.userId);
  const days = withinDays(rows, 30, dayjs.kst()).map((row) => ({
    date: kstDate(row.date),
    additions: row.additions,
    deletions: row.deletions,
  }));
  const today = kstDate(dayjs.kst());

  return {
    ok: true,
    days,
    today: days.find((day) => day.date === today) ?? { date: today, additions: 0, deletions: 0 },
  };
};

const readGoals = async (ctx: PrismToolContext, input: unknown) => {
  if (!z.object({}).safeParse(input).success) return toolFailure('error', ERROR_MESSAGE);

  const [current, history, entityRows] = await Promise.all([
    currentUserGoal(ctx.userId),
    dailyGoalHistory(ctx.userId),
    db
      .select({
        entityId: Entities.id,
        type: Entities.type,
        targetCharacterCount: EntityGoals.targetCharacterCount,
        dueAt: EntityGoals.dueAt,
      })
      .from(EntityGoals)
      .innerJoin(Entities, eq(EntityGoals.entityId, Entities.id))
      .where(and(eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE))),
  ]);

  const entityIds = entityRows.map((row) => row.entityId);
  const documents =
    entityIds.length === 0
      ? []
      : await db
          .select({ entityId: Documents.entityId, title: Documents.title })
          .from(Documents)
          .where(inArray(Documents.entityId, entityIds));
  const folders =
    entityIds.length === 0
      ? []
      : await db.select({ entityId: Folders.entityId, name: Folders.name }).from(Folders).where(inArray(Folders.entityId, entityIds));
  const titleByEntity = new Map<string, string | null>([
    ...documents.map((doc) => [doc.entityId, doc.title] as const),
    ...folders.map((folder) => [folder.entityId, folder.name] as const),
  ]);

  return {
    ok: true,
    current: current === null ? null : { targetCharacterCount: current.targetCharacterCount },
    recent: withinDays(history, 14, dayjs.kst()).map((row) => ({
      date: kstDate(row.date),
      targetCharacterCount: row.targetCharacterCount,
      additions: row.additions,
      achieved: row.achieved,
    })),
    entityGoals: entityRows.map((row) => ({
      entityId: row.entityId,
      kind: row.type === EntityType.DOCUMENT ? 'document' : 'folder',
      title: titleByEntity.get(row.entityId) ?? null,
      targetCharacterCount: row.targetCharacterCount,
      dueAt: row.dueAt === null ? null : kstDate(row.dueAt),
    })),
  };
};

const readSharing = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = ReadSharingInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const entityIds = await resolveEntityIds(ctx, parsed.data.ids);
  if (entityIds === null) return toolFailure('error', NOT_FOUND_TARGETS);

  const rows = await db
    .select({
      entityId: Entities.id,
      type: Entities.type,
      visibility: Entities.visibility,
      permalink: Entities.permalink,
    })
    .from(Entities)
    .where(and(inArray(Entities.id, entityIds), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)));
  if (rows.length !== entityIds.length) return toolFailure('error', NOT_FOUND_TARGETS);

  const documents = await db
    .select({ entityId: Documents.entityId, title: Documents.title, password: Documents.password })
    .from(Documents)
    .where(inArray(Documents.entityId, entityIds));
  const folders = await db
    .select({ entityId: Folders.entityId, name: Folders.name })
    .from(Folders)
    .where(inArray(Folders.entityId, entityIds));
  const documentByEntity = new Map(documents.map((doc) => [doc.entityId, doc]));
  const folderByEntity = new Map(folders.map((folder) => [folder.entityId, folder]));

  return {
    ok: true,
    items: rows.map((row) => {
      const doc = documentByEntity.get(row.entityId);

      return {
        entityId: row.entityId,
        kind: row.type === EntityType.DOCUMENT ? 'document' : 'folder',
        title: doc === undefined ? (folderByEntity.get(row.entityId)?.name ?? null) : doc.title,
        visibility: row.visibility,
        hasPassword: doc === undefined ? null : doc.password !== null,
        url: entityUrl(env.USERSITE_URL, row.permalink),
      };
    }),
  };
};

const readComments = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = ReadCommentsInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const owned = await db
    .select({ id: Documents.id })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(and(eq(Documents.id, parsed.data.documentId), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)))
    .then(first);
  if (!owned) return toolFailure('error', '그 문서를 찾지 못했어요 — search-entities나 list-entities로 문서를 다시 찾아보세요.');

  const conditions = [
    eq(DocumentCommentThreads.documentId, parsed.data.documentId),
    eq(DocumentCommentThreads.state, DocumentCommentThreadState.ACTIVE),
    parsed.data.resolved ? isNotNull(DocumentCommentThreads.resolvedAt) : isNull(DocumentCommentThreads.resolvedAt),
  ];

  const recent = await db
    .select({ id: DocumentCommentThreads.id, resolvedAt: DocumentCommentThreads.resolvedAt, createdAt: DocumentCommentThreads.createdAt })
    .from(DocumentCommentThreads)
    .where(and(...conditions))
    .orderBy(desc(DocumentCommentThreads.createdAt), desc(DocumentCommentThreads.id))
    .limit(COMMENT_PAGE_SIZE + 1);

  const page = pageOf(recent, COMMENT_PAGE_SIZE);
  const threads = page.items.toReversed();
  const threadIds = threads.map((thread) => thread.id);
  const comments =
    threadIds.length === 0
      ? []
      : await db
          .select({
            threadId: DocumentComments.threadId,
            author: Users.name,
            content: DocumentComments.content,
            createdAt: DocumentComments.createdAt,
          })
          .from(DocumentComments)
          .innerJoin(Users, eq(DocumentComments.userId, Users.id))
          .where(and(inArray(DocumentComments.threadId, threadIds), eq(DocumentComments.state, DocumentCommentState.ACTIVE)))
          .orderBy(asc(DocumentComments.createdAt), asc(DocumentComments.id));

  const commentsByThread = new Map<string, { author: string; content: string; createdAt: string }[]>();
  for (const comment of comments) {
    const list = commentsByThread.get(comment.threadId) ?? [];
    list.push({ author: comment.author, content: comment.content, createdAt: comment.createdAt.toISOString() });
    commentsByThread.set(comment.threadId, list);
  }

  return {
    ok: true,
    threads: threads.map((thread) => ({
      id: thread.id,
      resolved: thread.resolvedAt !== null,
      createdAt: thread.createdAt.toISOString(),
      comments: commentsByThread.get(thread.id) ?? [],
    })),
    truncated: page.truncated,
  };
};

const listTrash = async (ctx: PrismToolContext, input: unknown) => {
  if (!z.object({}).safeParse(input).success) return toolFailure('error', ERROR_MESSAGE);

  const parents = alias(Entities, 'parent_entities');
  const rows = await db
    .select({ entityId: Entities.id, type: Entities.type, deletedAt: Entities.deletedAt })
    .from(Entities)
    .leftJoin(parents, eq(Entities.parentId, parents.id))
    .where(
      and(
        eq(Entities.siteId, ctx.siteId),
        eq(Entities.state, EntityState.DELETED),
        gt(Entities.deletedAt, dayjs().subtract(30, 'days')),
        or(isNull(parents.id), eq(parents.state, EntityState.ACTIVE)),
      ),
    )
    .orderBy(desc(Entities.deletedAt))
    .limit(TRASH_PAGE_SIZE + 1);

  const page = pageOf(rows, TRASH_PAGE_SIZE);
  const entityIds = page.items.map((row) => row.entityId);
  const documents =
    entityIds.length === 0
      ? []
      : await db
          .select({ entityId: Documents.entityId, title: Documents.title })
          .from(Documents)
          .where(inArray(Documents.entityId, entityIds));
  const folders =
    entityIds.length === 0
      ? []
      : await db.select({ entityId: Folders.entityId, name: Folders.name }).from(Folders).where(inArray(Folders.entityId, entityIds));
  const titleByEntity = new Map<string, string | null>([
    ...documents.map((doc) => [doc.entityId, doc.title] as const),
    ...folders.map((folder) => [folder.entityId, folder.name] as const),
  ]);

  return {
    ok: true,
    items: page.items.map((row) => ({
      entityId: row.entityId,
      kind: row.type === EntityType.DOCUMENT ? 'document' : 'folder',
      title: titleByEntity.get(row.entityId) ?? null,
      deletedAt: row.deletedAt === null ? null : row.deletedAt.toISOString(),
    })),
    truncated: page.truncated,
  };
};

const listIcons = async (_ctx: PrismToolContext, input: unknown) => {
  if (!z.object({}).safeParse(input).success) return toolFailure('error', ERROR_MESSAGE);

  return { ok: true, icons: [...ENTITY_ICON_NAMES], colors: [...ENTITY_ICON_COLORS] };
};

const NOT_FOUND_FOLDER = '그 폴더를 찾지 못했어요 — list-entities로 다시 확인하세요.';
const NOT_FOUND_TARGETS = '일부 대상을 찾지 못했어요 — list-entities로 id를 다시 확인하세요.';
const NOT_FOUND_NOTE = '그 노트를 찾지 못했어요 — list-notes로 다시 확인하세요.';
const INVALID_COLOR = '그 색은 없어요 — 목록에 있는 색만 쓸 수 있어요.';

const folderEntityId = async (ctx: PrismToolContext, folderId: string): Promise<string | null> =>
  await ctx.executor
    .select({ entityId: Folders.entityId })
    .from(Folders)
    .innerJoin(Entities, eq(Folders.entityId, Entities.id))
    .where(and(eq(Folders.id, folderId), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)))
    .then(first)
    .then((row) => row?.entityId ?? null);

const idsOfKind = (ids: string[], kind: EntityRefKind): string[] => ids.filter((id) => entityRefKind(id) === kind);

export const entityRefFilter = (ids: string[]): SQL | undefined =>
  or(
    inArray(Entities.id, idsOfKind(ids, 'entity')),
    inArray(Documents.id, idsOfKind(ids, 'document')),
    inArray(Folders.id, idsOfKind(ids, 'folder')),
  );

type EntityRef = { entityId: string; deletedAt: Dayjs | null };

const resolveEntityRefs = async (ctx: PrismToolContext, ids: string[], state: EntityState): Promise<Map<string, EntityRef>> => {
  const wanted = new Map<string, EntityRefKind>();
  for (const id of ids) {
    const kind = entityRefKind(id);
    if (kind !== null) wanted.set(id, kind);
  }
  if (wanted.size === 0) return new Map();

  const rows = await ctx.executor
    .select({ entityId: Entities.id, deletedAt: Entities.deletedAt, documentId: Documents.id, folderId: Folders.id })
    .from(Entities)
    .leftJoin(Documents, eq(Documents.entityId, Entities.id))
    .leftJoin(Folders, eq(Folders.entityId, Entities.id))
    .where(and(eq(Entities.siteId, ctx.siteId), eq(Entities.state, state), entityRefFilter([...wanted.keys()])));

  const refs = new Map<string, EntityRef>();
  for (const row of rows) {
    const ref = { entityId: row.entityId, deletedAt: row.deletedAt };
    if (wanted.get(row.entityId) === 'entity') refs.set(row.entityId, ref);
    if (row.documentId !== null && wanted.get(row.documentId) === 'document') refs.set(row.documentId, ref);
    if (row.folderId !== null && wanted.get(row.folderId) === 'folder') refs.set(row.folderId, ref);
  }

  return refs;
};

const resolveEntityId = async (ctx: PrismToolContext, id: string): Promise<string | null> => {
  const refs = await resolveEntityRefs(ctx, [id], EntityState.ACTIVE);

  return refs.get(id)?.entityId ?? null;
};

const resolveEntityIds = async (ctx: PrismToolContext, ids: string[]): Promise<string[] | null> => {
  const unique = [...new Set(ids)];
  const refs = await resolveEntityRefs(ctx, unique, EntityState.ACTIVE);
  if (refs.size !== unique.length) return null;

  return [...new Set([...refs.values()].map((ref) => ref.entityId))];
};

const scopedNote = async (ctx: PrismToolContext, noteId: string): Promise<boolean> =>
  await ctx.executor
    .select({ id: Notes.id })
    .from(Notes)
    .where(and(eq(Notes.id, noteId), eq(Notes.siteId, ctx.siteId), eq(Notes.userId, ctx.userId), eq(Notes.state, NoteState.ACTIVE)))
    .then(first)
    .then((row) => row !== undefined);

const createFolder = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = CreateFolderInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  let parentEntityId: string | null = null;
  if (parsed.data.parentFolderId !== undefined) {
    const parent = await ctx.executor
      .select({ entityId: Folders.entityId })
      .from(Folders)
      .innerJoin(Entities, eq(Folders.entityId, Entities.id))
      .where(and(eq(Folders.id, parsed.data.parentFolderId), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)))
      .then(first);
    if (!parent) return toolFailure('error', '상위 폴더를 찾지 못했어요 — list-entities로 폴더를 다시 확인하세요.');
    parentEntityId = parent.entityId;
  }

  const folder = await createFolderCore(ctx.executor, {
    userId: ctx.userId,
    siteId: ctx.siteId,
    parentEntityId,
    name: parsed.data.name,
  });

  return { ok: true, folderId: folder.id, name: parsed.data.name, entityId: folder.entityId };
};

const deleteEntities = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = DeleteEntitiesInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const entityIds = await resolveEntityIds(ctx, parsed.data.ids);
  if (entityIds === null)
    return toolFailure(
      'error',
      '그 문서나 폴더를 지금 작업 중인 스페이스에서 찾지 못했어요 — list-entities로 지울 대상을 다시 확인하세요.',
    );

  await deleteEntitiesCore(ctx.executor, { userId: ctx.userId, entityIds });

  return { ok: true, count: entityIds.length };
};

const createDocument = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = CreateDocumentInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  let parentEntityId: string | null = null;
  if (parsed.data.folderId !== undefined) {
    parentEntityId = await folderEntityId(ctx, parsed.data.folderId);
    if (parentEntityId === null) return toolFailure('error', NOT_FOUND_FOLDER);
  }

  const document = await createDocumentCore(ctx.executor, { userId: ctx.userId, siteId: ctx.siteId, parentEntityId });

  return { ok: true, documentId: document.id, entityId: document.entityId };
};

const renameFolder = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = RenameFolderInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  if ((await folderEntityId(ctx, parsed.data.folderId)) === null) return toolFailure('error', NOT_FOUND_FOLDER);

  await renameFolderCore(ctx.executor, { userId: ctx.userId, folderId: parsed.data.folderId, name: parsed.data.name });

  return { ok: true, folderId: parsed.data.folderId, name: parsed.data.name };
};

const moveEntities = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = MoveEntitiesInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const entityIds = await resolveEntityIds(ctx, parsed.data.ids);
  if (entityIds === null) return toolFailure('error', NOT_FOUND_TARGETS);

  let parentEntityId: string | null = null;
  if (parsed.data.folderId !== undefined) {
    parentEntityId = await folderEntityId(ctx, parsed.data.folderId);
    if (parentEntityId === null) return toolFailure('error', NOT_FOUND_FOLDER);
  }

  await moveEntitiesCore(ctx.executor, {
    userId: ctx.userId,
    entityIds,
    parentEntityId,
    lowerOrder: null,
    upperOrder: null,
    targetSiteId: null,
  });

  return { ok: true, count: entityIds.length };
};

const duplicateDocument = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = DuplicateDocumentInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const owned = await ctx.executor
    .select({ id: Documents.id })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(and(eq(Documents.id, parsed.data.documentId), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)))
    .then(first);
  if (!owned) return toolFailure('error', '그 문서를 찾지 못했어요 — search-entities나 list-entities로 다시 찾아보세요.');

  const document = await duplicateDocumentCore(ctx.executor, { userId: ctx.userId, documentId: parsed.data.documentId });

  return { ok: true, documentId: document.id, entityId: document.entityId };
};

const updateIcon = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = UpdateIconInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  if (!validIcon(parsed.data.icon, parsed.data.iconColor)) {
    return toolFailure('error', '그 아이콘이나 색은 없어요 — 목록에 있는 이름만 쓸 수 있어요.');
  }
  const entityId = await resolveEntityId(ctx, parsed.data.id);
  if (entityId === null) return toolFailure('error', NOT_FOUND_TARGETS);

  await updateEntityIconCore(ctx.executor, { userId: ctx.userId, entityId, icon: parsed.data.icon, iconColor: parsed.data.iconColor });

  return { ok: true, entityId };
};

const recoverEntity = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = RecoverEntityInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  const refs = await resolveEntityRefs(ctx, [parsed.data.id], EntityState.DELETED);
  const ref = refs.get(parsed.data.id);
  if (ref === undefined || ref.deletedAt === null || !ref.deletedAt.isAfter(dayjs().subtract(30, 'days'))) {
    return toolFailure('error', '휴지통에서 그 항목을 찾지 못했어요 — list-trash로 다시 확인하세요.');
  }

  await recoverEntityCore(ctx.executor, { userId: ctx.userId, entityId: ref.entityId });

  return { ok: true, entityId: ref.entityId };
};

const createNote = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = CreateNoteInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  await assertActiveSubscription({ userId: ctx.userId });

  const color = parsed.data.color ?? NOTE_DEFAULT_COLOR;
  if (!NOTE_COLORS.includes(color)) return toolFailure('error', INVALID_COLOR);

  const note = await createNoteCore(ctx.executor, {
    userId: ctx.userId,
    siteId: ctx.siteId,
    content: parsed.data.content,
    color,
    entityIds: [],
  });

  return { ok: true, noteId: note.id };
};

const updateNote = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = UpdateNoteInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  if (parsed.data.color !== undefined && !NOTE_COLORS.includes(parsed.data.color)) return toolFailure('error', INVALID_COLOR);
  if (!(await scopedNote(ctx, parsed.data.noteId))) return toolFailure('error', NOT_FOUND_NOTE);

  await updateNoteCore(ctx.executor, { userId: ctx.userId, ...parsed.data });

  return { ok: true, noteId: parsed.data.noteId };
};

const attachNote = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = NoteLinkInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  if (!(await scopedNote(ctx, parsed.data.noteId))) return toolFailure('error', NOT_FOUND_NOTE);
  const entityId = await resolveEntityId(ctx, parsed.data.id);
  if (entityId === null) return toolFailure('error', NOT_FOUND_TARGETS);

  await addNoteEntityCore(ctx.executor, { userId: ctx.userId, noteId: parsed.data.noteId, entityId });

  return { ok: true, noteId: parsed.data.noteId, entityId };
};

const detachNote = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = NoteLinkInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  if (!(await scopedNote(ctx, parsed.data.noteId))) return toolFailure('error', NOT_FOUND_NOTE);
  const entityId = await resolveEntityId(ctx, parsed.data.id);
  if (entityId === null) return toolFailure('error', NOT_FOUND_TARGETS);

  await removeNoteEntityCore(ctx.executor, { userId: ctx.userId, noteId: parsed.data.noteId, entityId });

  return { ok: true, noteId: parsed.data.noteId, entityId };
};

const setGoal = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = SetGoalInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  if (parsed.data.id === undefined) {
    await upsertUserGoalCore(ctx.executor, { userId: ctx.userId, targetCharacterCount: parsed.data.targetCharacterCount });

    return { ok: true };
  }

  const entityId = await resolveEntityId(ctx, parsed.data.id);
  if (entityId === null) return toolFailure('error', NOT_FOUND_TARGETS);

  const dueAt = parsed.data.dueAt === undefined ? null : kstDueDate(parsed.data.dueAt);
  if (dueAt === null && parsed.data.dueAt !== undefined) {
    return toolFailure('error', '그 날짜는 달력에 없어요 — YYYY-MM-DD로 실제 날짜를 주세요.');
  }

  await upsertEntityGoalCore(ctx.executor, {
    userId: ctx.userId,
    entityId,
    targetCharacterCount: parsed.data.targetCharacterCount,
    dueAt,
  });

  return { ok: true };
};

const deleteNote = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = DeleteNoteInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  if (!(await scopedNote(ctx, parsed.data.noteId))) return toolFailure('error', NOT_FOUND_NOTE);

  await deleteNoteCore(ctx.executor, { userId: ctx.userId, noteId: parsed.data.noteId });

  return { ok: true, noteId: parsed.data.noteId };
};

const deleteGoal = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = DeleteGoalInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  if (parsed.data.id === undefined) {
    await deleteUserGoalCore(ctx.executor, { userId: ctx.userId });

    return { ok: true };
  }

  const entityId = await resolveEntityId(ctx, parsed.data.id);
  if (entityId === null) return toolFailure('error', NOT_FOUND_TARGETS);

  await deleteEntityGoalCore(ctx.executor, { userId: ctx.userId, entityId });

  return { ok: true };
};

const updateSharing = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = UpdateSharingInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const entityIds = await resolveEntityIds(ctx, parsed.data.ids);
  if (entityIds === null) return toolFailure('error', NOT_FOUND_TARGETS);

  const rows = await ctx.executor
    .select({ entityId: Entities.id, type: Entities.type, visibility: Entities.visibility })
    .from(Entities)
    .where(and(inArray(Entities.id, entityIds), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)));
  if (rows.length !== entityIds.length) return toolFailure('error', NOT_FOUND_TARGETS);

  const documentEntityIds = rows.filter((row) => row.type === EntityType.DOCUMENT).map((row) => row.entityId);
  const folderEntityIds = rows.filter((row) => row.type === EntityType.FOLDER).map((row) => row.entityId);

  const documents =
    documentEntityIds.length === 0
      ? []
      : await ctx.executor
          .select({ id: Documents.id, entityId: Documents.entityId, title: Documents.title })
          .from(Documents)
          .where(inArray(Documents.entityId, documentEntityIds));
  const folders =
    folderEntityIds.length === 0
      ? []
      : await ctx.executor
          .select({ id: Folders.id, entityId: Folders.entityId, name: Folders.name })
          .from(Folders)
          .where(inArray(Folders.entityId, folderEntityIds));
  const titleByEntity = new Map<string, string | null>([
    ...documents.map((doc) => [doc.entityId, doc.title] as const),
    ...folders.map((folder) => [folder.entityId, folder.name] as const),
  ]);

  if (documents.length > 0) {
    await updateDocumentsOptionCore(ctx.executor, {
      userId: ctx.userId,
      documentIds: documents.map((doc) => doc.id),
      visibility: parsed.data.visibility,
    });
  }
  for (const folder of folders) {
    await updateFolderOptionCore(ctx.executor, {
      userId: ctx.userId,
      folderId: folder.id,
      visibility: parsed.data.visibility,
      recursive: parsed.data.recursive ?? false,
    });
  }

  return {
    ok: true,
    count: rows.length,
    changes: rows.map((row) => ({
      id: row.entityId,
      kind: row.type === EntityType.DOCUMENT ? 'document' : 'folder',
      title: titleByEntity.get(row.entityId) ?? null,
      from: row.visibility,
      to: parsed.data.visibility,
    })),
  };
};

export const workspaceTools: Record<string, PrismToolHandler> = {
  'search-entities': searchEntities,
  'list-entities': listEntities,
  'read-document': readDocument,
  'list-notes': listNotes,
  'read-note': readNote,
  'read-stats': readStats,
  'read-goals': readGoals,
  'read-sharing': readSharing,
  'read-comments': readComments,
  'list-trash': listTrash,
  'list-icons': listIcons,
  'create-folder': createFolder,
  'create-document': createDocument,
  'rename-folder': renameFolder,
  'move-entities': moveEntities,
  'duplicate-document': duplicateDocument,
  'create-note': createNote,
  'update-note': updateNote,
  'attach-note': attachNote,
  'detach-note': detachNote,
  'set-goal': setGoal,
  'update-icon': updateIcon,
  'recover-entity': recoverEntity,
  'delete-entities': deleteEntities,
  'delete-note': deleteNote,
  'delete-goal': deleteGoal,
  'update-sharing': updateSharing,
};
