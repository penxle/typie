import { SubscriptionState } from '@typie/lib/enums';
import { isAppleTerminated, selectAppleStatusItem } from './iap-normalize.ts';
import type { InAppPurchaseStore } from '@typie/lib/enums';
import type { AppleStatusItem } from './iap-normalize.ts';

// 같은 고객·같은 구독 그룹의 다른 원거래 ID. Apple 은 승계 포인터를 싣지 않으므로 계보가 그 자리를 대신한다.
export const appleLineageTokens = (
  items: AppleStatusItem[],
  selected: AppleStatusItem,
  requestedOriginalTransactionId: string,
): string[] => {
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

export const resolveEnrollTarget = <T extends { id: string; store: InAppPurchaseStore; identifier: string }>(
  bindings: T[],
  { store, identifier, lineageTokens }: { store: InAppPurchaseStore; identifier: string; lineageTokens: string[] },
): T | null => {
  const sameStore = bindings.filter((binding) => binding.store === store);
  const same = sameStore.find((binding) => binding.identifier === identifier);
  if (same) {
    return same;
  }

  for (const token of lineageTokens) {
    const predecessor = sameStore.find((binding) => binding.identifier === token);
    if (predecessor) {
      return predecessor;
    }
  }

  return null;
};

export type ApplePredecessorCandidate = { id: string; userId: string; identifier: string; canonicalState: SubscriptionState | null };

export type ApplePredecessorSelection =
  | { kind: 'selected'; candidate: ApplePredecessorCandidate }
  | { kind: 'none' }
  | { kind: 'ambiguous'; count: number }
  | { kind: 'live'; count: number }
  | { kind: 'foreign'; userIds: string[] };

export const selectApplePredecessor = ({
  candidates,
  items,
  notifiedOriginalTransactionId,
  ownerUserId,
}: {
  candidates: ApplePredecessorCandidate[];
  items: AppleStatusItem[];
  notifiedOriginalTransactionId: string;
  ownerUserId: string | null;
}): ApplePredecessorSelection => {
  const notified = selectAppleStatusItem(items, notifiedOriginalTransactionId);
  if (notified.kind !== 'selected') {
    return { kind: 'none' };
  }

  const lineage = new Set(appleLineageTokens(items, notified.item, notifiedOriginalTransactionId));
  const related = candidates.filter((candidate) => lineage.has(candidate.identifier));
  if (related.length === 0) {
    return { kind: 'none' };
  }

  const userIds = [...new Set(related.map((candidate) => candidate.userId))];
  if (ownerUserId !== null && userIds.some((userId) => userId !== ownerUserId)) {
    return { kind: 'foreign', userIds: userIds.filter((userId) => userId !== ownerUserId) };
  }
  if (userIds.length > 1) {
    return { kind: 'ambiguous', count: related.length };
  }

  const isTerminated = (candidate: ApplePredecessorCandidate): boolean => {
    if (candidate.canonicalState === SubscriptionState.EXPIRED) {
      return true;
    }

    const selection = selectAppleStatusItem(items, candidate.identifier);
    return selection.kind === 'selected' && isAppleTerminated(selection.item);
  };

  const terminated = related.filter(isTerminated);
  if (terminated.length < related.length) {
    return { kind: 'live', count: related.length - terminated.length };
  }
  if (terminated.length > 1) {
    return { kind: 'ambiguous', count: terminated.length };
  }

  return { kind: 'selected', candidate: terminated[0] };
};
