import { InAppPurchaseStore } from '@typie/lib/enums';
import dayjs from 'dayjs';
import * as appstore from '#/external/appstore.ts';
import * as googleplay from '#/external/googleplay.ts';
import {
  extractIapRegistrationOwnership,
  isAppleTerminated,
  isGoogleTerminated,
  mapUnsupportedStorePayloadReason,
  normalizeApple,
  normalizeGoogle,
  selectAppleStatusItem,
} from './iap-normalize.ts';
import { opsAlertOnce } from './ops-alert.ts';
import type { androidpublisher_v3 } from '@googleapis/androidpublisher';
import type { PlanInterval } from '@typie/lib/enums';
import type { AppleStatusItem, IapPriorPeriod, IapRegistrationOwnership, NormalizedIap } from './iap-normalize.ts';

type TrackedIap = Extract<NormalizedIap, { kind: 'tracked' }>;

export type IapEnrollObservation = {
  identifier: string;
  normalized: TrackedIap;
  // 구독이 처음 부여된 시각. 승계·기존 행에는 쓰지 않는다(보존).
  startsAt: dayjs.Dayjs;
  // 스토어 계정 식별자가 소유자를 지목하는지. 증거 없이는 타 유저 바인딩을 회수하지 않는다.
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

// 등록 대상을 지목하는 소유 증거. legacy 는 증거가 실리지 않은 구매(식별자 도입 전) — 호출 세션에 귀속된다.
export type IapEnrollOwnership = Exclude<IapRegistrationOwnership, { kind: 'rejected' }>;

export type IapEnrollSource =
  | {
      store: typeof InAppPurchaseStore.APP_STORE;
      ownership: IapEnrollOwnership;
      items: AppleStatusItem[];
      selected: AppleStatusItem;
      requestedOriginalTransactionId: string;
    }
  | {
      store: typeof InAppPurchaseStore.GOOGLE_PLAY;
      ownership: IapEnrollOwnership;
      purchase: androidpublisher_v3.Schema$SubscriptionPurchaseV2;
      purchaseToken: string;
    };

export type IapEnrollRejection =
  | { kind: 'rejected'; reason: 'ownership-mismatch' | 'family-shared' | 'payload-invalid' | 'not-trackable' | 'expired'; detail: string }
  | { kind: 'lookup-failed'; detail: string };

export type IapEnrollFetch = IapEnrollRejection | { kind: 'fetched'; source: IapEnrollSource };

export type IapEnrollNormalization =
  Extract<IapEnrollRejection, { kind: 'rejected' }> | { kind: 'observed'; observation: IapEnrollObservation };

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

const fetchApple = async (originalTransactionId: string): Promise<IapEnrollFetch> => {
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

  const ownership = extractIapRegistrationOwnership({
    apple: {
      transactionAppAccountToken: transaction.appAccountToken ?? null,
      renewalAppAccountToken: selection.item.renewalInfo?.appAccountToken ?? null,
      renewalOriginalTransactionId: selection.item.renewalInfo?.originalTransactionId ?? null,
      transactionOriginalTransactionId: transaction.originalTransactionId,
      requestedOriginalTransactionId: originalTransactionId,
      inAppOwnershipType: transaction.inAppOwnershipType ?? null,
    },
  });

  if (ownership.kind === 'rejected') {
    return {
      kind: 'rejected',
      reason: ownership.reason === 'family-shared' ? 'family-shared' : 'ownership-mismatch',
      detail: `apple-${ownership.reason}`,
    };
  }

  return {
    kind: 'fetched',
    source: {
      store: InAppPurchaseStore.APP_STORE,
      ownership,
      items: statuses.items,
      selected: selection.item,
      requestedOriginalTransactionId: originalTransactionId,
    },
  };
};

const fetchGoogle = async (purchaseToken: string): Promise<IapEnrollFetch> => {
  const result = await googleplay.getSubscriptionV2(purchaseToken);
  if (result.kind !== 'ok') {
    return { kind: 'lookup-failed', detail: `google-${result.kind}` };
  }

  const purchase = result.purchase;

  const ownership = extractIapRegistrationOwnership({
    google: {
      obfuscatedExternalAccountId: purchase.externalAccountIdentifiers?.obfuscatedExternalAccountId ?? null,
      expiredObfuscatedExternalAccountId:
        purchase.outOfAppPurchaseContext?.expiredExternalAccountIdentifiers?.obfuscatedExternalAccountId ?? null,
    },
  });

  if (ownership.kind === 'rejected') {
    return { kind: 'rejected', reason: 'ownership-mismatch', detail: `google-${ownership.reason}` };
  }

  return { kind: 'fetched', source: { store: InAppPurchaseStore.GOOGLE_PLAY, ownership, purchase, purchaseToken } };
};

// 등록 경로의 스토어 조회 + 소유 증거 추출. 정규화(normalizeIapEnrollment)와 분리된 이유: 정규화에 먹일 선행
// 주기는 소유자의 것이어야 하는데, 소유자는 이 조회의 결과(증거)로만 확정된다. 락 밖 1차(소유자·관련 유저 확정)와
// 락 안 재조회가 같은 함수를 쓴다 — 갈라지면 락 밖 관측으로 적용하는 문이 열린다.
export const fetchIapEnrollment = async ({ store, data }: { store: InAppPurchaseStore; data: string }): Promise<IapEnrollFetch> => {
  if (store === InAppPurchaseStore.APP_STORE) {
    return await fetchApple(data);
  }

  return await fetchGoogle(data);
};

export type IapBoundContractProbe = { kind: 'terminated' } | { kind: 'live' } | { kind: 'lookup-failed'; detail: string };

// 독립 토큰 가드의 예외 판정: 바인딩이 가리키는 기존 계약의 확정 종료를 스토어에 재확인한다. DB 의 EXPIRED 를
// 근거로 쓰지 않는 이유는 동기화 지연이다 — 살아 있는 계약의 바인딩을 풀면 그 계약이 추적에서 이탈한다.
// 증거 기준은 승계 기제와 같다(스토어가 종결을 확인해 준 원천만).
export const probeIapBoundContractTermination = async ({
  store,
  identifier,
  now,
}: {
  store: InAppPurchaseStore;
  identifier: string;
  now: dayjs.Dayjs;
}): Promise<IapBoundContractProbe> => {
  if (store === InAppPurchaseStore.APP_STORE) {
    const statuses = await appstore.getSubscriptionStatuses(identifier);
    if (statuses.kind === 'error') {
      return { kind: 'lookup-failed', detail: 'apple-lookup-failed' };
    }

    const selection = selectAppleStatusItem(statuses.items, identifier);
    if (selection.kind === 'unknown') {
      return { kind: 'lookup-failed', detail: selection.reason };
    }

    return isAppleTerminated(selection.item) ? { kind: 'terminated' } : { kind: 'live' };
  }

  const result = await googleplay.getSubscriptionV2(identifier);
  // 410 은 토큰 영구 소멸(만료 후 보존 기간 경과 포함)이다 — 살아 있는 계약의 토큰은 소멸하지 않는다.
  // 404 는 설정 오류일 수 있어 종료 증거로 쓰지 않는다(external/googleplay 참조).
  if (result.kind === 'gone') {
    return { kind: 'terminated' };
  }
  if (result.kind !== 'ok') {
    return { kind: 'lookup-failed', detail: `google-${result.kind}` };
  }

  return isGoogleTerminated(result.purchase, now) ? { kind: 'terminated' } : { kind: 'live' };
};

const normalizeAppleEnrollment = async ({
  source,
  prior,
  now,
}: {
  source: Extract<IapEnrollSource, { store: typeof InAppPurchaseStore.APP_STORE }>;
  prior: IapPriorPeriod;
  now: dayjs.Dayjs;
}): Promise<IapEnrollNormalization> => {
  const normalized = normalizeApple({ item: source.selected, prior, now });
  if (normalized.kind === 'unknown') {
    const alertId = mapUnsupportedStorePayloadReason(normalized.reason);
    if (alertId) {
      await opsAlertOnce(alertId, source.requestedOriginalTransactionId, {
        source: 'utils/iap-enroll#normalizeAppleEnrollment',
        identifier: source.requestedOriginalTransactionId,
        reason: normalized.reason,
      });
    }
  }
  // 만료는 별도 reason 으로 가른다 — 스토어가 종료를 확정한 구매는 재시도로 절대 풀리지 않아,
  // GraphQL 표면이 클라이언트에게 트랜잭션 종료(재시도 루프 종결)를 지시할 전용 코드를 내려야 한다.
  if (normalized.kind !== 'tracked') {
    return { kind: 'rejected', reason: normalized.kind === 'expired' ? 'expired' : 'not-trackable', detail: untrackedDetail(normalized) };
  }

  const lineage = appleLineageTokens(source.items, source.selected, source.requestedOriginalTransactionId);

  // 승계 원천은 스토어가 종결을 확인해 준 계보 항목뿐이다 — 살아 있는 계약의 토큰을 교체하면 그 계약이 추적에서 이탈하고,
  // 그 토큰을 쥔 타 유저의 권한을 끊는다. 종료 판정에 완충(defer)을 섞지 않는 이유는 isAppleTerminated 참조.
  // 항목이 유일하게 선택되지 않는 계보(복수·서명 시각 동률)도 원천이 아니다.
  const successionSources = lineage.filter((token) => {
    const predecessor = selectAppleStatusItem(source.items, token);

    return predecessor.kind === 'selected' && isAppleTerminated(predecessor.item);
  });

  const originalPurchaseDate = source.selected.transaction?.originalPurchaseDate;

  return {
    kind: 'observed',
    observation: {
      identifier: source.requestedOriginalTransactionId,
      normalized,
      startsAt: originalPurchaseDate === undefined ? normalized.periodStartsAt : dayjs(originalPurchaseDate),
      ownershipVerified: source.ownership.kind === 'evidenced',
      declaredPredecessors: [],
      predecessorTokens: lineage,
      successionSources,
    },
  };
};

const normalizeGoogleEnrollment = async ({
  source,
  prior,
  planIntervals,
  now,
}: {
  source: Extract<IapEnrollSource, { store: typeof InAppPurchaseStore.GOOGLE_PLAY }>;
  prior: IapPriorPeriod;
  planIntervals: Record<string, PlanInterval>;
  now: dayjs.Dayjs;
}): Promise<IapEnrollNormalization> => {
  const normalized = normalizeGoogle({ purchase: source.purchase, prior, planIntervals, now });
  if (normalized.kind === 'unknown') {
    const alertId = mapUnsupportedStorePayloadReason(normalized.reason);
    if (alertId) {
      await opsAlertOnce(alertId, source.purchaseToken, {
        source: 'utils/iap-enroll#normalizeGoogleEnrollment',
        identifier: source.purchaseToken,
        reason: normalized.reason,
      });
    }
  }
  // 만료는 별도 reason 으로 가른다 — 스토어가 종료를 확정한 구매는 재시도로 절대 풀리지 않아,
  // GraphQL 표면이 클라이언트에게 트랜잭션 종료(재시도 루프 종결)를 지시할 전용 코드를 내려야 한다.
  if (normalized.kind !== 'tracked') {
    return { kind: 'rejected', reason: normalized.kind === 'expired' ? 'expired' : 'not-trackable', detail: untrackedDetail(normalized) };
  }

  return {
    kind: 'observed',
    observation: {
      identifier: source.purchaseToken,
      normalized,
      startsAt: source.purchase.startTime ? dayjs(source.purchase.startTime) : normalized.periodStartsAt,
      ownershipVerified: source.ownership.kind === 'evidenced',
      declaredPredecessors: normalized.successorTokens,
      predecessorTokens: normalized.successorTokens,
      successionSources: normalized.successorTokens,
    },
  };
};

// 등록 경로의 정규화·관측 조립. fetchIapEnrollment 의 결과에 소유자의 선행 주기를 먹인다 — 락 밖 1차와
// 락 안 재정규화가 같은 함수를 쓴다.
export const normalizeIapEnrollment = async ({
  source,
  prior,
  planIntervals,
  now,
}: {
  source: IapEnrollSource;
  prior: IapPriorPeriod;
  planIntervals: Record<string, PlanInterval>;
  now: dayjs.Dayjs;
}): Promise<IapEnrollNormalization> => {
  if (source.store === InAppPurchaseStore.APP_STORE) {
    return await normalizeAppleEnrollment({ source, prior, now });
  }

  return await normalizeGoogleEnrollment({ source, prior, planIntervals, now });
};
