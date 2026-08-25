import { logger } from '@typie/lib';
import { ENTITY_ICON_COLORS, ENTITY_ICON_NAMES, NOTE_COLORS, NOTE_DEFAULT_COLOR } from '@typie/lib/catalogs';
import { DocumentCommentState, DocumentCommentThreadState, EntityState, EntityType, NoteState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { toolFailure } from '@typie/prism';
import dayjs from 'dayjs';
import { and, asc, count, desc, eq, gt, inArray, isNotNull, isNull, or, sql } from 'drizzle-orm';
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
  CreateDocumentsInput,
  CreateFoldersInput,
  CreateNotesInput,
  DeleteEntitiesInput,
  DeleteGoalsInput,
  DeleteNotesInput,
  DuplicateDocumentsInput,
  entityRefKind,
  entityUrl,
  kstDate,
  kstDueDate,
  LIST_ROWS_MAX,
  ListEntitiesInput,
  MoveEntitiesInput,
  NoteLinksInput,
  notePreview,
  pageOf,
  preorder,
  ReadCommentsInput,
  ReadDocumentInput,
  ReadNoteInput,
  ReadSharingInput,
  RecoverEntitiesInput,
  RenameFoldersInput,
  SearchEntitiesInput,
  SetGoalsInput,
  snippetOf,
  TRASH_PAGE_SIZE,
  UpdateIconsInput,
  UpdateNotesInput,
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
  | { kind: 'document'; id: string; entityId: string; title: string | null; subtitle: string | null; depth: number }
  | { kind: 'folder'; id: string; entityId: string; name: string; depth: number };

const listEntities = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = ListEntitiesInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  const { offset, limit, recursive } = parsed.data;

  let scope: { id: string; entityId: string; name: string } | null = null;
  if (parsed.data.folderId !== undefined) {
    const folder = await db
      .select({ id: Folders.id, entityId: Folders.entityId, name: Folders.name })
      .from(Folders)
      .innerJoin(Entities, eq(Folders.entityId, Entities.id))
      .where(and(eq(Folders.id, parsed.data.folderId), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)))
      .then(first);
    if (!folder) return toolFailure('error', '그 폴더를 찾지 못했어요 — list-entities를 인자 없이 불러 스페이스 전체 트리부터 확인하세요.');
    scope = folder;
  }

  const inScope = and(
    eq(Entities.siteId, ctx.siteId),
    eq(Entities.state, EntityState.ACTIVE),
    scope === null ? isNull(Entities.parentId) : eq(Entities.parentId, scope.entityId),
  );
  const total = await db
    .select({ total: count() })
    .from(Entities)
    .where(inScope)
    .then((rows) => rows[0]?.total ?? 0);
  if (offset > 0 && offset >= total) {
    return toolFailure('error', `끝을 넘는 위치예요 — 여기 직계 항목은 ${total}개까지예요. offset을 그 안으로 주세요.`);
  }

  const page = await db.select({ id: Entities.id }).from(Entities).where(inScope).orderBy(asc(Entities.order)).offset(offset).limit(limit);
  const roots = page.map((row) => row.id);

  const descendants =
    recursive && roots.length > 0
      ? await db.execute<{ id: string; parent_id: string }>(sql`
        WITH RECURSIVE descendants AS (
          SELECT ${Entities.id}, ${Entities.parentId}, ${Entities.order}
          FROM ${Entities}
          WHERE ${inArray(Entities.parentId, roots)} AND ${eq(Entities.state, EntityState.ACTIVE)}
          UNION ALL
          SELECT ${Entities.id}, ${Entities.parentId}, ${Entities.order}
          FROM ${Entities}
          JOIN descendants ON ${Entities.parentId} = descendants.id
          WHERE ${eq(Entities.state, EntityState.ACTIVE)}
        )
        SELECT id, parent_id FROM descendants ORDER BY "order" ASC
      `)
      : [];

  const walk = preorder(
    roots,
    descendants.map((row) => ({ id: row.id, parentId: row.parent_id })),
  );
  if (walk.length > LIST_ROWS_MAX) {
    return toolFailure(
      'error',
      `결과가 너무 커요(${walk.length}행) — limit을 줄이거나 recursive를 끄고 폴더별로 보세요(한 번에 ${LIST_ROWS_MAX}행까지).`,
    );
  }

  const entityIds = walk.map((node) => node.id);
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
    folder: scope,
    items: walk.flatMap((node): EntityItem[] => {
      const doc = documentByEntity.get(node.id);
      if (doc) return [{ kind: 'document', id: doc.id, entityId: node.id, title: doc.title, subtitle: doc.subtitle, depth: node.depth }];
      const folder = folderByEntity.get(node.id);
      return folder ? [{ kind: 'folder', id: folder.id, entityId: node.id, name: folder.name, depth: node.depth }] : [];
    }),
    range: { offset, end: offset + roots.length, total },
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

