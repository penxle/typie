import assert from 'node:assert/strict';
import test from 'node:test';
import { InAppPurchaseRecordState } from '@typie/lib/enums';
import {
  deriveGoogleOrderChain,
  googleMoneyToDecimal,
  mapAppleTransaction,
  mapGoogleOrder,
  milliunitsToDecimal,
  sumGoogleMoneyDecimal,
} from './iap-ingest-core.ts';
import type { JWSTransactionDecodedPayload } from '@apple/app-store-server-library';
import type { androidpublisher_v3 } from '@googleapis/androidpublisher';

const appleTransaction = (overrides: Partial<JWSTransactionDecodedPayload> = {}): JWSTransactionDecodedPayload => ({
  transactionId: '1000000000000001',
  originalTransactionId: '1000000000000000',
  productId: 'product.monthly',
  price: 6_900_000,
  currency: 'KRW',
  purchaseDate: 1_749_721_126_000,
  environment: 'Production',
  inAppOwnershipType: 'PURCHASED',
  appAccountToken: '00000000-0000-4000-8000-000000000001',
  ...overrides,
});

const googleOrder = (overrides: Partial<androidpublisher_v3.Schema$Order> = {}): androidpublisher_v3.Schema$Order => ({
  orderId: 'GPA.0000-1111-2222-33333',
  state: 'PROCESSED',
  createTime: '2026-07-25T02:58:32.341Z',
  total: { currencyCode: 'KRW', units: '2900', nanos: 0 },
  lineItems: [{ productId: 'product.monthly' }],
  ...overrides,
});

test('milliunitsToDecimal converts milliunits to decimal strings', () => {
  assert.equal(milliunitsToDecimal(6_900_000), '6900');
  assert.equal(milliunitsToDecimal(4990), '4.99');
  assert.equal(milliunitsToDecimal(500), '0.5');
  assert.equal(milliunitsToDecimal(0), '0');
});

test('googleMoneyToDecimal handles missing units and fractional nanos', () => {
  assert.equal(googleMoneyToDecimal({ units: '2900', nanos: 0 }), '2900');
  assert.equal(googleMoneyToDecimal({ units: undefined, nanos: 0 }), '0');
  assert.equal(googleMoneyToDecimal({ units: '4', nanos: 990_000_000 }), '4.99');
  assert.equal(googleMoneyToDecimal(undefined), '0');
});

test('sumGoogleMoneyDecimal carries nanos into units', () => {
  assert.equal(
    sumGoogleMoneyDecimal([
      { units: '1', nanos: 500_000_000 },
      { units: '2', nanos: 600_000_000 },
    ]),
    '4.1',
  );
  assert.equal(sumGoogleMoneyDecimal([]), '0');
});

test('deriveGoogleOrderChain enumerates the renewal chain including the base order', () => {
  assert.deepEqual(deriveGoogleOrderChain('GPA.0000-1111-2222-33333'), ['GPA.0000-1111-2222-33333']);
  assert.deepEqual(deriveGoogleOrderChain('GPA.0000-1111-2222-33333..0'), ['GPA.0000-1111-2222-33333', 'GPA.0000-1111-2222-33333..0']);
  assert.deepEqual(deriveGoogleOrderChain('GPA.0000-1111-2222-33333..2'), [
    'GPA.0000-1111-2222-33333',
    'GPA.0000-1111-2222-33333..0',
    'GPA.0000-1111-2222-33333..1',
    'GPA.0000-1111-2222-33333..2',
  ]);
  assert.deepEqual(deriveGoogleOrderChain('GPA.0000-1111-2222-33333..x'), ['GPA.0000-1111-2222-33333..x']);
});

