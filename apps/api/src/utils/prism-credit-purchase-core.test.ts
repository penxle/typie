import assert from 'node:assert/strict';
import { test } from 'node:test';
import { PrismCreditPack } from '@typie/lib/enums';
import { PRISM_CREDIT_PACKS } from '@typie/prism';
import {
  allocateCancels,
  classifyReconcile,
  quoteRemainder,
  quoteWithdrawal,
  REMAINDER_REFUND_RATE,
  WITHDRAWAL_WINDOW_MS,
} from './prism-credit-purchase-core.ts';
import type { LedgerRow, PurchaseRow, RefundRow } from './prism-credit-purchase-core.ts';

const DAY = 24 * 60 * 60 * 1000;
const T0 = 1_700_000_000_000;

const purchase = (over: Partial<PurchaseRow> = {}): PurchaseRow => ({
  id: 'PRCP1',
  paymentKey: 'PRCP1',
  price: 14_900,
  credits: 300,
  bonusCredits: 30,
  state: 'PAID',
  paidAt: T0,
  ...over,
});

const entry = (kind: LedgerRow['kind'], paidDelta: number, freeDelta: number, createdAt: number, key: string | null = null): LedgerRow => ({
  kind,
  paidDelta,
  freeDelta,
  key,
  createdAt,
});

const bought = (p: PurchaseRow, at: number): LedgerRow[] => [
  entry('PURCHASE', p.credits * 1000, 0, at, p.id),
  ...(p.bonusCredits > 0 ? [entry('BONUS', 0, p.bonusCredits * 1000, at, p.id)] : []),
];

test('격자 거울: @typie/lib PrismCreditPack 키 = PRISM_CREDIT_PACKS 팩 키', () => {
  assert.deepEqual(
    Object.keys(PrismCreditPack),
    PRISM_CREDIT_PACKS.map((p) => p.pack),
  );
});

test('WITHDRAWAL: 7일 안·미사용 → 전액, 원장은 유상 전량·보너스 회수', () => {
  const p = purchase();
  const quote = quoteWithdrawal({ purchase: p, entries: bought(p, T0), refunds: [], now: T0 + 3 * DAY });
  assert.deepEqual(quote, {
    eligible: true,
    amount: 14_900,
    delta: { paidDelta: -300_000, freeDelta: -30_000 },
    cancels: [{ purchaseId: 'PRCP1', paymentKey: 'PRCP1', amount: 14_900, status: 'planned' }],
    shortfall: 0,
  });
});

test('WITHDRAWAL: 경계 — 정확히 7일은 가능, 7일+1ms는 window_expired', () => {
  const p = purchase();
  assert.equal(quoteWithdrawal({ purchase: p, entries: bought(p, T0), refunds: [], now: T0 + WITHDRAWAL_WINDOW_MS }).eligible, true);
  assert.deepEqual(quoteWithdrawal({ purchase: p, entries: bought(p, T0), refunds: [], now: T0 + WITHDRAWAL_WINDOW_MS + 1 }), {
    eligible: false,
    reason: 'window_expired',
  });
});

test('WITHDRAWAL: 무상만 쓴 건 미사용, 유상 순차감이 있으면 used, 반환으로 상쇄되면 미사용', () => {
  const p = purchase();
  const freeOnly = [...bought(p, T0), entry('CHAT_CHARGE', 0, -5000, T0 + 1)];
  assert.equal(quoteWithdrawal({ purchase: p, entries: freeOnly, refunds: [], now: T0 + DAY }).eligible, true);

  const paidUsed = [...bought(p, T0), entry('REVIEW_CHARGE', -640_000, -30_000, T0 + 1, 'PRRR1')];
  assert.deepEqual(quoteWithdrawal({ purchase: p, entries: paidUsed, refunds: [], now: T0 + DAY }), { eligible: false, reason: 'used' });

  const refunded = [...paidUsed, entry('REVIEW_REFUND', 640_000, 30_000, T0 + 2, 'PRRR1')];
  assert.equal(quoteWithdrawal({ purchase: p, entries: refunded, refunds: [], now: T0 + DAY }).eligible, true);
});

test('WITHDRAWAL: 구매 전 차감은 무관, 보너스 일부 소진이면 남은 무상만 회수, 부채 상태여도 가능', () => {
  const p = purchase();
  const entries = [entry('CHAT_CHARGE', -400_000, 0, T0 - DAY), ...bought(p, T0), entry('CHAT_CHARGE', 0, -20_000, T0 + 1)];
  const quote = quoteWithdrawal({ purchase: p, entries, refunds: [], now: T0 + DAY });
  assert.equal(quote.eligible, true);
  if (quote.eligible) assert.deepEqual(quote.delta, { paidDelta: -300_000, freeDelta: -10_000 });
});