const folderEntityIds = async (ctx: PrismToolContext, folderIds: string[]): Promise<Map<string, string> | null> => {
  const unique = [...new Set(folderIds)];
  if (unique.length === 0) return new Map();

  const rows = await ctx.executor
    .select({ folderId: Folders.id, entityId: Folders.entityId })
    .from(Folders)
    .innerJoin(Entities, eq(Folders.entityId, Entities.id))
    .where(and(inArray(Folders.id, unique), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)));
  if (rows.length !== unique.length) return null;

  return new Map(rows.map((row) => [row.folderId, row.entityId]));
};

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

const resolveEntityIdMap = async (ctx: PrismToolContext, ids: string[]): Promise<Map<string, string> | null> => {
  const unique = [...new Set(ids)];
  const refs = await resolveEntityRefs(ctx, unique, EntityState.ACTIVE);
  if (refs.size !== unique.length) return null;

  return new Map([...refs].map(([id, ref]) => [id, ref.entityId]));
};

const resolveEntityIds = async (ctx: PrismToolContext, ids: string[]): Promise<string[] | null> => {
  const map = await resolveEntityIdMap(ctx, ids);
  if (map === null) return null;

  return [...new Set(map.values())];
};

const scopedNotes = async (ctx: PrismToolContext, noteIds: string[]): Promise<boolean> => {
  const unique = [...new Set(noteIds)];
  const rows = await ctx.executor
    .select({ id: Notes.id })
    .from(Notes)
    .where(and(inArray(Notes.id, unique), eq(Notes.siteId, ctx.siteId), eq(Notes.userId, ctx.userId), eq(Notes.state, NoteState.ACTIVE)));

  return rows.length === unique.length;
};

const createFolders = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = CreateFoldersInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const parents = await folderEntityIds(
    ctx,
    parsed.data.items.flatMap((item) => (item.parentFolderId === undefined ? [] : [item.parentFolderId])),
  );
  if (parents === null) return toolFailure('error', '상위 폴더를 찾지 못했어요 — list-entities로 폴더를 다시 확인하세요.');

  const folders = [];
  for (const item of parsed.data.items) {
    const folder = await createFolderCore(ctx.executor, {
      userId: ctx.userId,
      siteId: ctx.siteId,
      parentEntityId: item.parentFolderId === undefined ? null : (parents.get(item.parentFolderId) ?? null),
      name: item.name,
    });
    folders.push({ folderId: folder.id, name: item.name, entityId: folder.entityId });
  }

  return { ok: true, folders };
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

const createDocuments = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = CreateDocumentsInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const parents = await folderEntityIds(
    ctx,
    parsed.data.items.flatMap((item) => (item.folderId === undefined ? [] : [item.folderId])),
  );
  if (parents === null) return toolFailure('error', NOT_FOUND_FOLDER);

  const documents = [];
  for (const item of parsed.data.items) {
    const document = await createDocumentCore(ctx.executor, {
      userId: ctx.userId,
      siteId: ctx.siteId,
      parentEntityId: item.folderId === undefined ? null : (parents.get(item.folderId) ?? null),
    });
    documents.push({ documentId: document.id, entityId: document.entityId });
  }

  return { ok: true, documents };
};

const renameFolders = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = RenameFoldersInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  const folders = await folderEntityIds(
    ctx,
    parsed.data.items.map((item) => item.folderId),
  );
  if (folders === null) return toolFailure('error', NOT_FOUND_FOLDER);

  for (const item of parsed.data.items) {
    await renameFolderCore(ctx.executor, { userId: ctx.userId, folderId: item.folderId, name: item.name });
  }

  return { ok: true, count: parsed.data.items.length };
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

