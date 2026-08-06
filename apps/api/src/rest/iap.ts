import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { InAppPurchaseStore, PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, eq, inArray, ne } from 'drizzle-orm';
import { Hono } from 'hono';
import * as uuid from 'uuid';
import { redis } from '#/cache.ts';
import { db, first, Plans, Subscriptions, UserInAppPurchases, Users } from '#/db/index.ts';
import { production } from '#/env.ts';
import * as appstore from '#/external/appstore.ts';
import * as googleplay from '#/external/googleplay.ts';
import * as slack from '#/external/slack.ts';
import { enqueueJob } from '#/mq/index.ts';
import { discoverAppleSuccessor, mapUnsupportedStorePayloadReason, normalizeGoogle, selectAppleStatusItem } from '#/utils/iap-normalize.ts';
import { applyNormalizedIapLocked, resolveAcknowledgeDuty } from '#/utils/iap-sync.ts';
import { opsAlert, opsAlertOnce } from '#/utils/ops-alert.ts';
import { lockUserSubscriptionState } from '#/utils/subscription-lock.ts';
import type { ResponseBodyV2 } from '@apple/app-store-server-library';
import type { androidpublisher_v3 } from '@googleapis/androidpublisher';
import type { PlanInterval } from '@typie/lib/enums';
import type { Env } from '#/context.ts';
import type { Transaction } from '#/db/index.ts';
import type { DeveloperNotification } from '#/external/googleplay.ts';
import type { IapPriorPeriod } from '#/utils/iap-normalize.ts';
import type { IapAcknowledgeDuty } from '#/utils/iap-sync.ts';
import type { OpsAlertId } from '#/utils/ops-alert.ts';

export const iap = new Hono<Env>();

const log = logger.getChild('iap');

const PENDING_REFUND_REVIEW_WINDOW_MS = 86_400_000;
// 미바인딩·미등록은 이 창 안에서는 정상 국면이다 — 스토어 결제 직후 등록 mutation 이 아직 바인딩을 만들지 않았거나,
// 연속 변경 알림이 역순으로 도착한 구간이다. 재전송 백오프가 이 구간을 자연 해소하므로 알람은 그 뒤에 낸다.
const ENROLLMENT_RACE_WINDOW_MS = 3_600_000;
const OPS_ALERT_DEDUPE_TTL_SECONDS = 86_400;
const VOIDED_PRODUCT_TYPE_SUBSCRIPTION = 1;

type SuccessionBinding = { id: string; userId: string; identifier: string };

const logNotification = async (payload: Record<string, unknown>) => {
  await slack.sendMessage({
    channel: '#alert',
    username: '운영 알림',
    iconEmoji: ':rotating_light:',
    message: `\`\`\`\n${JSON.stringify(payload, null, 2)}\n\`\`\``,
  });
};

// 재전송이 같은 사실을 반복 보고하면 실제 실패가 그 안에 묻힌다 — 토큰·원거래 ID 당 하루 1회로 접는다.
// 레디스 실패는 알람 발화 쪽으로 폴백한다(디듀프 저장소 장애가 관측을 없애서는 안 된다).
const alertOnce = async (id: OpsAlertId, key: string, context: Record<string, unknown>) => {
  let acquired: boolean;

  try {
    acquired = (await redis.set(`ops-alert-dedupe:${id}:${key}`, '1', 'EX', OPS_ALERT_DEDUPE_TTL_SECONDS, 'NX')) === 'OK';
  } catch {
    acquired = true;
  }

  if (acquired) {
    await opsAlert(id, context);
  }
};

// 관측 시각을 못 재면(필드 부재·비유한) 완충 구간으로 간주하지 않는다 — 판정 불가는 알람 쪽으로 기운다.
const isBeyondEnrollmentRace = (observedAt: number | null): boolean =>
  observedAt === null || !Number.isFinite(observedAt) || dayjs().diff(dayjs(observedAt)) > ENROLLMENT_RACE_WINDOW_MS;