test('WITHDRAWAL: 이미 환불된 구매·PAID 아님·원장 미도착', () => {
  const p = purchase();
  const refunds: RefundRow[] = [{ id: 'PRCR1', kind: 'WITHDRAWAL', purchaseId: 'PRCP1', state: 'DONE', cancels: [] }];
  assert.deepEqual(quoteWithdrawal({ purchase: p, entries: bought(p, T0), refunds, now: T0 + DAY }), {
    eligible: false,
    reason: 'already_refunded',
  });
  const remainderTouched: RefundRow[] = [
    {
      id: 'PRCR2',
      kind: 'REMAINDER',
      purchaseId: null,
      state: 'DONE',
      cancels: [{ purchaseId: 'PRCP1', paymentKey: 'PRCP1', amount: 5000, status: 'succeeded' }],
    },
  ];
  assert.deepEqual(quoteWithdrawal({ purchase: p, entries: bought(p, T0), refunds: remainderTouched, now: T0 + DAY }), {
    eligible: false,
    reason: 'already_refunded',
  });
  assert.deepEqual(quoteWithdrawal({ purchase: purchase({ state: 'PENDING', paidAt: null }), entries: [], refunds: [], now: T0 }), {
    eligible: false,
    reason: 'not_paid',
  });
  assert.deepEqual(quoteWithdrawal({ purchase: p, entries: [], refunds: [], now: T0 + DAY }), { eligible: false, reason: 'not_paid' });
});

test('REMAINDER: 가중평균 단가(보너스 제외)×잔여 유상×0.9 절사, 무상 전량 소멸, 최신 구매부터 배분', () => {
  const p1 = purchase({ id: 'PRCP1', paymentKey: 'PRCP1', price: 4900, credits: 100, bonusCredits: 0, paidAt: T0 });
  const p2 = purchase({ id: 'PRCP2', paymentKey: 'PRCP2', price: 14_900, credits: 300, bonusCredits: 30, paidAt: T0 + DAY });
  const entries = [...bought(p1, T0), ...bought(p2, T0 + DAY), entry('REVIEW_CHARGE', -100_000, -30_000, T0 + 2 * DAY, 'PRRR1')];
  const quote = quoteRemainder({ purchases: [p1, p2], entries, refunds: [] });

  const unit = (4900 + 14_900) / (100 + 300);
  const expected = Math.floor(300 * unit * REMAINDER_REFUND_RATE);
  assert.deepEqual(quote, {
    eligible: true,
    amount: expected,
    delta: { paidDelta: -300_000, freeDelta: 0 },
    cancels: [{ purchaseId: 'PRCP2', paymentKey: 'PRCP2', amount: expected, status: 'planned' }],
    shortfall: 0,
  });
});

test('REMAINDER: 유상 잔액 ≤ 0(부채·전부 소진)·구매 없음·금액 0은 no_paid_balance', () => {
  const p = purchase({ price: 4900, credits: 100, bonusCredits: 0 });
  const debt = [...bought(p, T0), entry('CHAT_CHARGE', -120_000, 0, T0 + 1)];
  assert.deepEqual(quoteRemainder({ purchases: [p], entries: debt, refunds: [] }), { eligible: false, reason: 'no_paid_balance' });
  assert.deepEqual(quoteRemainder({ purchases: [], entries: [entry('GRANT', 0, 100_000, T0)], refunds: [] }), {
    eligible: false,
    reason: 'no_paid_balance',
  });
  assert.deepEqual(quoteRemainder({ purchases: [], entries: [entry('ADJUSTMENT', 100_000, 0, T0)], refunds: [] }), {
    eligible: false,
    reason: 'no_paid_balance',
  });
  const tiny = [...bought(p, T0), entry('CHAT_CHARGE', -99_990, 0, T0 + 1)];
  assert.deepEqual(quoteRemainder({ purchases: [p], entries: tiny, refunds: [] }), { eligible: false, reason: 'no_paid_balance' });
});

test('REMAINDER: 철회된 구매는 단가·배분에서 제외, 기취소액은 배분에서 차감, 부족분은 shortfall', () => {
  const p1 = purchase({ id: 'PRCP1', paymentKey: 'PRCP1', price: 4900, credits: 100, bonusCredits: 0, paidAt: T0 });
  const p2 = purchase({ id: 'PRCP2', paymentKey: 'PRCP2', price: 4900, credits: 100, bonusCredits: 0, paidAt: T0 + DAY });
  const p3 = purchase({ id: 'PRCP3', paymentKey: 'PRCP3', price: 4900, credits: 100, bonusCredits: 0, paidAt: T0 + 2 * DAY });
  const refunds: RefundRow[] = [
    {
      id: 'PRCR1',
      kind: 'WITHDRAWAL',
      purchaseId: 'PRCP1',
      state: 'DONE',
      cancels: [{ purchaseId: 'PRCP1', paymentKey: 'PRCP1', amount: 4900, status: 'succeeded' }],
    },
    {
      id: 'PRCR2',
      kind: 'REMAINDER',
      purchaseId: null,
      state: 'DONE',
      cancels: [{ purchaseId: 'PRCP3', paymentKey: 'PRCP3', amount: 4000, status: 'succeeded' }],
    },
  ];
  const entries = [
    ...bought(p1, T0),
    ...bought(p2, T0 + DAY),
    ...bought(p3, T0 + 2 * DAY),
    entry('REFUND_OUT', -100_000, 0, T0 + 3 * DAY, 'PRCR1'),
  ];
  const quote = quoteRemainder({ purchases: [p1, p2, p3], entries, refunds });
  assert.equal(quote.eligible, true);
  if (!quote.eligible) return;
  assert.equal(quote.amount, Math.floor(200 * 49 * 0.9));
  assert.deepEqual(quote.cancels, [
    { purchaseId: 'PRCP3', paymentKey: 'PRCP3', amount: 900, status: 'planned' },
    { purchaseId: 'PRCP2', paymentKey: 'PRCP2', amount: 4900, status: 'planned' },
  ]);
  assert.equal(quote.shortfall, quote.amount - 5800);
});

