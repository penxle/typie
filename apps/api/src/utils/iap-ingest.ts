import { InAppPurchaseStore } from '@typie/lib/enums';
import { eq, inArray, sql } from 'drizzle-orm';
import * as uuid from 'uuid';
import { db, first, InAppPurchaseRecords, UserInAppPurchases, Users } from '#/db/index.ts';
import { production } from '#/env.ts';
import * as appstore from '#/external/appstore.ts';
import * as googleplay from '#/external/googleplay.ts';
import { deriveGoogleOrderChain, mapAppleTransaction, mapGoogleOrder } from './iap-ingest-core.ts';
import { opsAlertOnce } from './ops-alert.ts';
import type { IapPaymentRecordDraft } from './iap-ingest-core.ts';

export type CollectedIapPayment = IapPaymentRecordDraft & { userId: string };

type IngestBinding = { id: string; userId: string; store: InAppPurchaseStore; identifier: string };

type IapPaymentCollection =
  { kind: 'ok'; records: CollectedIapPayment[] } | { kind: 'skipped'; reason: 'token-gone' | 'token-not-found' | 'no-order-id' };

export type IapIngestOutcome =
  { kind: 'ingested'; records: number } | { kind: 'skipped'; reason: 'binding-missing' | 'token-gone' | 'token-not-found' | 'no-order-id' };

const collectApple = async (binding: IngestBinding): Promise<IapPaymentCollection> => {
  const history = await appstore.getTransactionHistory(binding.identifier);
  if (history.kind === 'error') {
    throw new Error(`apple transaction history fetch failed: ${binding.id}`);
  }

  const mapped: { record: IapPaymentRecordDraft; appAccountToken: string | null }[] = [];

  for (const transaction of history.transactions) {
    const mapping = mapAppleTransaction(transaction, { allowSandbox: !production });

    if (mapping.kind === 'invalid') {
      await opsAlertOnce('iap-ingest-invalid-payload', `${binding.id}:${transaction.transactionId ?? 'unknown'}`, {
        source: 'utils/iap-ingest#collectApple',
        bindingId: binding.id,
        transactionId: transaction.transactionId,
        reason: mapping.reason,
      });
      continue;
    }

    if (mapping.kind === 'record') {
      mapped.push(mapping);
    }
  }

  // appAccountToken 은 구매 시점에 앱이 심은 users.uuid 다 — 있으면 트랜잭션 단위로 정밀 귀속하고,
  // 없으면(토큰 배선 이전 구매·앱 밖 재가입) 바인딩 소유자로 폴백한다.
  const tokens = [
    ...new Set(
      mapped
        .map((entry) => entry.appAccountToken)
        .filter((token): token is string => !!token && uuid.validate(token))
        .map((token) => token.toLowerCase()),
    ),
  ];
  const owners =
    tokens.length === 0 ? [] : await db.select({ id: Users.id, uuid: Users.uuid }).from(Users).where(inArray(Users.uuid, tokens));
  const ownerByToken = new Map(owners.map((owner) => [owner.uuid, owner.id]));

  return {
    kind: 'ok',
    records: mapped.map((entry) => ({
      ...entry.record,
      userId: (entry.appAccountToken && ownerByToken.get(entry.appAccountToken.toLowerCase())) || binding.userId,
    })),
  };
};