// 해당 토큰의 계약이 끝났다는 통지 — REVOKED(12) · EXPIRED(13) · PENDING_PURCHASE_CANCELED(20).
// 이후 등록이 와도 되살아날 구독이 없으므로 바인딩 없는 상태로 재시도를 이어갈 이유가 없다.
const TERMINAL_SUBSCRIPTION_NOTIFICATION_TYPES = new Set([12, 13, 20]);

// 커밋 후 의무다 — 롤백된 트랜잭션의 토큰을 승인하지 않는다.
// Apple 정규화는 productId 를 싣지 않아(iap-normalize 의 normalizeApple) 이 경로의 duty 는 항상 null 이다 — 승인은 Google 전용 의무다.
const settleAcknowledge = async (duty: IapAcknowledgeDuty | null) => {
  if (!duty) {
    return;
  }

  try {
    await googleplay.acknowledgeSubscription(duty);
  } catch (err) {
    await opsAlert('google-acknowledge-failed', { ...duty, error: err instanceof Error ? err.message : String(err) });
  }
};

const findBinding = async (store: InAppPurchaseStore, identifier: string) => {
  return await db
    .select({ id: UserInAppPurchases.id, userId: UserInAppPurchases.userId, identifier: UserInAppPurchases.identifier })
    .from(UserInAppPurchases)
    .where(and(eq(UserInAppPurchases.store, store), eq(UserInAppPurchases.identifier, identifier)))
    .then(first);
};

// 승격 전 충돌 검사는 primitive 를 직접 부르는 경로가 스스로 해야 한다 — syncIapBinding 을 우회하면 검사도 함께
// 우회된다. 유니크 위반은 경합의 최후 방어일 뿐이고, WILL_ACTIVATE 는 부분 유니크가 분리되어 DB 가 잡지도 못한다.
const findPromotionConflict = async (tx: Transaction, { userId, subscriptionId }: { userId: string; subscriptionId: string }) => {
  return await tx
    .select({ id: Subscriptions.id })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(
      and(
        eq(Subscriptions.userId, userId),
        ne(Subscriptions.id, subscriptionId),
        ne(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
        inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.WILL_ACTIVATE]),
      ),
    )
    .then(first);
};

type LockedSuccession =
  | {
      kind: 'ok';
      binding: { id: string; userId: string; store: InAppPurchaseStore; identifier: string; subscriptionId: string };
      prior: IapPriorPeriod;
      canonicalId: string;
    }
  | { kind: 'changed' }
  | { kind: 'invariant'; reason: string; bindingId: string };

// 락 전 캡처값과의 동일성 재검증 — 대기 중 등록·이전이 바인딩을 옮겼으면 우리가 쥔 것은 이전 유저의 락이다.
const loadLockedSuccession = async (tx: Transaction, captured: SuccessionBinding): Promise<LockedSuccession> => {
  const binding = await tx
    .select({
      id: UserInAppPurchases.id,
      userId: UserInAppPurchases.userId,
      store: UserInAppPurchases.store,
      identifier: UserInAppPurchases.identifier,
      subscriptionId: UserInAppPurchases.subscriptionId,
    })
    .from(UserInAppPurchases)
    .where(eq(UserInAppPurchases.id, captured.id))
    .for('no key update')
    .then(first);

  if (!binding || binding.userId !== captured.userId || binding.identifier !== captured.identifier) {
    return { kind: 'changed' };
  }

  if (!binding.subscriptionId) {
    return { kind: 'invariant', reason: 'iap binding without canonical subscription', bindingId: binding.id };
  }

  const canonical = await tx
    .select({
      id: Subscriptions.id,
      state: Subscriptions.state,
      currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
      currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
    })
    .from(Subscriptions)
    .where(eq(Subscriptions.id, binding.subscriptionId))
    .for('no key update')
    .then(first);

  if (!canonical) {
    return { kind: 'invariant', reason: 'iap binding canonical subscription missing', bindingId: binding.id };
  }

  return {
    kind: 'ok',
    binding: { ...binding, subscriptionId: binding.subscriptionId },
    prior: {
      state: canonical.state,
      currentPeriodStartsAt: canonical.currentPeriodStartsAt,
      currentPeriodEndsAt: canonical.currentPeriodEndsAt,
    },
    canonicalId: canonical.id,
  };
};