test('REMAINDER: MANUAL로 종결된 취소분도 기취소액으로 차감된다', () => {
  const p1 = purchase({ id: 'PRCP1', paymentKey: 'PRCP1', price: 4900, credits: 100, bonusCredits: 0, paidAt: T0 });
  const p2 = purchase({ id: 'PRCP2', paymentKey: 'PRCP2', price: 4900, credits: 100, bonusCredits: 0, paidAt: T0 + DAY });
  const p3 = purchase({ id: 'PRCP3', paymentKey: 'PRCP3', price: 4900, credits: 100, bonusCredits: 0, paidAt: T0 + 2 * DAY });
  const refunds: RefundRow[] = [
    {
      id: 'PRCR1',
      kind: 'REMAINDER',
      purchaseId: null,
      state: 'DONE',
      cancels: [{ purchaseId: 'PRCP3', paymentKey: 'PRCP3', amount: 4000, status: 'manual' }],
    },
  ];
  const entries = [
    ...bought(p1, T0),
    ...bought(p2, T0 + DAY),
    ...bought(p3, T0 + 2 * DAY),
    entry('REFUND_OUT', -100_000, 0, T0 + 3 * DAY, 'PRCR1'),
  ];
  const quote = quoteRemainder({ purchases: [p1, p2, p3], entries, refunds });
  assert.equal(quote.eligible, true);
  if (!quote.eligible) return;
  assert.equal(quote.amount, Math.floor(200 * 49 * 0.9));
  assert.deepEqual(quote.cancels, [
    { purchaseId: 'PRCP3', paymentKey: 'PRCP3', amount: 900, status: 'planned' },
    { purchaseId: 'PRCP2', paymentKey: 'PRCP2', amount: 4900, status: 'planned' },
    { purchaseId: 'PRCP1', paymentKey: 'PRCP1', amount: 3020, status: 'planned' },
  ]);
  assert.equal(quote.shortfall, 0);
});

test('allocateCancels: 금액 0이면 빈 배분, 최신 우선, 가용액 초과 없음', () => {
  const p1 = purchase({ id: 'A', paymentKey: 'A', price: 1000, paidAt: T0 });
  const p2 = purchase({ id: 'B', paymentKey: 'B', price: 1000, paidAt: T0 + 1 });
  assert.deepEqual(allocateCancels([p1, p2], [], 0), []);
  assert.deepEqual(allocateCancels([p1, p2], [], 1500), [
    { purchaseId: 'B', paymentKey: 'B', amount: 1000, status: 'planned' },
    { purchaseId: 'A', paymentKey: 'A', amount: 500, status: 'planned' },
  ]);
});

test('classifyReconcile: paid+금액 일치 finalize / 불일치 mismatch / FAILED·CANCELLED fail / 그 외 defer', () => {
  assert.equal(classifyReconcile({ kind: 'paid', amount: 4900 }, 4900), 'finalize');
  assert.equal(classifyReconcile({ kind: 'paid', amount: 4000 }, 4900), 'mismatch');
  assert.equal(classifyReconcile({ kind: 'not-paid', paymentStatus: 'FAILED' }, 4900), 'fail');
  assert.equal(classifyReconcile({ kind: 'not-paid', paymentStatus: 'CANCELLED' }, 4900), 'fail');
  assert.equal(classifyReconcile({ kind: 'not-paid', paymentStatus: 'PARTIAL_CANCELLED' }, 4900), 'fail');
  assert.equal(classifyReconcile({ kind: 'not-paid', paymentStatus: 'READY' }, 4900), 'defer');
  assert.equal(classifyReconcile({ kind: 'error' }, 4900), 'defer');
});

test('classifyReconcile: not-found는 fail — PG에 결제 건이 없으면 도달하지 못한 결제로 확정한다', () => {
  assert.equal(classifyReconcile({ kind: 'not-found' }, 4900), 'fail');
});