const collectGoogle = async (binding: IngestBinding): Promise<IapPaymentCollection> => {
  const subscription = await googleplay.getSubscriptionV2(binding.identifier);

  if (subscription.kind === 'gone') {
    return { kind: 'skipped', reason: 'token-gone' };
  }
  if (subscription.kind === 'not-found') {
    return { kind: 'skipped', reason: 'token-not-found' };
  }
  if (subscription.kind === 'error') {
    throw new Error(`google subscription fetch failed: ${binding.id}`);
  }

  const orderIds = [
    ...new Set(
      (subscription.purchase.lineItems ?? [])
        .map((lineItem) => lineItem.latestSuccessfulOrderId)
        .filter((orderId): orderId is string => !!orderId)
        .flatMap(deriveGoogleOrderChain),
    ),
  ];

  if (orderIds.length === 0) {
    return { kind: 'skipped', reason: 'no-order-id' };
  }

  const records: CollectedIapPayment[] = [];
  let notFound = 0;

  for (const orderId of orderIds) {
    const result = await googleplay.getOrder(orderId);

    // 접미사 건너뜀(결제 거절 회차 등) 방어 — 존재하지 않는 회차는 조용히 넘어간다.
    if (result.kind === 'not-found') {
      notFound += 1;
      continue;
    }
    if (result.kind === 'error') {
      throw new Error(`google order fetch failed: ${binding.id} ${orderId}`);
    }

    const mapping = mapGoogleOrder(result.order);

    if (mapping.kind === 'unknown-state') {
      await opsAlertOnce('iap-ingest-unknown-order-state', orderId, {
        source: 'utils/iap-ingest#collectGoogle',
        bindingId: binding.id,
        orderId,
        state: mapping.state,
      });
      continue;
    }

    if (mapping.kind === 'invalid') {
      await opsAlertOnce('iap-ingest-invalid-payload', orderId, {
        source: 'utils/iap-ingest#collectGoogle',
        bindingId: binding.id,
        orderId,
        reason: mapping.reason,
      });
      continue;
    }

    if (mapping.kind === 'record') {
      records.push({ ...mapping.record, userId: binding.userId });
    }
  }

  // 전 주문 404는 정상 스킵(PENDING·CANCELED 전건)과 달리 설정 오류(패키지명 등)의 유일한 무음 신호다 —
  // 구독 조회는 성공하므로 바인딩 단위 관측(token-not-found)에도 잡히지 않는다.
  if (notFound === orderIds.length) {
    await opsAlertOnce('iap-ingest-orders-all-not-found', binding.id, {
      source: 'utils/iap-ingest#collectGoogle',
      bindingId: binding.id,
      orderIds: orderIds.length,
    });
  }

  return { kind: 'ok', records };
};

export const collectIapPaymentRecords = async (binding: IngestBinding): Promise<IapPaymentCollection> => {
  return binding.store === InAppPurchaseStore.APP_STORE ? await collectApple(binding) : await collectGoogle(binding);
};

export const ingestIapPayments = async ({ bindingId }: { bindingId: string }): Promise<IapIngestOutcome> => {
  const binding = await db
    .select({
      id: UserInAppPurchases.id,
      userId: UserInAppPurchases.userId,
      store: UserInAppPurchases.store,
      identifier: UserInAppPurchases.identifier,
    })
    .from(UserInAppPurchases)
    .where(eq(UserInAppPurchases.id, bindingId))
    .then(first);

  if (!binding) {
    return { kind: 'skipped', reason: 'binding-missing' };
  }

  const collection = await collectIapPaymentRecords(binding);
  if (collection.kind === 'skipped') {
    return collection;
  }

  // userId·purchasedAt·createdAt 은 갱신하지 않는다 — 귀속은 최초 관측을 유지한다(계정 이전 후 재수집이 과거
  // 결제의 귀속을 바꾸지 않는다). 행 단위 멱등 upsert 라 트랜잭션 없이도 부분 진행이 안전하다(재실행이 수렴).
  for (const record of collection.records) {
    await db
      .insert(InAppPurchaseRecords)
      .values({
        store: binding.store,
        identifier: record.identifier,
        userId: record.userId,
        productId: record.productId,
        state: record.state,
        amount: record.amount,
        currency: record.currency,
        refundedAmount: record.refundedAmount,
        purchasedAt: record.purchasedAt,
        refundedAt: record.refundedAt,
        data: record.data,
      })
      .onConflictDoUpdate({
        target: [InAppPurchaseRecords.store, InAppPurchaseRecords.identifier],
        set: {
          productId: record.productId,
          state: record.state,
          amount: record.amount,
          currency: record.currency,
          refundedAmount: record.refundedAmount,
          refundedAt: record.refundedAt,
          data: record.data,
          updatedAt: sql`now()`,
        },
      });
  }

  return { kind: 'ingested', records: collection.records.length };
};
