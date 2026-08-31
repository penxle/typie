import { InAppPurchaseStore, PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, eq, isNotNull, isNull, ne } from 'drizzle-orm';
import { db, first, Plans, Subscriptions, UserInAppPurchases } from '#/db/index.ts';
import * as appstore from '#/external/appstore.ts';
import * as googleplay from '#/external/googleplay.ts';
import { isSubscriptionEntitled } from './entitlement.ts';
import {
  discoverAppleSuccessor,
  mapUnsupportedStorePayloadReason,
  normalizeApple,
  normalizeGoogle,
  selectAppleStatusItem,
} from './iap-normalize.ts';
import { findPromotionConflict, isRevivalGated } from './iap-sync-core.ts';
import { opsAlert, opsAlertOnce } from './ops-alert.ts';
import { lockUserSubscriptionState } from './subscription-lock.ts';
import type { PlanInterval } from '@typie/lib/enums';
import type { Transaction } from '#/db/index.ts';
import type { IapPriorPeriod, NormalizedIap } from './iap-normalize.ts';
import type { ConflictCandidate } from './iap-sync-core.ts';

export type SyncIapOutcome =
  | { kind: 'applied' }
  | { kind: 'deferred' }
  | { kind: 'gone' }
  | { kind: 'skipped'; reason: 'binding-missing' | 'conflict' | 'terminated-unresolved' };

export type IapAcknowledgeDuty = { productId: string; purchaseToken: string };

type LockedIapBinding = {
  id: string;
  userId: string;
  store: InAppPurchaseStore;
  identifier: string;
  subscriptionId: string;
};

// 저장된 추적 가능 토큰의 PENDING 승인은 권한·전이 여부와 무관한 의무다(결제 성립 후 3일 자동 환불 차단).
export const resolveAcknowledgeDuty = (normalized: NormalizedIap, purchaseToken: string): IapAcknowledgeDuty | null =>
  normalized.kind === 'tracked' && normalized.acknowledgePending && normalized.productId
    ? { productId: normalized.productId, purchaseToken }
    : null;