test('deriveGoogleOrderChain refuses enumeration beyond the suffix limit', () => {
  assert.equal(deriveGoogleOrderChain('GPA.0000-1111-2222-33333..1000').length, 1002);
  assert.deepEqual(deriveGoogleOrderChain('GPA.0000-1111-2222-33333..1001'), ['GPA.0000-1111-2222-33333..1001']);
  assert.deepEqual(deriveGoogleOrderChain('GPA.0000-1111-2222-33333..1000000000'), ['GPA.0000-1111-2222-33333..1000000000']);
});

test('mapAppleTransaction maps a production purchase to a PAID record', () => {
  const mapping = mapAppleTransaction(appleTransaction(), { allowSandbox: false });

  assert.equal(mapping.kind, 'record');
  if (mapping.kind !== 'record') return;
  assert.equal(mapping.record.identifier, '1000000000000001');
  assert.equal(mapping.record.state, InAppPurchaseRecordState.PAID);
  assert.equal(mapping.record.amount, '6900');
  assert.equal(mapping.record.currency, 'KRW');
  assert.equal(mapping.record.refundedAmount, null);
  assert.equal(mapping.record.purchasedAt.valueOf(), 1_749_721_126_000);
  assert.equal(mapping.appAccountToken, '00000000-0000-4000-8000-000000000001');
});

test('mapAppleTransaction records zero-price transactions', () => {
  const mapping = mapAppleTransaction(appleTransaction({ price: 0 }), { allowSandbox: false });

  assert.equal(mapping.kind, 'record');
  if (mapping.kind !== 'record') return;
  assert.equal(mapping.record.amount, '0');
});

test('mapAppleTransaction gates sandbox by allowSandbox', () => {
  const sandbox = appleTransaction({ environment: 'Sandbox' });

  assert.equal(mapAppleTransaction(sandbox, { allowSandbox: false }).kind, 'skip');
  assert.equal(mapAppleTransaction(sandbox, { allowSandbox: true }).kind, 'record');
});

test('mapAppleTransaction skips family-shared transactions', () => {
  assert.equal(mapAppleTransaction(appleTransaction({ inAppOwnershipType: 'FAMILY_SHARED' }), { allowSandbox: false }).kind, 'skip');
});

test('mapAppleTransaction flags missing price as invalid', () => {
  assert.equal(mapAppleTransaction(appleTransaction({ price: undefined }), { allowSandbox: false }).kind, 'invalid');
});

test('mapAppleTransaction stamps refunds with the full amount', () => {
  const mapping = mapAppleTransaction(appleTransaction({ revocationDate: 1_750_000_000_000, revocationReason: 0 }), {
    allowSandbox: false,
  });

  assert.equal(mapping.kind, 'record');
  if (mapping.kind !== 'record') return;
  assert.equal(mapping.record.state, InAppPurchaseRecordState.REFUNDED);
  assert.equal(mapping.record.refundedAmount, '6900');
  assert.equal(mapping.record.refundedAt?.valueOf(), 1_750_000_000_000);
});

test('mapGoogleOrder maps a processed order to a PAID record', () => {
  const mapping = mapGoogleOrder(googleOrder());

  assert.equal(mapping.kind, 'record');
  if (mapping.kind !== 'record') return;
  assert.equal(mapping.record.identifier, 'GPA.0000-1111-2222-33333');
  assert.equal(mapping.record.state, InAppPurchaseRecordState.PAID);
  assert.equal(mapping.record.amount, '2900');
  assert.equal(mapping.record.productId, 'product.monthly');
});

test('mapGoogleOrder treats undefined total units as zero', () => {
  const mapping = mapGoogleOrder(googleOrder({ total: { currencyCode: 'KRW', nanos: 0 } }));

  assert.equal(mapping.kind, 'record');
  if (mapping.kind !== 'record') return;
  assert.equal(mapping.record.amount, '0');
});

test('mapGoogleOrder skips unsettled orders and flags unknown states', () => {
  assert.equal(mapGoogleOrder(googleOrder({ state: 'PENDING' })).kind, 'skip');
  assert.equal(mapGoogleOrder(googleOrder({ state: 'CANCELED' })).kind, 'skip');
  assert.equal(mapGoogleOrder(googleOrder({ state: 'SOMETHING_NEW' })).kind, 'unknown-state');
});