type SuccessionOutcome =
  | { kind: 'applied'; acknowledge: IapAcknowledgeDuty | null; bindingId: string }
  | { kind: 'bound'; bindingId: string }
  | { kind: 'changed' }
  | {
      kind: 'conflict';
      userId: string;
      subscriptionId: string;
      conflictingSubscriptionId: string;
      acknowledge: IapAcknowledgeDuty | null;
    }
  | { kind: 'invariant'; reason: string; bindingId: string };

// enrollmentRace = "아직 아무것도 없다"(매칭 유저 0건·후보 0건) 계열. 구조적 미해결(복수·불일치)과 달리
// 등록 경합 완충 구간에서는 정상 국면이라 알람을 신선도로 거른다.
type AppleUnresolved = { kind: 'unresolved'; reason: string; candidates: number; enrollmentRace: boolean };

type AppleSuccessionOutcome = SuccessionOutcome | AppleUnresolved | { kind: 'lookup-failed' };

type AppleCandidate = AppleUnresolved | { kind: 'found'; binding: SuccessionBinding } | { kind: 'lookup-failed' };

// appAccountToken 은 구매 시점에 스토어가 서명해 박은 소유 증거다 — users.uuid 로 소유자를 직접 찾는다.
// 유저는 찾았는데 APP_STORE 바인딩이 없으면 등록이 아직 안 온 것이다(경합).
const findAppleBindingByAccountToken = async (appAccountToken: string): Promise<AppleCandidate> => {
  if (!uuid.validate(appAccountToken)) {
    return { kind: 'unresolved', reason: 'apple-account-token-unmatched', candidates: 0, enrollmentRace: false };
  }

  const matched = await db
    .select({ id: UserInAppPurchases.id, userId: UserInAppPurchases.userId, identifier: UserInAppPurchases.identifier })
    .from(Users)
    .innerJoin(UserInAppPurchases, and(eq(UserInAppPurchases.userId, Users.id), eq(UserInAppPurchases.store, InAppPurchaseStore.APP_STORE)))
    .where(eq(Users.uuid, appAccountToken));

  if (matched.length !== 1) {
    return {
      kind: 'unresolved',
      reason: matched.length === 0 ? 'apple-account-token-unmatched' : 'apple-account-token-ambiguous',
      candidates: matched.length,
      enrollmentRace: matched.length === 0,
    };
  }

  return { kind: 'found', binding: matched[0] };
};

// 토큰 없는 앱 밖 구매의 정상 경로다 — 새 ID 조회는 같은 고객의 전 구독을 반환하므로 응답 안의 다른 원거래 ID로
// 기존 바인딩을 역발견한다.
const findAppleBindingByStatuses = async (originalTransactionId: string): Promise<AppleCandidate> => {
  const statuses = await appstore.getSubscriptionStatuses(originalTransactionId);
  if (statuses.kind === 'error') {
    return { kind: 'lookup-failed' };
  }

  const identifiers = [
    ...new Set(
      statuses.items
        .map((item) => item.transaction?.originalTransactionId)
        .filter((identifier): identifier is string => !!identifier && identifier !== originalTransactionId),
    ),
  ];

  if (identifiers.length === 0) {
    return { kind: 'unresolved', reason: 'apple-reverse-discovery-empty', candidates: 0, enrollmentRace: true };
  }

  const bindings = await db
    .select({ id: UserInAppPurchases.id, userId: UserInAppPurchases.userId, identifier: UserInAppPurchases.identifier })
    .from(UserInAppPurchases)
    .where(and(eq(UserInAppPurchases.store, InAppPurchaseStore.APP_STORE), inArray(UserInAppPurchases.identifier, identifiers)));

  if (bindings.length !== 1) {
    return {
      kind: 'unresolved',
      reason: bindings.length === 0 ? 'apple-reverse-discovery-unbound' : 'apple-reverse-discovery-ambiguous',
      candidates: bindings.length,
      enrollmentRace: bindings.length === 0,
    };
  }

  return { kind: 'found', binding: bindings[0] };
};

