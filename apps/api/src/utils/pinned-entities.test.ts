import assert from 'node:assert/strict';
import test from 'node:test';
import { EntityType } from '@typie/lib/enums';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import { generateFractionalOrder } from './order.ts';
import { buildPinnedEntitiesBatchQuery, generatePinnedOrders, isPinnableEntityType, resolvePinSiteId } from './pinned-entities.ts';
import type { Database } from '#/db/index.ts';

const database = drizzle.mock({ schema: tables }) as unknown as Database;

test('pinned entities are site scoped active rows ordered by pinned order', () => {
  const query = buildPinnedEntitiesBatchQuery(database, { siteIds: ['S0FIRST', 'S0SECOND'] }).toSQL();

  assert.match(query.sql, /"entities"\."site_id" in \(/);
  assert.match(query.sql, /"entities"\."state" = /);
  assert.match(query.sql, /"entities"\."pinned_order" is not null/);
  assert.match(query.sql, /order by "entities"\."site_id" asc, "entities"\."pinned_order" asc/);
  assert.deepEqual(query.params, ['S0FIRST', 'S0SECOND', 'ACTIVE']);
});

test('pinned orders are generated in sequence between the bounds', () => {
  const lower = generateFractionalOrder({ lower: null, upper: null });
  const upper = generateFractionalOrder({ lower, upper: null });

  const orders = generatePinnedOrders({ count: 3, lowerOrder: lower, upperOrder: upper });

  assert.equal(orders.length, 3);
  for (const [index, order] of orders.entries()) {
    assert.ok(order > lower && order < upper, `${order} must sit between ${lower} and ${upper}`);
    if (index > 0) assert.ok(order > orders[index - 1], `${order} must follow ${orders[index - 1]}`);
  }
});

test('pinned orders append after the lower bound when there is no upper bound', () => {
  const lower = generateFractionalOrder({ lower: null, upper: null });

  const orders = generatePinnedOrders({ count: 2, lowerOrder: lower, upperOrder: null });

  assert.ok(orders[0] > lower);
  assert.ok(orders[1] > orders[0]);
});

test('pinned orders start from nothing when the list is empty', () => {
  const orders = generatePinnedOrders({ count: 1, lowerOrder: null, upperOrder: null });

  assert.equal(orders.length, 1);
  assert.ok(orders[0].length > 0);
});

test('pinnable entity types are documents and folders only', () => {
  assert.equal(isPinnableEntityType(EntityType.DOCUMENT), true);
  assert.equal(isPinnableEntityType(EntityType.FOLDER), true);
  assert.equal(isPinnableEntityType(EntityType.DIVIDER), false);
});

test('pin targets must belong to a single site', () => {
  assert.equal(resolvePinSiteId([{ siteId: 'S0FIRST' }, { siteId: 'S0FIRST' }]), 'S0FIRST');
  assert.equal(resolvePinSiteId([{ siteId: 'S0FIRST' }, { siteId: 'S0SECOND' }]), null);
  assert.equal(resolvePinSiteId([]), null);
});
