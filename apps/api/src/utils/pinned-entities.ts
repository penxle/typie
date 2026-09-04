import { EntityState, EntityType } from '@typie/lib/enums';
import { and, asc, eq, inArray, isNotNull } from 'drizzle-orm';
import { Entities } from '#/db/schemas/tables.ts';
import { generateFractionalOrder } from './order.ts';
import type { Database } from '#/db/index.ts';

export const PINNABLE_ENTITY_TYPES: readonly EntityType[] = [EntityType.DOCUMENT, EntityType.FOLDER];

export const isPinnableEntityType = (type: EntityType) => PINNABLE_ENTITY_TYPES.includes(type);

export const resolvePinSiteId = (entities: readonly { siteId: string }[]) => {
  const siteId = entities[0]?.siteId ?? null;
  return siteId !== null && entities.every((entity) => entity.siteId === siteId) ? siteId : null;
};

export const buildPinnedEntitiesBatchQuery = (executor: Database, input: { siteIds: string[] }) =>
  executor
    .select()
    .from(Entities)
    .where(and(inArray(Entities.siteId, input.siteIds), eq(Entities.state, EntityState.ACTIVE), isNotNull(Entities.pinnedOrder)))
    .orderBy(asc(Entities.siteId), asc(Entities.pinnedOrder));

export const generatePinnedOrders = (input: { count: number; lowerOrder: string | null; upperOrder: string | null }) => {
  const orders: string[] = [];
  let lower = input.lowerOrder;

  for (let index = 0; index < input.count; index++) {
    const order = generateFractionalOrder({ lower, upper: input.upperOrder });
    orders.push(order);
    lower = order;
  }

  return orders;
};