// 승격 전 충돌 검사는 primitive 를 직접 부르는 경로가 스스로 해야 한다 — syncIapBinding 을 우회하면 검사도 함께
// 우회된다. 유니크 위반은 경합의 최후 방어일 뿐이고, WILL_ACTIVATE 는 부분 유니크가 분리되어 DB 가 잡지도 못한다.
export const loadConflictCandidates = async (
  tx: Transaction,
  { userId, excludeSubscriptionId }: { userId: string; excludeSubscriptionId: string },
): Promise<ConflictCandidate[]> =>
  await tx
    .select({
      id: Subscriptions.id,
      state: Subscriptions.state,
      planAvailability: Plans.availability,
      startsAt: Subscriptions.startsAt,
      currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
      currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(
      and(
        eq(Subscriptions.userId, userId),
        ne(Subscriptions.id, excludeSubscriptionId),
        ne(Subscriptions.state, SubscriptionState.EXPIRED),
      ),
    );

// 락 안 공통 적용점. 공개 wrapper 를 락 안에서 재호출하면 별도 커넥션의 advisory 락이 자기 자신을 기다리므로
// 재조정·등록·웹훅 승계가 전부 이 primitive 를 공유한다.
// 반환하는 acknowledge 는 커밋 후 의무다 — 롤백된 트랜잭션의 토큰을 승인하지 않는다.
export const applyNormalizedIapLocked = async (
  tx: Transaction,
  {
    binding,
    normalized,
    newIdentifier,
  }: {
    binding: LockedIapBinding;
    normalized: NormalizedIap;
    newIdentifier?: string;
  },
): Promise<{ acknowledge: IapAcknowledgeDuty | null }> => {
  const now = dayjs();

  if (newIdentifier !== undefined && newIdentifier !== binding.identifier) {
    await tx.update(UserInAppPurchases).set({ identifier: newIdentifier }).where(eq(UserInAppPurchases.id, binding.id));
  }

  if (normalized.kind === 'expired') {
    // 주기 컬럼은 자르지 않는다 — 회수는 상태가 표현한다.
    await tx.update(Subscriptions).set({ state: SubscriptionState.EXPIRED }).where(eq(Subscriptions.id, binding.subscriptionId));
    await tx
      .update(UserInAppPurchases)
      .set({ terminatedAt: now })
      .where(and(eq(UserInAppPurchases.id, binding.id), isNull(UserInAppPurchases.terminatedAt)));

    return { acknowledge: null };
  }

  if (normalized.kind !== 'tracked') {
    return { acknowledge: null };
  }

  const acknowledge = resolveAcknowledgeDuty(normalized, newIdentifier ?? binding.identifier);

  const canonical = await tx
    .select({ state: Subscriptions.state })
    .from(Subscriptions)
    .where(eq(Subscriptions.id, binding.subscriptionId))
    .then(first);

  if (isRevivalGated(canonical?.state, normalized, now)) {
    return { acknowledge };
  }

  const plan = await tx
    .select({ id: Plans.id })
    .from(Plans)
    .where(and(eq(Plans.id, normalized.planKey), eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)))
    .then(first);

  await tx
    .update(Subscriptions)
    .set({
      state: normalized.state,
      currentPeriodStartsAt: normalized.periodStartsAt,
      currentPeriodEndsAt: normalized.periodEndsAt,
      ...(plan && { planId: plan.id }),
    })
    .where(eq(Subscriptions.id, binding.subscriptionId));

  await tx
    .update(UserInAppPurchases)
    .set({ terminatedAt: null })
    .where(and(eq(UserInAppPurchases.id, binding.id), isNotNull(UserInAppPurchases.terminatedAt)));

  return { acknowledge };
};

export const syncIapBinding = async ({ bindingId }: { bindingId: string }): Promise<SyncIapOutcome> => {
  const { outcome, acknowledge } = await db.transaction(
    async (tx): Promise<{ outcome: SyncIapOutcome; acknowledge: IapAcknowledgeDuty | null }> => {
      const bindingRef = await tx
        .select({ userId: UserInAppPurchases.userId })
        .from(UserInAppPurchases)
        .where(eq(UserInAppPurchases.id, bindingId))
        .then(first);

      if (!bindingRef) {
        return { outcome: { kind: 'skipped', reason: 'binding-missing' }, acknowledge: null };
      }

      await lockUserSubscriptionState(tx, bindingRef.userId);

      const binding = await tx
        .select({
          id: UserInAppPurchases.id,
          userId: UserInAppPurchases.userId,
          store: UserInAppPurchases.store,
          identifier: UserInAppPurchases.identifier,
          subscriptionId: UserInAppPurchases.subscriptionId,
          terminatedAt: UserInAppPurchases.terminatedAt,
        })
        .from(UserInAppPurchases)
        .where(eq(UserInAppPurchases.id, bindingId))
        .for('no key update')
        .then(first);

      if (!binding) {
        return { outcome: { kind: 'skipped', reason: 'binding-missing' }, acknowledge: null };
      }

      // 락 대기 중 소유가 옮겨졌으면 우리가 쥔 것은 이전 유저의 락이다 — 그 아래에서 갱신하지 않는다.
      if (binding.userId !== bindingRef.userId) {
        return { outcome: { kind: 'deferred' }, acknowledge: null };
      }

      // 종료가 확정되지 않았는데 canonical 이 없는 것이 불변식 위반이다. deferred 로 남겨 사람이 canonical 을 채우면
      // 잡 재시도·일일 재조정이 회복한다.
      if (!binding.subscriptionId && !binding.terminatedAt) {
        await opsAlert('invariant-violation', {
          reason: 'iap binding without canonical subscription',
          bindingId,
          userId: binding.userId,
        });

        return { outcome: { kind: 'deferred' }, acknowledge: null };
      }

      // 종료가 확정된 바인딩의 canonical 부재는 정상 종결이다 — 재시도로 풀리지 않으므로 조용히 끝낸다.
      if (!binding.subscriptionId) {
        return { outcome: { kind: 'skipped', reason: 'terminated-unresolved' }, acknowledge: null };
      }

      const canonical = await tx
        .select({
          id: Subscriptions.id,
          state: Subscriptions.state,
          planAvailability: Plans.availability,
          startsAt: Subscriptions.startsAt,
          currentPeriodStartsAt: Subscriptions.currentPeriodStartsAt,
          currentPeriodEndsAt: Subscriptions.currentPeriodEndsAt,
          createdAt: Subscriptions.createdAt,
        })
        .from(Subscriptions)
        .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
        .where(eq(Subscriptions.id, binding.subscriptionId))
        .for('no key update', { of: Subscriptions })
        .then(first);

      if (!canonical) {
        await opsAlert('invariant-violation', {
          reason: 'iap binding canonical subscription missing',
          bindingId,
          subscriptionId: binding.subscriptionId,
        });

        return { outcome: { kind: 'deferred' }, acknowledge: null };
      }

      const now = dayjs();
      const prior: IapPriorPeriod = {
        state: canonical.state,
        currentPeriodStartsAt: canonical.currentPeriodStartsAt,
        currentPeriodEndsAt: canonical.currentPeriodEndsAt,
      };

      const plans = await tx
        .select({ id: Plans.id, interval: Plans.interval })
        .from(Plans)
        .where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE));
      const planIntervals: Record<string, PlanInterval> = Object.fromEntries(plans.map((plan) => [plan.id, plan.interval]));

      const lockedBinding: LockedIapBinding = {
        id: binding.id,
        userId: binding.userId,
        store: binding.store,
        identifier: binding.identifier,
        subscriptionId: binding.subscriptionId,
      };

      let normalized: NormalizedIap;
      let newIdentifier: string | undefined;

      // 스토어 조회는 유저 advisory 락 안이다 — 라이브 응답이 곧 최신이라 stale 판별 규칙이 사라진다.
      // 조회 정지는 런타임 기본 타임아웃과 lock_timeout 이 유계한다.
      if (binding.store === InAppPurchaseStore.APP_STORE) {
        const statuses = await appstore.getSubscriptionStatuses(binding.identifier);
        if (statuses.kind === 'error') {
          return { outcome: { kind: 'deferred' }, acknowledge: null };
        }

        const selection = selectAppleStatusItem(statuses.items, binding.identifier);
        if (selection.kind === 'unknown') {
          if (selection.reason === 'apple-transaction-id-mismatch') {
            await opsAlert('apple-transaction-id-mismatch', { bindingId, identifier: binding.identifier });
          }

          return { outcome: { kind: 'deferred' }, acknowledge: null };
        }

        // 후계 발견은 요청 ID 항목의 정규화 결과와 무관하게 항상 수행한다.
        const successor = discoverAppleSuccessor({
          items: statuses.items,
          selected: selection.item,
          requestedOriginalTransactionId: binding.identifier,
          prior,
          now,
        });

        if (successor.kind === 'unresolved') {
          await opsAlert('apple-unbound-discovery-unresolved', {
            bindingId,
            identifier: binding.identifier,
            candidates: successor.candidates,
            canonicalState: canonical.state,
          });
        }

        let adopted: { originalTransactionId: string; normalized: NormalizedIap } | null = null;
        if (successor.kind === 'succeeded') {
          // (store, identifier) 는 전역 유니크다 — 새 원거래 ID를 다른 바인딩이 점유 중인데 교체하면 트랜잭션이
          // 유니크 위반으로 죽어 재시도마다 같은 자리에서 무너진다. 세션이 없는 이 경로는 회수·이전을 하지 않고
          // 관측만 남긴다(소유 증거가 있는 등록 경로의 몫이다).
          const occupant = await tx
            .select({ id: UserInAppPurchases.id, userId: UserInAppPurchases.userId })
            .from(UserInAppPurchases)
            .where(and(eq(UserInAppPurchases.store, binding.store), eq(UserInAppPurchases.identifier, successor.originalTransactionId)))
            .then(first);

          if (occupant) {
            await opsAlert('iap-foreign-predecessor-observed', {
              source: 'utils/iap-sync',
              bindingId,
              identifier: binding.identifier,
              successorIdentifier: successor.originalTransactionId,
              occupantBindingId: occupant.id,
              occupantUserId: occupant.userId,
            });
          } else {
            adopted = successor;
          }
        }

        if (adopted) {
          newIdentifier = adopted.originalTransactionId;
          normalized = adopted.normalized;
        } else {
          normalized = normalizeApple({ item: selection.item, prior, now });
        }
      } else {
        const subscription = await googleplay.getSubscriptionV2(binding.identifier);

        if (subscription.kind === 'gone') {
          // 살아있음의 기준은 상태가 아니라 권한이다 — 기간이 지난 WILL_EXPIRE 를 살아있다고 보면 소멸한 토큰이
          // 매일 같은 알람을 반복하고 종료 확정이 영원히 찍히지 않는다.
          if (isSubscriptionEntitled(canonical, now)) {
            await opsAlert('google-token-gone-live-canonical', {
              bindingId,
              subscriptionId: canonical.id,
              state: canonical.state,
            });

            return { outcome: { kind: 'deferred' }, acknowledge: null };
          }

          // 바인딩은 지우지 않는다 — (store, identifier) 가 앱 밖 재가입의 유일한 연결키다.
          await tx
            .update(UserInAppPurchases)
            .set({ terminatedAt: now })
            .where(and(eq(UserInAppPurchases.id, binding.id), isNull(UserInAppPurchases.terminatedAt)));

          return { outcome: { kind: 'gone' }, acknowledge: null };
        }

        // 404 는 gone 이 아니다 — 설정 오류일 수 있어 영구 제외하면 원인을 고쳐도 복권 백스톱이 돌아오지 않는다.
        if (subscription.kind === 'not-found') {
          await opsAlert('google-token-not-found', { bindingId, identifier: binding.identifier });

          return { outcome: { kind: 'deferred' }, acknowledge: null };
        }

        if (subscription.kind === 'error') {
          return { outcome: { kind: 'deferred' }, acknowledge: null };
        }

        // 자기 토큰 조회라 응답의 승계 신호는 이 바인딩 자신을 가리킨다 — 미지 토큰의 승계는 웹훅 경로의 몫이다.
        normalized = normalizeGoogle({ purchase: subscription.purchase, prior, planIntervals, now });
      }

      if (normalized.kind === 'unknown') {
        const alertId = mapUnsupportedStorePayloadReason(normalized.reason);
        if (alertId) {
          await opsAlertOnce(alertId, bindingId, {
            source: 'utils/iap-sync',
            bindingId,
            identifier: binding.identifier,
            reason: normalized.reason,
          });
        }
      }

      // defer·unknown 은 판정 보류이고, untracked(PENDING 류)는 등록 밖 경로에서 권한 근거가 아니다.
      if (normalized.kind !== 'tracked' && normalized.kind !== 'expired') {
        return { outcome: { kind: 'deferred' }, acknowledge: null };
      }

      const conflict = findPromotionConflict({
        candidates: await loadConflictCandidates(tx, { userId: binding.userId, excludeSubscriptionId: canonical.id }),
        canonical,
        normalized,
        now,
      });

      if (conflict) {
        await opsAlert('iap-promotion-conflict-skipped', {
          bindingId,
          userId: binding.userId,
          subscriptionId: canonical.id,
          conflictingSubscriptionId: conflict.id,
        });

        // 전이는 막아도 승인 의무는 남는다 — 승격을 스킵했다고 3일 자동 환불을 방치하면 돈의 불변식이 깨진다.
        return {
          outcome: { kind: 'skipped', reason: 'conflict' },
          acknowledge: resolveAcknowledgeDuty(normalized, newIdentifier ?? binding.identifier),
        };
      }

      const { acknowledge } = await applyNormalizedIapLocked(tx, { binding: lockedBinding, normalized, newIdentifier });

      return { outcome: { kind: 'applied' }, acknowledge };
    },
  );

  if (acknowledge) {
    try {
      await googleplay.acknowledgeSubscription(acknowledge);
    } catch (err) {
      await opsAlert('google-acknowledge-failed', {
        ...acknowledge,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  return outcome;
};
