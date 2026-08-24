import { EntityState, NoteState } from '@typie/lib/enums';
import { NotFoundError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, asc, eq, inArray } from 'drizzle-orm';
import { Entities, first, firstOrThrow, firstOrThrowWith, NoteEntities, Notes } from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { generateFractionalOrder } from './order.ts';
import { assertSitePermission } from './permission.ts';
import { assertActiveSubscription } from './plan.ts';
import type { NoteStatus } from '@typie/lib/enums';
import type { Database, Transaction } from '#/db/index.ts';

type CreateNoteCoreArgs = {
  userId: string;
  siteId: string;
  content: string;
  color: string;
  entityIds: string[];
  clientId?: string | null;
};

export const createNoteCore = async (executor: Database | Transaction, args: CreateNoteCoreArgs) => {
  await assertSitePermission({ userId: args.userId, siteId: args.siteId });

  if (args.entityIds.length > 0) {
    const entities = await executor
      .select({ id: Entities.id })
      .from(Entities)
      .where(and(inArray(Entities.id, args.entityIds), eq(Entities.siteId, args.siteId), eq(Entities.state, EntityState.ACTIVE)));

    if (entities.length !== args.entityIds.length) {
      throw new NotFoundError();
    }
  }

  const firstNote = await executor
    .select({ order: Notes.order })
    .from(Notes)
    .where(and(eq(Notes.userId, args.userId), eq(Notes.state, NoteState.ACTIVE)))
    .orderBy(asc(Notes.order))
    .limit(1)
    .then(first);

  const order = generateFractionalOrder({ lower: null, upper: firstNote?.order });

  const note = await executor.transaction(async (tx) => {
    const note = await tx
      .insert(Notes)
      .values({
        userId: args.userId,
        siteId: args.siteId,
        content: args.content,
        color: args.color,
        order,
      })
      .returning()
      .then(firstOrThrow);

    if (args.entityIds.length > 0) {
      await tx.insert(NoteEntities).values(
        args.entityIds.map((entityId) => ({
          noteId: note.id,
          entityId,
        })),
      );
    }

    return note;
  });

  pubsub.publish('note:update', args.siteId, {
    kind: 'CREATED',
    noteId: note.id,
    originClientId: args.clientId ?? undefined,
  });

  return note;
};

type UpdateNoteCoreArgs = {
  userId: string;
  noteId: string;
  content?: string | null;
  color?: string | null;
  status?: NoteStatus | null;
  entityId?: string | null;
  clientId?: string | null;
};

export const updateNoteCore = async (executor: Database | Transaction, args: UpdateNoteCoreArgs) => {
  await assertActiveSubscription({ userId: args.userId });

  const updated = await executor.transaction(async (tx) => {
    const note = await tx
      .select()
      .from(Notes)
      .where(and(eq(Notes.id, args.noteId), eq(Notes.userId, args.userId), eq(Notes.state, NoteState.ACTIVE)))
      .for('update')
      .then(firstOrThrowWith(new NotFoundError()));

    const updated = await tx
      .update(Notes)
      .set({
        content: args.content ?? undefined,
        color: args.color ?? undefined,
        status: args.status ?? undefined,
        updatedAt: dayjs(),
      })
      .where(eq(Notes.id, note.id))
      .returning()
      .then(firstOrThrow);

    if (args.entityId) {
      await tx
        .select({ id: Entities.id })
        .from(Entities)
        .where(and(eq(Entities.id, args.entityId), eq(Entities.state, EntityState.ACTIVE), eq(Entities.siteId, note.siteId)))
        .then(firstOrThrow);

      await tx.delete(NoteEntities).where(eq(NoteEntities.noteId, note.id));
      await tx.insert(NoteEntities).values({ noteId: note.id, entityId: args.entityId });
    }

    return updated;
  });

  pubsub.publish('note:update', updated.siteId, {
    kind: 'UPDATED',
    noteId: updated.id,
    originClientId: args.clientId ?? undefined,
  });

  return updated;
};

type DeleteNoteCoreArgs = {
  userId: string;
  noteId: string;
  clientId?: string | null;
};

export const deleteNoteCore = async (executor: Database | Transaction, args: DeleteNoteCoreArgs) => {
  const deleted = await executor
    .update(Notes)
    .set({ state: NoteState.DELETED, updatedAt: dayjs() })
    .where(and(eq(Notes.id, args.noteId), eq(Notes.userId, args.userId), eq(Notes.state, NoteState.ACTIVE)))
    .returning()
    .then(firstOrThrowWith(new NotFoundError()));

  pubsub.publish('note:update', deleted.siteId, {
    kind: 'DELETED',
    noteId: deleted.id,
    originClientId: args.clientId ?? undefined,
  });

  return deleted;
};

type NoteEntityCoreArgs = {
  userId: string;
  noteId: string;
  entityId: string;
  clientId?: string | null;
};

export const addNoteEntityCore = async (executor: Database | Transaction, args: NoteEntityCoreArgs) => {
  const note = await executor
    .select()
    .from(Notes)
    .where(and(eq(Notes.id, args.noteId), eq(Notes.userId, args.userId), eq(Notes.state, NoteState.ACTIVE)))
    .then(firstOrThrowWith(new NotFoundError()));

  await assertActiveSubscription({ userId: args.userId });

  await executor
    .select({ id: Entities.id })
    .from(Entities)
    .where(and(eq(Entities.id, args.entityId), eq(Entities.state, EntityState.ACTIVE)))
    .then(firstOrThrow);

  await executor.insert(NoteEntities).values({ noteId: note.id, entityId: args.entityId }).onConflictDoNothing();

  pubsub.publish('note:update', note.siteId, {
    kind: 'UPDATED',
    noteId: note.id,
    originClientId: args.clientId ?? undefined,
  });

  return note;
};

export const removeNoteEntityCore = async (executor: Database | Transaction, args: NoteEntityCoreArgs) => {
  const note = await executor
    .select()
    .from(Notes)
    .where(and(eq(Notes.id, args.noteId), eq(Notes.userId, args.userId), eq(Notes.state, NoteState.ACTIVE)))
    .then(firstOrThrowWith(new NotFoundError()));

  await assertActiveSubscription({ userId: args.userId });

  await executor.delete(NoteEntities).where(and(eq(NoteEntities.noteId, note.id), eq(NoteEntities.entityId, args.entityId)));

  pubsub.publish('note:update', note.siteId, {
    kind: 'UPDATED',
    noteId: note.id,
    originClientId: args.clientId ?? undefined,
  });

  return note;
};