const adoptUnboundAppleNotification = async ({
  originalTransactionId,
  appAccountToken,
}: {
  originalTransactionId: string;
  appAccountToken: string | null;
}): Promise<AppleSuccessionOutcome> => {
  const candidate = appAccountToken
    ? await findAppleBindingByAccountToken(appAccountToken)
    : await findAppleBindingByStatuses(originalTransactionId);

  if (candidate.kind !== 'found') {
    return candidate;
  }

  const captured = candidate.binding;

  return await db.transaction(async (tx): Promise<AppleSuccessionOutcome> => {
    await lockUserSubscriptionState(tx, captured.userId);

    // 락 대기 중 등록 경로가 이 원거래 ID 를 선점했으면 (store, identifier) 유니크가 교체를 막는다 — 점유자에게 넘긴다.
    const occupant = await tx
      .select({ id: UserInAppPurchases.id })
      .from(UserInAppPurchases)
      .where(and(eq(UserInAppPurchases.store, InAppPurchaseStore.APP_STORE), eq(UserInAppPurchases.identifier, originalTransactionId)))
      .then(first);

    if (occupant) {
      return { kind: 'bound', bindingId: occupant.id };
    }

    const locked = await loadLockedSuccession(tx, captured);
    if (locked.kind !== 'ok') {
      return locked;
    }

    const now = dayjs();

    // 확정 조회는 유저 락 안이다 — 라이브 응답이 곧 최신이라 stale 판별 규칙이 사라진다.
    const statuses = await appstore.getSubscriptionStatuses(locked.binding.identifier);
    if (statuses.kind === 'error') {
      return { kind: 'lookup-failed' };
    }

    const selection = selectAppleStatusItem(statuses.items, locked.binding.identifier);
    if (selection.kind === 'unknown') {
      return { kind: 'unresolved', reason: selection.reason, candidates: 0, enrollmentRace: false };
    }

    const successor = discoverAppleSuccessor({
      items: statuses.items,
      selected: selection.item,
      requestedOriginalTransactionId: locked.binding.identifier,
      prior: locked.prior,
      now,
    });

    if (successor.kind !== 'succeeded') {
      return {
        kind: 'unresolved',
        reason: `apple-successor-${successor.kind}`,
        candidates: successor.kind === 'unresolved' ? successor.candidates : 0,
        enrollmentRace: successor.kind === 'none',
      };
    }

    // 알림이 알린 새 ID 가 아닌 후계는 이 경로의 대상이 아니다 — 그 계약은 자기 알림·일일 재조정이 다룬다.
    if (successor.originalTransactionId !== originalTransactionId) {
      return { kind: 'unresolved', reason: 'apple-successor-identifier-mismatch', candidates: 1, enrollmentRace: false };
    }

    if (successor.normalized.kind === 'tracked' && successor.normalized.state === SubscriptionState.ACTIVE) {
      const conflict = await findPromotionConflict(tx, { userId: locked.binding.userId, subscriptionId: locked.canonicalId });
      if (conflict) {
        // 전이는 막아도 승인 의무는 남는다 — 승격을 스킵했다고 3일 자동 환불을 방치하면 돈의 불변식이 깨진다.
        return {
          kind: 'conflict',
          userId: locked.binding.userId,
          subscriptionId: locked.canonicalId,
          conflictingSubscriptionId: conflict.id,
          acknowledge: resolveAcknowledgeDuty(successor.normalized, originalTransactionId),
        };
      }
    }

    const { acknowledge } = await applyNormalizedIapLocked(tx, {
      binding: locked.binding,
      normalized: successor.normalized,
      newIdentifier: originalTransactionId,
    });

    return { kind: 'applied', acknowledge, bindingId: locked.binding.id };
  });
};

