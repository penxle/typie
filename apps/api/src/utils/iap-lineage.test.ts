import '@typie/lib/dayjs';

import assert from 'node:assert/strict';
import test from 'node:test';
import { Status } from '@apple/app-store-server-library';
import { InAppPurchaseStore, SubscriptionState } from '@typie/lib/enums';
import { resolveEnrollTarget, selectApplePredecessor } from './iap-lineage.ts';
import type { ApplePredecessorCandidate } from './iap-lineage.ts';
import type { AppleStatusItem } from './iap-normalize.ts';

const binding = (id: string, store: InAppPurchaseStore, identifier: string) => ({ id, store, identifier });

test('등록 대상 해석: 요청 토큰과 같은 식별자의 행이 대상이다', () => {
  const rows = [binding('b1', InAppPurchaseStore.GOOGLE_PLAY, 'T1'), binding('b2', InAppPurchaseStore.GOOGLE_PLAY, 'T2')];
  assert.deepEqual(resolveEnrollTarget(rows, { store: InAppPurchaseStore.GOOGLE_PLAY, identifier: 'T2', lineageTokens: [] }), rows[1]);
});

test('등록 대상 해석: 계보 토큰은 선언 순서대로 행을 찾는다', () => {
  const rows = [binding('b1', InAppPurchaseStore.GOOGLE_PLAY, 'OLD-2'), binding('b2', InAppPurchaseStore.GOOGLE_PLAY, 'OLD-1')];
  assert.deepEqual(
    resolveEnrollTarget(rows, { store: InAppPurchaseStore.GOOGLE_PLAY, identifier: 'NEW', lineageTokens: ['OLD-1', 'OLD-2'] }),
    rows[1],
  );
});

test('등록 대상 해석: 다른 스토어의 행은 대상이 아니다', () => {
  const rows = [binding('b1', InAppPurchaseStore.APP_STORE, 'OLD')];
  assert.equal(resolveEnrollTarget(rows, { store: InAppPurchaseStore.GOOGLE_PLAY, identifier: 'NEW', lineageTokens: ['OLD'] }), null);
});

test('등록 대상 해석: 일치가 없으면 새 계보다', () => {
  assert.equal(
    resolveEnrollTarget([binding('b1', InAppPurchaseStore.GOOGLE_PLAY, 'X')], {
      store: InAppPurchaseStore.GOOGLE_PLAY,
      identifier: 'NEW',
      lineageTokens: [],
    }),
    null,
  );
});

const item = (
  originalTransactionId: string,
  over: { status?: number; appTransactionId?: string; group?: string } = {},
): AppleStatusItem => ({
  status: over.status ?? Status.ACTIVE,
  outerOriginalTransactionId: originalTransactionId,
  transaction: { originalTransactionId, transactionId: `TX-${originalTransactionId}`, appTransactionId: over.appTransactionId ?? 'APP-1' },
  renewalInfo: null,
  subscriptionGroupIdentifier: over.group ?? 'G1',
});

const candidate = (over: Partial<ApplePredecessorCandidate> = {}): ApplePredecessorCandidate => ({
  id: 'b1',
  userId: 'U1',
  identifier: 'OT-OLD',
  canonicalState: SubscriptionState.EXPIRED,
  ...over,
});

const select = (candidates: ApplePredecessorCandidate[], items: AppleStatusItem[], ownerUserId: string | null = null) =>
  selectApplePredecessor({ candidates, items, notifiedOriginalTransactionId: 'OT-NEW', ownerUserId });

test('애플 predecessor 선택: 같은 앱·그룹의 종료된 계보 하나면 선택한다', () => {
  const result = select([candidate()], [item('OT-NEW'), item('OT-OLD', { status: Status.EXPIRED })]);
  assert.deepEqual(result, { kind: 'selected', candidate: candidate() });
});

test('애플 predecessor 선택: 로컬 EXPIRED 는 스토어 항목이 살아 있어도 종료로 본다', () => {
  const result = select([candidate()], [item('OT-NEW'), item('OT-OLD')]);
  assert.deepEqual(result, { kind: 'selected', candidate: candidate() });
});

test('애플 predecessor 선택: 계보 밖 행은 후보가 아니다', () => {
  assert.deepEqual(select([candidate()], [item('OT-NEW'), item('OT-OLD', { group: 'G2' })]), { kind: 'none' });
  assert.deepEqual(select([candidate()], [item('OT-NEW'), item('OT-OLD', { appTransactionId: 'APP-2' })]), { kind: 'none' });
  assert.deepEqual(select([candidate()], [item('OT-NEW')]), { kind: 'none' });
});

test('애플 predecessor 선택: 살아 있는 계보가 섞이면 보류한다', () => {
  const live = candidate({ canonicalState: SubscriptionState.ACTIVE });
  assert.deepEqual(select([live], [item('OT-NEW'), item('OT-OLD')]), { kind: 'live', count: 1 });
});

test('애플 predecessor 선택: 종료된 계보가 둘 이상이면 모호하다', () => {
  const rows = [candidate(), candidate({ id: 'b2', identifier: 'OT-OLD2' })];
  const result = select(rows, [item('OT-NEW'), item('OT-OLD', { status: Status.EXPIRED }), item('OT-OLD2', { status: Status.EXPIRED })]);
  assert.deepEqual(result, { kind: 'ambiguous', count: 2 });
});

test('애플 predecessor 선택: 소유자가 둘 이상이면 모호하다', () => {
  const rows = [candidate(), candidate({ id: 'b2', userId: 'U2', identifier: 'OT-OLD2' })];
  assert.deepEqual(
    select(rows, [item('OT-NEW'), item('OT-OLD', { status: Status.EXPIRED }), item('OT-OLD2', { status: Status.EXPIRED })]),
    {
      kind: 'ambiguous',
      count: 2,
    },
  );
});

test('애플 predecessor 선택: 소유 증거와 후보 소유자가 다르면 foreign', () => {
  assert.deepEqual(select([candidate({ userId: 'U2' })], [item('OT-NEW'), item('OT-OLD', { status: Status.EXPIRED })], 'U1'), {
    kind: 'foreign',
    userIds: ['U2'],
  });
});
