import { InAppPurchaseStore } from '@typie/lib/enums';
import dayjs from 'dayjs';
import * as appstore from '#/external/appstore.ts';
import * as googleplay from '#/external/googleplay.ts';
import {
  isAppleTerminated,
  mapUnsupportedStorePayloadReason,
  normalizeApple,
  normalizeGoogle,
  selectAppleStatusItem,
  validateIapRegistrationOwnership,
} from './iap-normalize.ts';
import { opsAlertOnce } from './ops-alert.ts';
import type { PlanInterval } from '@typie/lib/enums';
import type { AppleStatusItem, IapPriorPeriod, NormalizedIap } from './iap-normalize.ts';

type TrackedIap = Extract<NormalizedIap, { kind: 'tracked' }>;

export type IapEnrollObservation = {
  identifier: string;
  normalized: TrackedIap;
  // 구독이 처음 부여된 시각. 승계·기존 행에는 쓰지 않는다(보존).
  startsAt: dayjs.Dayjs;
  // 스토어 계정 식별자로 현재 세션 소유가 증명됐는지. 증명 없이는 타 유저 바인딩을 회수하지 않는다.
  ownershipVerified: boolean;
  // 스토어가 명시적으로 선언한 승계 포인터. 전역에도 없으면 변칙 신호다(Apple 은 선언이 없어 항상 비어 있다).
  declaredPredecessors: string[];
  // 관련 유저 확정(잠금 범위) 대상 토큰. 같은 계약의 이전 토큰이 다른 유저에 남아 있으면 한 스토어 계약이 두 유저의 권한이 된다.
  // 잠금은 넓게(관측 가능한 계보 전부), 쓰기는 좁게(successionSources) 가른다.
  predecessorTokens: string[];
  // 스토어가 확정 종료를 확인해 준 원천. 이 토큰들에서 요청 토큰으로의 교체만 검증된 승계이고(그 밖은 독립 토큰 거절),
  // 회수(타 유저 바인딩 쓰기)도 이 집합으로만 한다.
  successionSources: string[];
};

export type IapEnrollLookup =
  | { kind: 'observed'; observation: IapEnrollObservation }
  | { kind: 'rejected'; reason: 'ownership-mismatch' | 'family-shared' | 'payload-invalid' | 'not-trackable'; detail: string }
  | { kind: 'lookup-failed'; detail: string };

const untrackedDetail = (normalized: NormalizedIap): string =>
  normalized.kind === 'tracked' || normalized.kind === 'expired' ? normalized.kind : `${normalized.kind}:${normalized.reason}`;

// 같은 고객·같은 구독 그룹의 다른 원거래 ID. Apple 은 승계 포인터를 싣지 않으므로 계보가 그 자리를 대신한다.
const appleLineageTokens = (items: AppleStatusItem[], selected: AppleStatusItem, requestedOriginalTransactionId: string): string[] => {
  const appTransactionId = selected.transaction?.appTransactionId;
  const subscriptionGroupIdentifier = selected.subscriptionGroupIdentifier;
  if (!appTransactionId || !subscriptionGroupIdentifier) {
    return [];
  }

  return [
    ...new Set(
      items.flatMap((item) => {
        const originalTransactionId = item.transaction?.originalTransactionId;
        if (
          !originalTransactionId ||
          originalTransactionId === requestedOriginalTransactionId ||
          item.transaction?.appTransactionId !== appTransactionId ||
          item.subscriptionGroupIdentifier !== subscriptionGroupIdentifier
        ) {
          return [];
        }

        return [originalTransactionId];
      }),
    ),
  ];
};