iap.post('/appstore', async (c) => {
  const body = await c.req.json<ResponseBodyV2>();
  if (!body.signedPayload) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  const notification = await appstore.decodeNotification(body.signedPayload).catch((err: unknown) => {
    // 서명 검증 실패는 재전송해도 통과하지 않는다 — 재시도를 유도하지 않고 원인은 사람이 본다.
    Sentry.captureException(err);
    return null;
  });

  if (!notification) {
    return c.json({ error: 'invalid_signature' }, 401);
  }

  // 유효한 동의 없이 소비 정보를 회신하지 않는다 — 회신 재개는 동의 모델 도입 이후다.
  if (notification.notificationType === 'CONSUMPTION_REQUEST') {
    return c.json({}, 200);
  }

  const originalTransactionId = notification.data.transaction?.originalTransactionId;
  if (!originalTransactionId) {
    await logNotification({ source: 'rest/appstore', reason: 'no_transaction', notification });
    return c.json({}, 200);
  }

  // 알림 타입은 보지 않는다 — 어떤 알림이든 바인딩을 찾아 락 안 조회로 확정한다.
  const binding = await findBinding(InAppPurchaseStore.APP_STORE, originalTransactionId);
  if (binding) {
    await enqueueJob('iap:sync', { bindingId: binding.id });
    return c.json({}, 200);
  }

  const outcome = await adoptUnboundAppleNotification({
    originalTransactionId,
    appAccountToken: notification.data.transaction?.appAccountToken ?? null,
  });

  if (outcome.kind === 'lookup-failed') {
    return c.json({ error: 'retry' }, 500);
  }

  // 락 안에서 귀속이 달라졌으면 등록·탈퇴가 먼저 처리한 것이다 — 전이하지 않는다(일일 재조정이 같은 발견 단계를 수행한다).
  if (outcome.kind === 'changed') {
    return c.json({}, 200);
  }

  // canonical 부재는 사람이 고칠 불변식 위반이다 — 200 으로 삼키면 수리 후 이 알림을 다시 태울 트리거가 없어
  // 유저가 앱을 열 때까지 잠긴다. 재전송 창이 수리 시간을 감당한다(syncIapBinding 의 deferred 와 같은 취급).
  if (outcome.kind === 'invariant') {
    await alertOnce('invariant-violation', originalTransactionId, {
      source: 'rest/appstore',
      reason: outcome.reason,
      bindingId: outcome.bindingId,
      identifier: originalTransactionId,
    });

    return c.json({ error: 'retry' }, 500);
  }

  if (outcome.kind === 'applied') {
    await settleAcknowledge(outcome.acknowledge);
    await enqueueJob('iap:ingest', { bindingId: outcome.bindingId });
  } else if (outcome.kind === 'bound') {
    await enqueueJob('iap:sync', { bindingId: outcome.bindingId });
  } else if (outcome.kind === 'conflict') {
    await opsAlert('iap-promotion-conflict-skipped', {
      source: 'rest/appstore',
      identifier: originalTransactionId,
      userId: outcome.userId,
      subscriptionId: outcome.subscriptionId,
      conflictingSubscriptionId: outcome.conflictingSubscriptionId,
    });
    await settleAcknowledge(outcome.acknowledge);
  } else // 결제 직후의 0건은 등록 mutation 이 아직 바인딩을 만들지 않은 완충 구간이다 — 구조적 미해결만 즉시 알람한다.
  if (
    outcome.kind === 'unresolved' &&
    (!outcome.enrollmentRace || isBeyondEnrollmentRace(notification.data.transaction?.purchaseDate ?? null))
  ) {
    await alertOnce('apple-unbound-discovery-unresolved', originalTransactionId, {
      identifier: originalTransactionId,
      reason: outcome.reason,
      candidates: outcome.candidates,
    });
  }

  return c.json({}, 200);
});

type GoogleSuccessionOutcome =
  | SuccessionOutcome
  | { kind: 'foreign'; bindingId: string; userId: string; obfuscatedAccountId: string }
  | { kind: 'not-adopted'; reason: string; bindingId: string };

