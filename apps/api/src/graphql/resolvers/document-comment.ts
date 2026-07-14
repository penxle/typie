import { DocumentCommentState, DocumentCommentThreadState, EntityState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, asc, eq, inArray, isNotNull, isNull } from 'drizzle-orm';
import { filter, pipe } from 'graphql-yoga';
import { db, DocumentComments, DocumentCommentThreads, Documents, Entities, firstOrThrow, TableCode, validateDbId } from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { normalizeStableSelection } from '#/utils/comment-selection.ts';
import { assertDocumentCommentAccess } from '#/utils/permission.ts';
import { builder } from '../builder.ts';
import { Document, DocumentComment, DocumentCommentThread, User } from '../objects.ts';

/**
 * * Types
 */

DocumentCommentThread.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    user: t.expose('userId', { type: User }),
    selection: t.expose('selection', { type: 'JSON' }),
    state: t.expose('state', { type: DocumentCommentThreadState }),
    resolved: t.boolean({ resolve: (self) => self.resolvedAt !== null }),
    resolvedAt: t.expose('resolvedAt', { type: 'DateTime', nullable: true }),
    resolvedBy: t.expose('resolvedBy', { type: User, nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),

    comments: t.field({
      type: [DocumentComment],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'DocumentCommentThread.comments',
          load: (threadIds: string[]) =>
            db
              .select({ threadId: DocumentComments.threadId, id: DocumentComments.id })
              .from(DocumentComments)
              .where(and(inArray(DocumentComments.threadId, threadIds), eq(DocumentComments.state, DocumentCommentState.ACTIVE)))
              .orderBy(asc(DocumentComments.createdAt), asc(DocumentComments.id)),
          key: ({ threadId }) => threadId,
          many: true,
        });

        const rows = await loader.load(self.id);
        // eslint-disable-next-line @typescript-eslint/no-explicit-any -- loadable ref resolves from IDs
        return rows.map((r) => r.id) as any;
      },
    }),
  }),
});

DocumentComment.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    content: t.exposeString('content'),
    user: t.expose('userId', { type: User }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
  }),
});