const duplicateDocuments = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = DuplicateDocumentsInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const unique = [...new Set(parsed.data.documentIds)];
  const owned = await ctx.executor
    .select({ id: Documents.id })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(and(inArray(Documents.id, unique), eq(Entities.siteId, ctx.siteId), eq(Entities.state, EntityState.ACTIVE)));
  if (owned.length !== unique.length) {
    return toolFailure('error', '일부 문서를 찾지 못했어요 — search-entities나 list-entities로 다시 찾아보세요.');
  }

  const documents = [];
  for (const documentId of parsed.data.documentIds) {
    const document = await duplicateDocumentCore(ctx.executor, { userId: ctx.userId, documentId });
    documents.push({ documentId: document.id, entityId: document.entityId });
  }

  return { ok: true, documents };
};

const updateIcons = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = UpdateIconsInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  if (parsed.data.items.some((item) => item.icon === undefined && item.iconColor === undefined)) {
    return toolFailure('error', '아이콘이나 색 중 하나는 줘야 해요.');
  }
  if (parsed.data.items.some((item) => !validIcon(item.icon, item.iconColor))) {
    return toolFailure('error', '그 아이콘이나 색은 없어요 — 목록에 있는 이름만 쓸 수 있어요.');
  }
  const entityIds = await resolveEntityIdMap(
    ctx,
    parsed.data.items.map((item) => item.id),
  );
  if (entityIds === null) return toolFailure('error', NOT_FOUND_TARGETS);

  for (const item of parsed.data.items) {
    const entityId = entityIds.get(item.id);
    if (entityId === undefined) return toolFailure('error', NOT_FOUND_TARGETS);
    await updateEntityIconCore(ctx.executor, { userId: ctx.userId, entityId, icon: item.icon, iconColor: item.iconColor });
  }

  return { ok: true, count: parsed.data.items.length };
};

const recoverEntities = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = RecoverEntitiesInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  const unique = [...new Set(parsed.data.ids)];
  const refs = await resolveEntityRefs(ctx, unique, EntityState.DELETED);
  const floor = dayjs().subtract(30, 'days');
  const recoverable = [...refs.values()].filter((ref) => ref.deletedAt !== null && ref.deletedAt.isAfter(floor));
  if (recoverable.length !== unique.length) {
    return toolFailure('error', '휴지통에서 일부 항목을 찾지 못했어요 — list-trash로 다시 확인하세요.');
  }

  const entityIds = [...new Set(recoverable.map((ref) => ref.entityId))];
  for (const entityId of entityIds) {
    await recoverEntityCore(ctx.executor, { userId: ctx.userId, entityId });
  }

  return { ok: true, count: entityIds.length };
};

const createNotes = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = CreateNotesInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  await assertActiveSubscription({ userId: ctx.userId });

  const items = parsed.data.items.map((item) => ({ content: item.content, color: item.color ?? NOTE_DEFAULT_COLOR }));
  if (items.some((item) => !NOTE_COLORS.includes(item.color))) return toolFailure('error', INVALID_COLOR);

  const noteIds = [];
  for (const item of items) {
    const note = await createNoteCore(ctx.executor, {
      userId: ctx.userId,
      siteId: ctx.siteId,
      content: item.content,
      color: item.color,
      entityIds: [],
    });
    noteIds.push(note.id);
  }

  return { ok: true, noteIds };
};

const updateNotes = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = UpdateNotesInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  if (parsed.data.items.some((item) => item.color !== undefined && !NOTE_COLORS.includes(item.color))) {
    return toolFailure('error', INVALID_COLOR);
  }
  const noteIds = parsed.data.items.map((item) => item.noteId);
  if (!(await scopedNotes(ctx, noteIds))) return toolFailure('error', NOT_FOUND_NOTE);

  for (const item of parsed.data.items) {
    await updateNoteCore(ctx.executor, { userId: ctx.userId, ...item });
  }

  return { ok: true, count: parsed.data.items.length };
};

const resolveNoteLinks = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = NoteLinksInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  const noteIds = parsed.data.items.map((item) => item.noteId);
  if (!(await scopedNotes(ctx, noteIds))) return toolFailure('error', NOT_FOUND_NOTE);
  const entityIds = await resolveEntityIdMap(
    ctx,
    parsed.data.items.map((item) => item.id),
  );
  if (entityIds === null) return toolFailure('error', NOT_FOUND_TARGETS);

  const links = [];
  for (const item of parsed.data.items) {
    const entityId = entityIds.get(item.id);
    if (entityId === undefined) return toolFailure('error', NOT_FOUND_TARGETS);
    links.push({ noteId: item.noteId, entityId });
  }

  return links;
};