const lookupApple = async ({
  originalTransactionId,
  sessionUuid,
  prior,
  now,
}: {
  originalTransactionId: string;
  sessionUuid: string;
  prior: IapPriorPeriod;
  now: dayjs.Dayjs;
}): Promise<IapEnrollLookup> => {
  const statuses = await appstore.getSubscriptionStatuses(originalTransactionId);
  if (statuses.kind === 'error') {
    return { kind: 'lookup-failed', detail: 'apple-lookup-failed' };
  }

  const selection = selectAppleStatusItem(statuses.items, originalTransactionId);
  if (selection.kind === 'unknown') {
    return { kind: 'rejected', reason: 'not-trackable', detail: selection.reason };
  }

  const transaction = selection.item.transaction;
  if (!transaction?.originalTransactionId) {
    return { kind: 'rejected', reason: 'payload-invalid', detail: 'apple-transaction-missing' };
  }

  const verdict = validateIapRegistrationOwnership({
    sessionUuid,
    apple: {
      transactionAppAccountToken: transaction.appAccountToken ?? null,
      renewalAppAccountToken: selection.item.renewalInfo?.appAccountToken ?? null,
      renewalOriginalTransactionId: selection.item.renewalInfo?.originalTransactionId ?? null,
      transactionOriginalTransactionId: transaction.originalTransactionId,
      requestedOriginalTransactionId: originalTransactionId,
      inAppOwnershipType: transaction.inAppOwnershipType ?? null,
    },
  });

  if (verdict.kind === 'rejected') {
    return {
      kind: 'rejected',
      reason: verdict.reason === 'family-shared' ? 'family-shared' : 'ownership-mismatch',
      detail: `apple-${verdict.reason}`,
    };
  }

  const normalized = normalizeApple({ item: selection.item, prior, now });
  if (normalized.kind === 'unknown') {
    const alertId = mapUnsupportedStorePayloadReason(normalized.reason);
    if (alertId) {
      await opsAlertOnce(alertId, originalTransactionId, {
        source: 'utils/iap-enroll#lookupApple',
        identifier: originalTransactionId,
        reason: normalized.reason,
      });
    }
  }
  if (normalized.kind !== 'tracked') {
    return { kind: 'rejected', reason: 'not-trackable', detail: untrackedDetail(normalized) };
  }

  const lineage = appleLineageTokens(statuses.items, selection.item, originalTransactionId);

  // 승계 원천은 스토어가 종결을 확인해 준 계보 항목뿐이다 — 살아 있는 계약의 토큰을 교체하면 그 계약이 추적에서 이탈하고,
  // 그 토큰을 쥔 타 유저의 권한을 끊는다. 종료 판정에 완충(defer)을 섞지 않는 이유는 isAppleTerminated 참조.
  // 항목이 유일하게 선택되지 않는 계보(복수·서명 시각 동률)도 원천이 아니다.
  const successionSources = lineage.filter((token) => {
    const predecessor = selectAppleStatusItem(statuses.items, token);

    return predecessor.kind === 'selected' && isAppleTerminated(predecessor.item);
  });

  return {
    kind: 'observed',
    observation: {
      identifier: originalTransactionId,
      normalized,
      startsAt: transaction.originalPurchaseDate === undefined ? normalized.periodStartsAt : dayjs(transaction.originalPurchaseDate),
      ownershipVerified: !verdict.legacy,
      declaredPredecessors: [],
      predecessorTokens: lineage,
      successionSources,
    },
  };
};

const lookupGoogle = async ({
  purchaseToken,
  sessionUuid,
  prior,
  planIntervals,
  now,
}: {
  purchaseToken: string;
  sessionUuid: string;
  prior: IapPriorPeriod;
  planIntervals: Record<string, PlanInterval>;
  now: dayjs.Dayjs;
}): Promise<IapEnrollLookup> => {
  const result = await googleplay.getSubscriptionV2(purchaseToken);
  if (result.kind !== 'ok') {
    return { kind: 'lookup-failed', detail: `google-${result.kind}` };
  }

  const purchase = result.purchase;

  const verdict = validateIapRegistrationOwnership({
    sessionUuid,
    google: {
      obfuscatedExternalAccountId: purchase.externalAccountIdentifiers?.obfuscatedExternalAccountId ?? null,
      expiredObfuscatedExternalAccountId:
        purchase.outOfAppPurchaseContext?.expiredExternalAccountIdentifiers?.obfuscatedExternalAccountId ?? null,
    },
  });

  if (verdict.kind === 'rejected') {
    return { kind: 'rejected', reason: 'ownership-mismatch', detail: `google-${verdict.reason}` };
  }

  const normalized = normalizeGoogle({ purchase, prior, planIntervals, now });
  if (normalized.kind === 'unknown') {
    const alertId = mapUnsupportedStorePayloadReason(normalized.reason);
    if (alertId) {
      await opsAlertOnce(alertId, purchaseToken, {
        source: 'utils/iap-enroll#lookupGoogle',
        identifier: purchaseToken,
        reason: normalized.reason,
      });
    }
  }
  if (normalized.kind !== 'tracked') {
    return { kind: 'rejected', reason: 'not-trackable', detail: untrackedDetail(normalized) };
  }

  return {
    kind: 'observed',
    observation: {
      identifier: purchaseToken,
      normalized,
      startsAt: purchase.startTime ? dayjs(purchase.startTime) : normalized.periodStartsAt,
      ownershipVerified: !verdict.legacy,
      declaredPredecessors: normalized.successorTokens,
      predecessorTokens: normalized.successorTokens,
      successionSources: normalized.successorTokens,
    },
  };
};

// 등록 경로의 스토어 조회·소유 증거·정규화 한 묶음. 락 밖 1차(관련 유저 확정)와 락 안 재조회가 같은 함수를 쓴다 —
// 갈라지면 락 밖 관측으로 적용하는 문이 열린다.
export const lookupIapEnrollment = async ({
  store,
  data,
  sessionUuid,
  prior,
  planIntervals,
  now,
}: {
  store: InAppPurchaseStore;
  data: string;
  sessionUuid: string;
  prior: IapPriorPeriod;
  planIntervals: Record<string, PlanInterval>;
  now: dayjs.Dayjs;
}): Promise<IapEnrollLookup> => {
  if (store === InAppPurchaseStore.APP_STORE) {
    return await lookupApple({ originalTransactionId: data, sessionUuid, prior, now });
  }

  return await lookupGoogle({ purchaseToken: data, sessionUuid, prior, planIntervals, now });
};