builder.objectFields(Document, (t) => ({
  commentThreads: t.field({
    type: [DocumentCommentThread],
    args: {
      resolved: t.arg.boolean({ required: false }),
    },
    resolve: async (document, args, ctx) => {
      if (!ctx.session) {
        return [];
      }

      try {
        await assertDocumentCommentAccess({ userId: ctx.session.userId, documentId: document.id });
      } catch (err) {
        if (err instanceof TypieError) {
          return [];
        }
        throw err;
      }

      const conditions = [
        eq(DocumentCommentThreads.documentId, document.id),
        eq(DocumentCommentThreads.state, DocumentCommentThreadState.ACTIVE),
      ];

      if (args.resolved === true) {
        conditions.push(isNotNull(DocumentCommentThreads.resolvedAt));
      } else if (args.resolved === false) {
        conditions.push(isNull(DocumentCommentThreads.resolvedAt));
      }

      return await db
        .select()
        .from(DocumentCommentThreads)
        .where(and(...conditions))
        .orderBy(asc(DocumentCommentThreads.createdAt), asc(DocumentCommentThreads.id));
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  createDocumentCommentThread: t.withAuth({ session: true }).fieldWithInput({
    type: DocumentCommentThread,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      selection: t.input.field({ type: 'JSON' }),
      content: t.input.string(),
      clientId: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      await assertDocumentCommentAccess({ userId: ctx.session.userId, documentId: input.documentId });

      const thread = await db.transaction(async (tx) => {
        const created = await tx
          .insert(DocumentCommentThreads)
          .values({
            documentId: input.documentId,
            userId: ctx.session.userId,
            selection: normalizeStableSelection(input.selection),
          })
          .returning()
          .then(firstOrThrow);

        await tx.insert(DocumentComments).values({
          threadId: created.id,
          userId: ctx.session.userId,
          content: input.content,
        });

        return created;
      });

      pubsub.publish('document:comment', input.documentId, { threadId: thread.id, originClientId: input.clientId });
      return thread;
    },
  }),

  createDocumentComment: t.withAuth({ session: true }).fieldWithInput({
    type: DocumentCommentThread,
    input: {
      threadId: t.input.id({ validate: validateDbId(TableCode.DOCUMENT_COMMENT_THREADS) }),
      content: t.input.string(),
      clientId: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const thread = await db
        .select({ documentId: DocumentCommentThreads.documentId, state: DocumentCommentThreads.state })
        .from(DocumentCommentThreads)
        .where(eq(DocumentCommentThreads.id, input.threadId))
        .then(firstOrThrow);

      if (thread.state !== DocumentCommentThreadState.ACTIVE) {
        throw new TypieError({ code: 'not_found' });
      }

      // 엔티티 ACTIVE + availability 게이트를 함께 검사한다.
      await assertDocumentCommentAccess({ userId: ctx.session.userId, documentId: thread.documentId });

      await db.transaction(async (tx) => {
        await tx.insert(DocumentComments).values({ threadId: input.threadId, userId: ctx.session.userId, content: input.content });
        // thread.updatedAt = 마지막 서브트리(코멘트) 변경 시각
        await tx.update(DocumentCommentThreads).set({ updatedAt: dayjs() }).where(eq(DocumentCommentThreads.id, input.threadId));
      });

      pubsub.publish('document:comment', thread.documentId, { threadId: input.threadId, originClientId: input.clientId });
      return input.threadId;
    },
  }),

  updateDocumentComment: t.withAuth({ session: true }).fieldWithInput({
    type: DocumentCommentThread,
    input: {
      commentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENT_COMMENTS) }),
      content: t.input.string(),
      clientId: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const comment = await db
        .select({
          userId: DocumentComments.userId,
          state: DocumentComments.state,
          threadId: DocumentComments.threadId,
          threadState: DocumentCommentThreads.state,
          documentId: DocumentCommentThreads.documentId,
          entityState: Entities.state,
        })
        .from(DocumentComments)
        .innerJoin(DocumentCommentThreads, eq(DocumentComments.threadId, DocumentCommentThreads.id))
        .innerJoin(Documents, eq(DocumentCommentThreads.documentId, Documents.id))
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(DocumentComments.id, input.commentId))
        .then(firstOrThrow);

      if (comment.state !== DocumentCommentState.ACTIVE || comment.threadState !== DocumentCommentThreadState.ACTIVE) {
        throw new TypieError({ code: 'not_found' });
      }

      if (comment.entityState !== EntityState.ACTIVE) {
        throw new TypieError({ code: 'permission_denied' });
      }

      if (comment.userId !== ctx.session.userId) {
        throw new TypieError({ code: 'permission_denied' });
      }

      await db.transaction(async (tx) => {
        await tx
          .update(DocumentComments)
          .set({ content: input.content, updatedAt: dayjs() })
          .where(eq(DocumentComments.id, input.commentId));
        await tx.update(DocumentCommentThreads).set({ updatedAt: dayjs() }).where(eq(DocumentCommentThreads.id, comment.threadId));
      });

      pubsub.publish('document:comment', comment.documentId, { threadId: comment.threadId, originClientId: input.clientId });
      return comment.threadId;
    },
  }),

  deleteDocumentComment: t.withAuth({ session: true }).fieldWithInput({
    type: DocumentCommentThread,
    input: {
      commentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENT_COMMENTS) }),
      clientId: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const comment = await db
        .select({
          userId: DocumentComments.userId,
          state: DocumentComments.state,
          threadId: DocumentComments.threadId,
          threadState: DocumentCommentThreads.state,
          documentId: DocumentCommentThreads.documentId,
          entityState: Entities.state,
        })
        .from(DocumentComments)
        .innerJoin(DocumentCommentThreads, eq(DocumentComments.threadId, DocumentCommentThreads.id))
        .innerJoin(Documents, eq(DocumentCommentThreads.documentId, Documents.id))
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(DocumentComments.id, input.commentId))
        .then(firstOrThrow);

      if (comment.state !== DocumentCommentState.ACTIVE || comment.threadState !== DocumentCommentThreadState.ACTIVE) {
        throw new TypieError({ code: 'not_found' });
      }

      if (comment.entityState !== EntityState.ACTIVE) {
        throw new TypieError({ code: 'permission_denied' });
      }

      if (comment.userId !== ctx.session.userId) {
        const { isOwner } = await assertDocumentCommentAccess({ userId: ctx.session.userId, documentId: comment.documentId });
        if (!isOwner) {
          throw new TypieError({ code: 'permission_denied' });
        }
      }

      const root = await db
        .select({ id: DocumentComments.id })
        .from(DocumentComments)
        .where(eq(DocumentComments.threadId, comment.threadId))
        .orderBy(asc(DocumentComments.createdAt), asc(DocumentComments.id))
        .limit(1)
        .then(firstOrThrow);

      if (root.id === input.commentId) {
        throw new TypieError({ code: 'cannot_delete_root_comment', status: 400 });
      }

      await db.transaction(async (tx) => {
        await tx
          .update(DocumentComments)
          .set({ state: DocumentCommentState.DELETED, updatedAt: dayjs() })
          .where(eq(DocumentComments.id, input.commentId));
        await tx.update(DocumentCommentThreads).set({ updatedAt: dayjs() }).where(eq(DocumentCommentThreads.id, comment.threadId));
      });

      pubsub.publish('document:comment', comment.documentId, { threadId: comment.threadId, originClientId: input.clientId });
      return comment.threadId;
    },
  }),

  deleteDocumentCommentThread: t.withAuth({ session: true }).fieldWithInput({
    type: DocumentCommentThread,
    input: {
      threadId: t.input.id({ validate: validateDbId(TableCode.DOCUMENT_COMMENT_THREADS) }),
      clientId: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const thread = await loadThreadForManage(input.threadId, ctx.session.userId);

      const updated = await db
        .update(DocumentCommentThreads)
        .set({ state: DocumentCommentThreadState.DELETED, updatedAt: dayjs() })
        .where(eq(DocumentCommentThreads.id, input.threadId))
        .returning()
        .then(firstOrThrow);

      pubsub.publish('document:comment', thread.documentId, { threadId: input.threadId, originClientId: input.clientId });
      return updated;
    },
  }),

  resolveDocumentCommentThread: t.withAuth({ session: true }).fieldWithInput({
    type: DocumentCommentThread,
    input: {
      threadId: t.input.id({ validate: validateDbId(TableCode.DOCUMENT_COMMENT_THREADS) }),
      clientId: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const thread = await loadThreadForManage(input.threadId, ctx.session.userId);

      const updated = await db
        .update(DocumentCommentThreads)
        .set({ resolvedAt: dayjs(), resolvedBy: ctx.session.userId, updatedAt: dayjs() })
        .where(eq(DocumentCommentThreads.id, input.threadId))
        .returning()
        .then(firstOrThrow);

      pubsub.publish('document:comment', thread.documentId, { threadId: input.threadId, originClientId: input.clientId });
      return updated;
    },
  }),

  unresolveDocumentCommentThread: t.withAuth({ session: true }).fieldWithInput({
    type: DocumentCommentThread,
    input: {
      threadId: t.input.id({ validate: validateDbId(TableCode.DOCUMENT_COMMENT_THREADS) }),
      clientId: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const thread = await loadThreadForManage(input.threadId, ctx.session.userId);

      const updated = await db
        .update(DocumentCommentThreads)
        .set({ resolvedAt: null, resolvedBy: null, updatedAt: dayjs() })
        .where(eq(DocumentCommentThreads.id, input.threadId))
        .returning()
        .then(firstOrThrow);

      pubsub.publish('document:comment', thread.documentId, { threadId: input.threadId, originClientId: input.clientId });
      return updated;
    },
  }),
}));