const attachNotes = async (ctx: PrismToolContext, input: unknown) => {
  const links = await resolveNoteLinks(ctx, input);
  if (!Array.isArray(links)) return links;

  for (const link of links) {
    await addNoteEntityCore(ctx.executor, { userId: ctx.userId, ...link });
  }

  return { ok: true, count: links.length };
};

const detachNotes = async (ctx: PrismToolContext, input: unknown) => {
  const links = await resolveNoteLinks(ctx, input);
  if (!Array.isArray(links)) return links;

  for (const link of links) {
    await removeNoteEntityCore(ctx.executor, { userId: ctx.userId, ...link });
  }

  return { ok: true, count: links.length };
};

const setGoals = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = SetGoalsInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const entityIds = await resolveEntityIdMap(
    ctx,
    parsed.data.items.flatMap((item) => (item.id === undefined ? [] : [item.id])),
  );
  if (entityIds === null) return toolFailure('error', NOT_FOUND_TARGETS);

  const goals = [];
  for (const item of parsed.data.items) {
    if (item.id === undefined) {
      goals.push({ entityId: null, targetCharacterCount: item.targetCharacterCount, dueAt: null });
      continue;
    }
    const entityId = entityIds.get(item.id);
    if (entityId === undefined) return toolFailure('error', NOT_FOUND_TARGETS);
    const dueAt = item.dueAt === undefined ? null : kstDueDate(item.dueAt);
    if (dueAt === null && item.dueAt !== undefined) {
      return toolFailure('error', '그 날짜는 달력에 없어요 — YYYY-MM-DD로 실제 날짜를 주세요.');
    }
    goals.push({ entityId, targetCharacterCount: item.targetCharacterCount, dueAt });
  }

  for (const goal of goals) {
    if (goal.entityId === null) {
      await upsertUserGoalCore(ctx.executor, { userId: ctx.userId, targetCharacterCount: goal.targetCharacterCount });
    } else {
      await upsertEntityGoalCore(ctx.executor, {
        userId: ctx.userId,
        entityId: goal.entityId,
        targetCharacterCount: goal.targetCharacterCount,
        dueAt: goal.dueAt,
      });
    }
  }

  return { ok: true, count: goals.length };
};

const deleteNotes = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = DeleteNotesInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);
  const noteIds = [...new Set(parsed.data.noteIds)];
  if (!(await scopedNotes(ctx, noteIds))) return toolFailure('error', NOT_FOUND_NOTE);

  for (const noteId of noteIds) {
    await deleteNoteCore(ctx.executor, { userId: ctx.userId, noteId });
  }

  return { ok: true, count: noteIds.length };
};

const deleteGoals = async (ctx: PrismToolContext, input: unknown) => {
  const parsed = DeleteGoalsInput.safeParse(input);
  if (!parsed.success) return toolFailure('error', ERROR_MESSAGE);

  const scopedIds = [...new Set(parsed.data.items.flatMap((item) => (item.id === undefined ? [] : [item.id])))];
  const entityIds = await resolveEntityIdMap(ctx, scopedIds);
  if (entityIds === null) return toolFailure('error', NOT_FOUND_TARGETS);

  const targets = [...new Set(scopedIds.map((id) => entityIds.get(id)).filter((entityId) => entityId !== undefined))];
  const userGoal = parsed.data.items.some((item) => item.id === undefined);

  if (userGoal) {
    await deleteUserGoalCore(ctx.executor, { userId: ctx.userId });
  }
  for (const entityId of targets) {
    await deleteEntityGoalCore(ctx.executor, { userId: ctx.userId, entityId });
  }

  return { ok: true, count: targets.length + (userGoal ? 1 : 0) };
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
  'create-folders': createFolders,
  'create-documents': createDocuments,
  'rename-folders': renameFolders,
  'move-entities': moveEntities,
  'duplicate-documents': duplicateDocuments,
  'create-notes': createNotes,
  'update-notes': updateNotes,
  'attach-notes': attachNotes,
  'detach-notes': detachNotes,
  'set-goals': setGoals,
  'update-icons': updateIcons,
  'recover-entities': recoverEntities,
  'delete-entities': deleteEntities,
  'delete-notes': deleteNotes,
  'delete-goals': deleteGoals,
  'update-sharing': updateSharing,
};