const adoptUnboundGoogleNotification = async ({
  purchaseToken,
  purchase,
  captured,
}: {
  purchaseToken: string;
  purchase: androidpublisher_v3.Schema$SubscriptionPurchaseV2;
  captured: SuccessionBinding;
}): Promise<GoogleSuccessionOutcome> => {
  return await db.transaction(async (tx): Promise<GoogleSuccessionOutcome> => {
    await lockUserSubscriptionState(tx, captured.userId);

    // 락 대기 중 등록 경로가 이 토큰을 선점했으면 (store, identifier) 유니크가 교체를 막는다 — 점유자에게 넘긴다.
    const occupant = await tx
      .select({ id: UserInAppPurchases.id })
      .from(UserInAppPurchases)
      .where(and(eq(UserInAppPurchases.store, InAppPurchaseStore.GOOGLE_PLAY), eq(UserInAppPurchases.identifier, purchaseToken)))
      .then(first);

    if (occupant) {
      return { kind: 'bound', bindingId: occupant.id };
    }

    const locked = await loadLockedSuccession(tx, captured);
    if (locked.kind !== 'ok') {
      return locked;
    }

    // 세션 없는 경로는 승계까지만 한다 — 스토어가 알려준 소유자가 predecessor 바인딩의 유저와 다르면 이전이고,
    // 회수·이전은 소유 증거가 있는 등록 경로의 몫이다.
    const obfuscatedAccountId =
      purchase.externalAccountIdentifiers?.obfuscatedExternalAccountId ??
      purchase.outOfAppPurchaseContext?.expiredExternalAccountIdentifiers?.obfuscatedExternalAccountId;

    if (obfuscatedAccountId) {
      const owner = await tx.select({ uuid: Users.uuid }).from(Users).where(eq(Users.id, locked.binding.userId)).then(first);

      if (owner?.uuid !== obfuscatedAccountId) {
        return { kind: 'foreign', bindingId: locked.binding.id, userId: locked.binding.userId, obfuscatedAccountId };
      }
    }

    const plans = await tx
      .select({ id: Plans.id, interval: Plans.interval })
      .from(Plans)
      .where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE));
    const planIntervals: Record<string, PlanInterval> = Object.fromEntries(plans.map((plan) => [plan.id, plan.interval]));

    const normalized = normalizeGoogle({ purchase, prior: locked.prior, planIntervals, now: dayjs() });

    // unknown 이 곧바로 아래에서 'not-adopted' 로 접히며 reason 을 잃는다 — 접히기 전, 사유가 실제로 관측·폐기되는
    // 이 지점에서 알람 배선을 한다(등록·재조정과 같은 사유 집합을 공유하는 매핑 함수).
    if (normalized.kind === 'unknown') {
      const alertId = mapUnsupportedStorePayloadReason(normalized.reason);
      if (alertId) {
        await opsAlertOnce(alertId, purchaseToken, {
          source: 'rest/iap#adoptUnboundGoogleNotification',
          bindingId: locked.binding.id,
          identifier: purchaseToken,
          reason: normalized.reason,
        });
      }
    }

    // 성공·추적 대상이 아닌 새 토큰은 canonical 을 탈취하지 못한다 — 교체 없이 기존 토큰을 유지하고 그 토큰을 재조회한다.
    if (normalized.kind === 'expired' || normalized.kind === 'defer' || normalized.kind === 'untracked' || normalized.kind === 'unknown') {
      return { kind: 'not-adopted', reason: normalized.kind, bindingId: locked.binding.id };
    }

    if (normalized.state === SubscriptionState.ACTIVE) {
      const conflict = await findPromotionConflict(tx, { userId: locked.binding.userId, subscriptionId: locked.canonicalId });
      if (conflict) {
        // 전이는 막아도 승인 의무는 남는다 — 토큰을 교체하지 않았어도 스토어는 이 토큰을 이미 확인했으므로
        // 승격을 스킵했다고 3일 자동 환불을 방치하면 돈의 불변식이 깨진다.
        return {
          kind: 'conflict',
          userId: locked.binding.userId,
          subscriptionId: locked.canonicalId,
          conflictingSubscriptionId: conflict.id,
          acknowledge: resolveAcknowledgeDuty(normalized, purchaseToken),
        };
      }
    }

    const { acknowledge } = await applyNormalizedIapLocked(tx, {
      binding: locked.binding,
      normalized,
      newIdentifier: purchaseToken,
    });

    return { kind: 'applied', acknowledge, bindingId: locked.binding.id };
  });
};