/**
 * * Subscriptions
 */

builder.subscriptionFields((t) => ({
  documentCommentStream: t.withAuth({ session: true }).field({
    type: DocumentCommentThread,
    args: {
      documentId: t.arg.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      clientId: t.arg.string(),
    },
    subscribe: async (_, args, ctx) => {
      await assertDocumentCommentAccess({ userId: ctx.session.userId, documentId: args.documentId });

      return pipe(
        pubsub.subscribe('document:comment', args.documentId),
        filter((event) => event.originClientId !== args.clientId),
      );
    },
    resolve: (event) => event.threadId,
  }),
}));

/**
 * * Utils
 */

const loadThreadForManage = async (threadId: string, userId: string) => {
  const thread = await db
    .select({
      userId: DocumentCommentThreads.userId,
      documentId: DocumentCommentThreads.documentId,
      state: DocumentCommentThreads.state,
      entityState: Entities.state,
    })
    .from(DocumentCommentThreads)
    .innerJoin(Documents, eq(DocumentCommentThreads.documentId, Documents.id))
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(eq(DocumentCommentThreads.id, threadId))
    .then(firstOrThrow);

  if (thread.state !== DocumentCommentThreadState.ACTIVE) {
    throw new TypieError({ code: 'not_found' });
  }

  if (thread.entityState !== EntityState.ACTIVE) {
    throw new TypieError({ code: 'permission_denied' });
  }

  if (thread.userId === userId) {
    return thread;
  }

  const { isOwner } = await assertDocumentCommentAccess({ userId, documentId: thread.documentId });
  if (!isOwner) {
    throw new TypieError({ code: 'permission_denied' });
  }

  return thread;
};
