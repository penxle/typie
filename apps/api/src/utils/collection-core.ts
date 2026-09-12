import { and, asc, inArray, isNotNull } from 'drizzle-orm';
import { Collections, Publications } from '#/db/schemas/tables.ts';
import { publishedPublicationPredicate, publishedPublicationsBase } from './publication-view-core.ts';
import type { Database, Transaction } from '#/db/index.ts';

type Executor = Database | Transaction;

export const PIN_LIMIT = 3;
export const canPinMore = (currentCount: number) => currentCount < PIN_LIMIT;

export const buildCollectionsBySpaceQuery = (executor: Executor, input: { spaceIds: string[] }) =>
  executor.select().from(Collections).where(inArray(Collections.spaceId, input.spaceIds)).orderBy(asc(Collections.createdAt));

export const buildPinnedPublicationsQuery = (executor: Executor, input: { spaceIds: string[] }) =>
  publishedPublicationsBase(executor)
    .where(and(inArray(Publications.spaceId, input.spaceIds), publishedPublicationPredicate(), isNotNull(Publications.pinnedOrder)))
    .orderBy(asc(Publications.spaceId), asc(Publications.pinnedOrder));