test('mapGoogleOrder flags missing currency as invalid', () => {
  assert.equal(mapGoogleOrder(googleOrder({ total: { units: '2900' } })).kind, 'invalid');
});

test('mapGoogleOrder stamps full refunds from the refund event', () => {
  const mapping = mapGoogleOrder(
    googleOrder({
      state: 'REFUNDED',
      orderHistory: {
        refundEvent: { eventTime: '2026-08-01T00:00:00Z', refundDetails: { total: { currencyCode: 'KRW', units: '2900', nanos: 0 } } },
      },
    }),
  );

  assert.equal(mapping.kind, 'record');
  if (mapping.kind !== 'record') return;
  assert.equal(mapping.record.state, InAppPurchaseRecordState.REFUNDED);
  assert.equal(mapping.record.refundedAmount, '2900');
  assert.equal(mapping.record.refundedAt?.toISOString(), '2026-08-01T00:00:00.000Z');
});

test('mapGoogleOrder falls back to the full amount when the refund event lacks details', () => {
  const mapping = mapGoogleOrder(googleOrder({ state: 'REFUNDED' }));

  assert.equal(mapping.kind, 'record');
  if (mapping.kind !== 'record') return;
  assert.equal(mapping.record.refundedAmount, '2900');
  assert.equal(mapping.record.refundedAt, null);
});

test('mapGoogleOrder leaves refundedAt null when the store reports no refund timestamps', () => {
  const mapping = mapGoogleOrder(
    googleOrder({
      state: 'PARTIALLY_REFUNDED',
      orderHistory: { partialRefundEvents: [{ refundDetails: { total: { currencyCode: 'KRW', units: '1000', nanos: 0 } } }] },
    }),
  );

  assert.equal(mapping.kind, 'record');
  if (mapping.kind !== 'record') return;
  assert.equal(mapping.record.state, InAppPurchaseRecordState.REFUNDED);
  assert.equal(mapping.record.refundedAmount, '1000');
  assert.equal(mapping.record.refundedAt, null);
});

test('mapGoogleOrder sums partial refund events', () => {
  const mapping = mapGoogleOrder(
    googleOrder({
      state: 'PARTIALLY_REFUNDED',
      orderHistory: {
        partialRefundEvents: [
          { processTime: '2026-08-02T00:00:00Z', refundDetails: { total: { currencyCode: 'KRW', units: '1000', nanos: 0 } } },
          { processTime: '2026-08-03T00:00:00Z', refundDetails: { total: { currencyCode: 'KRW', units: '500', nanos: 0 } } },
        ],
      },
    }),
  );

  assert.equal(mapping.kind, 'record');
  if (mapping.kind !== 'record') return;
  assert.equal(mapping.record.state, InAppPurchaseRecordState.REFUNDED);
  assert.equal(mapping.record.refundedAmount, '1500');
  assert.equal(mapping.record.refundedAt?.toISOString(), '2026-08-03T00:00:00.000Z');
});

test('mapGoogleOrder flags partial refunds without usable refund details as invalid', () => {
  const missingHistory = mapGoogleOrder(googleOrder({ state: 'PARTIALLY_REFUNDED' }));

  assert.equal(missingHistory.kind, 'invalid');
  if (missingHistory.kind !== 'invalid') return;
  assert.equal(missingHistory.reason, 'missing-refund-detail');

  assert.equal(mapGoogleOrder(googleOrder({ state: 'PARTIALLY_REFUNDED', orderHistory: { partialRefundEvents: [] } })).kind, 'invalid');

  assert.equal(
    mapGoogleOrder(
      googleOrder({ state: 'PARTIALLY_REFUNDED', orderHistory: { partialRefundEvents: [{ processTime: '2026-08-02T00:00:00Z' }] } }),
    ).kind,
    'invalid',
  );
});