iap.post('/googleplay', async (c) => {
  const notification = await c.req.json<DeveloperNotification>();

  if (notification.testNotification || notification.oneTimeProductNotification) {
    await logNotification({ source: 'rest/googleplay', notification });
    return c.json({}, 200);
  }

  if (notification.pendingRefundReviewNotification) {
    // 24시간 기한의 chargeback 검토 요청 — 자동 판정 없이 사람이 콘솔에서 ReviewRefund 를 수행한다.
    const eventTime = Number(notification.eventTimeMillis);
    await opsAlert('google-pending-refund-review', {
      payload: notification.pendingRefundReviewNotification,
      eventTimeMillis: notification.eventTimeMillis,
      deadline: Number.isFinite(eventTime)
        ? dayjs(eventTime + PENDING_REFUND_REVIEW_WINDOW_MS)
            .kst()
            .format()
        : null,
    });

    return c.json({}, 200);
  }

  if (notification.voidedPurchaseNotification) {
    const voided = notification.voidedPurchaseNotification;
    if (voided.productType !== VOIDED_PRODUCT_TYPE_SUBSCRIPTION) {
      await logNotification({ source: 'rest/googleplay', notification });
      return c.json({}, 200);
    }

    const binding = await findBinding(InAppPurchaseStore.GOOGLE_PLAY, voided.purchaseToken);
    if (binding) {
      await enqueueJob('iap:sync', { bindingId: binding.id });
    } else {
      await logNotification({ source: 'rest/googleplay', reason: 'voided_purchase_unbound', notification });
    }

    return c.json({}, 200);
  }

  if (!notification.subscriptionNotification) {
    await logNotification({ source: 'rest/googleplay', notification });
    return c.json({}, 200);
  }

  const purchaseToken = notification.subscriptionNotification.purchaseToken;

  // 알림 타입은 보지 않는다 — 어떤 알림이든 바인딩을 찾아 락 안 조회로 확정한다.
  const binding = await findBinding(InAppPurchaseStore.GOOGLE_PLAY, purchaseToken);
  if (binding) {
    await enqueueJob('iap:sync', { bindingId: binding.id });
    return c.json({}, 200);
  }

  // 구글 알림은 환경과 무관하게 prod·dev 양쪽으로 발송된다 — dev 에 없는 바인딩을 재전송으로 기다리면
  // 성립할 수 없는 재시도가 무한히 쌓인다(현행 동작 유지). 실유저의 모든 구글 알림이 여기로 떨어지므로
  // 슬랙에 내면 결제·갱신마다 소음이 된다 — 서버 로그로만 남긴다.
  if (!production) {
    log.info('unbound google notification dropped on non-production {*}', { notification });
    return c.json({}, 200);
  }

  // 미지 토큰은 무시 전에 조회한다 — 앱 밖 재가입은 새 토큰으로만 알림이 오므로 무시하면 승계 신호를 볼 기회가 없다.
  const result = await googleplay.getSubscriptionV2(purchaseToken);
  if (result.kind !== 'ok') {
    await logNotification({ source: 'rest/googleplay', reason: `unbound_lookup_${result.kind}`, notification });
    return c.json({ error: 'retry' }, 400);
  }

  const purchase = result.purchase;
  const successorTokens = [purchase.linkedPurchaseToken, purchase.outOfAppPurchaseContext?.expiredPurchaseToken].filter(
    (token): token is string => !!token,
  );

  if (successorTokens.length === 0) {
    // 진짜 독립 토큰 — 400 재시도가 등록 경합 완충이다(백오프 동안 등록 mutation 이 바인딩을 만들면 다음 전송이 정상 경로를 탄다).
    // 그 완충 구간을 알람하면 정상 등록 지연이 매번 알람이 된다 — 코드는 유지하고 알람만 신선도로 거른다.
    if (isBeyondEnrollmentRace(Number(notification.eventTimeMillis))) {
      await alertOnce('iap-unbound-independent-notification', purchaseToken, {
        purchaseToken,
        notificationType: notification.subscriptionNotification.notificationType,
      });

      // 종결 알림은 완충이 끝난 시점에 닫는다 — 여기서 400 을 유지하면 Pub/Sub 보존 기한까지
      // 같은 알림이 재전송되며 하루 1회 알람이 반복된다(성립할 수 없는 재시도).
      if (TERMINAL_SUBSCRIPTION_NOTIFICATION_TYPES.has(notification.subscriptionNotification.notificationType)) {
        await logNotification({ source: 'rest/googleplay', reason: 'unbound_terminal_settled', notification });
        return c.json({}, 200);
      }
    }

    return c.json({ error: 'retry' }, 400);
  }

  // 승계 신호는 전역 기준으로 판정한다 — 같은 스토어 계정의 이전 계약은 다른 타이피 유저에게 연결돼 있을 수 있다.
  const predecessors = await db
    .select({ id: UserInAppPurchases.id, userId: UserInAppPurchases.userId, identifier: UserInAppPurchases.identifier })
    .from(UserInAppPurchases)
    .where(and(eq(UserInAppPurchases.store, InAppPurchaseStore.GOOGLE_PLAY), inArray(UserInAppPurchases.identifier, successorTokens)));

  const captured = successorTokens.flatMap((token) => predecessors.filter((row) => row.identifier === token))[0] ?? null;

  if (!captured) {
    // 연속 플랜 변경의 역순 도착이면 선행 알림 처리로 자연 해소된다 — 재전송을 유도하되, 해소되지 않으면 알람이다.
    if (isBeyondEnrollmentRace(Number(notification.eventTimeMillis))) {
      await alertOnce('iap-succession-target-unregistered', purchaseToken, {
        purchaseToken,
        successorTokens,
        eventTimeMillis: notification.eventTimeMillis,
      });
    }

    return c.json({ error: 'retry' }, 503);
  }

  const outcome = await adoptUnboundGoogleNotification({ purchaseToken, purchase, captured });

  // 락 안에서 귀속이 달라졌으면 처리하지 않는다 — 재전송이 새 귀속으로 다시 탄다.
  if (outcome.kind === 'changed') {
    return c.json({ error: 'retry' }, 400);
  }

  // canonical 부재는 사람이 고칠 불변식 위반이다 — 200 으로 삼키면 수리 후 이 알림을 다시 태울 트리거가 없어
  // 유저가 앱을 열 때까지 잠긴다. 재전송 창이 수리 시간을 감당한다(syncIapBinding 의 deferred 와 같은 취급).
  if (outcome.kind === 'invariant') {
    await alertOnce('invariant-violation', purchaseToken, {
      source: 'rest/googleplay',
      reason: outcome.reason,
      bindingId: outcome.bindingId,
      purchaseToken,
    });

    return c.json({ error: 'retry' }, 500);
  }

  if (outcome.kind === 'applied') {
    await settleAcknowledge(outcome.acknowledge);
    await enqueueJob('iap:ingest', { bindingId: outcome.bindingId });
  } else if (outcome.kind === 'bound' || outcome.kind === 'not-adopted') {
    await enqueueJob('iap:sync', { bindingId: outcome.bindingId });
  } else if (outcome.kind === 'foreign') {
    await opsAlert('iap-foreign-predecessor-observed', {
      source: 'rest/googleplay',
      purchaseToken,
      predecessorBindingId: outcome.bindingId,
      predecessorUserId: outcome.userId,
      obfuscatedAccountId: outcome.obfuscatedAccountId,
    });
  } else if (outcome.kind === 'conflict') {
    await opsAlert('iap-promotion-conflict-skipped', {
      source: 'rest/googleplay',
      identifier: purchaseToken,
      userId: outcome.userId,
      subscriptionId: outcome.subscriptionId,
      conflictingSubscriptionId: outcome.conflictingSubscriptionId,
    });
    await settleAcknowledge(outcome.acknowledge);
  }

  return c.json({}, 200);
});
