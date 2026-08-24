import { EntityState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { Entities, EntityGoals, firstOrThrow, UserGoals } from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { assertSitePermission } from './permission.ts';
import type { Dayjs } from 'dayjs';
import type { Database, Transaction } from '#/db/index.ts';

type UpsertUserGoalCoreArgs = {
  userId: string;
  targetCharacterCount: number;
};

export const upsertUserGoalCore = async (executor: Database | Transaction, args: UpsertUserGoalCoreArgs) => {
  if (args.targetCharacterCount <= 0) {
    throw new TypieError({ code: 'invalid_target_character_count' });
  }

  const today = dayjs.kst().startOf('day');

  await executor
    .insert(UserGoals)
    .values({ userId: args.userId, targetCharacterCount: args.targetCharacterCount, effectiveAt: today })
    .onConflictDoUpdate({
      target: [UserGoals.userId, UserGoals.effectiveAt],
      set: { targetCharacterCount: args.targetCharacterCount },
    });
};

type DeleteUserGoalCoreArgs = {
  userId: string;
};

export const deleteUserGoalCore = async (executor: Database | Transaction, args: DeleteUserGoalCoreArgs) => {
  const today = dayjs.kst().startOf('day');

  await executor
    .insert(UserGoals)
    .values({ userId: args.userId, targetCharacterCount: null, effectiveAt: today })
    .onConflictDoUpdate({
      target: [UserGoals.userId, UserGoals.effectiveAt],
      set: { targetCharacterCount: null },
    });
};

type UpsertEntityGoalCoreArgs = {
  userId: string;
  entityId: string;
  targetCharacterCount: number;
  dueAt: Dayjs | null;
};

export const upsertEntityGoalCore = async (executor: Database | Transaction, args: UpsertEntityGoalCoreArgs) => {
  if (args.targetCharacterCount <= 0) {
    throw new TypieError({ code: 'invalid_target_character_count' });
  }

  const entity = await executor
    .select({ siteId: Entities.siteId })
    .from(Entities)
    .where(and(eq(Entities.id, args.entityId), eq(Entities.state, EntityState.ACTIVE)))
    .then(firstOrThrow);

  await assertSitePermission({ userId: args.userId, siteId: entity.siteId });

  await executor
    .insert(EntityGoals)
    .values({ entityId: args.entityId, targetCharacterCount: args.targetCharacterCount, dueAt: args.dueAt })
    .onConflictDoUpdate({
      target: [EntityGoals.entityId],
      set: { targetCharacterCount: args.targetCharacterCount, dueAt: args.dueAt, updatedAt: dayjs() },
    });

  pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: args.entityId });
};

type DeleteEntityGoalCoreArgs = {
  userId: string;
  entityId: string;
};

export const deleteEntityGoalCore = async (executor: Database | Transaction, args: DeleteEntityGoalCoreArgs) => {
  const entity = await executor
    .select({ siteId: Entities.siteId })
    .from(Entities)
    .where(and(eq(Entities.id, args.entityId), eq(Entities.state, EntityState.ACTIVE)))
    .then(firstOrThrow);

  await assertSitePermission({ userId: args.userId, siteId: entity.siteId });

  await executor.delete(EntityGoals).where(eq(EntityGoals.entityId, args.entityId));

  pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: args.entityId });
};
