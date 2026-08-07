import type { DocumentHeadKind } from '@typie/lib/enums';

export const ISOLATION_THRESHOLD = 250;

export type FoldedEntry = {
  userId: string;
  applied: boolean;
  charCount: number;
  grossInsertions: number;
  grossDeletions: number;
  heads: Uint8Array;
};

export type LatestHead = {
  id: string;
  kind: DocumentHeadKind;
  bucketMs: number;
  hasExcludedContributor: boolean;
};

export type Contribution = { userId: string; additions: number; deletions: number };

export type HeadWrite = {
  action: 'update' | 'insert';
  headId: string | null;
  kind: DocumentHeadKind;
  heads: Uint8Array;
  characterCount: number;
  isolatedAuthorId: string | null;
  contributions: Contribution[];
  contributorUserIds: string[];
};

type Segment = {
  nets: Map<string, number>;
  userIds: Set<string>;
  heads: Uint8Array;
  charCount: number;
};

const toContributions = (nets: Map<string, number>): Contribution[] =>
  [...nets].filter(([, net]) => net !== 0).map(([userId, net]) => ({ userId, additions: Math.max(net, 0), deletions: Math.max(-net, 0) }));

export const planHeadWrites = (input: {
  entries: FoldedEntry[];
  baseCharCount: number;
  latestHead: LatestHead | null;
  bucketMs: number;
  systemUserId: string;
  threshold?: number;
}): HeadWrite[] => {
  const threshold = input.threshold ?? ISOLATION_THRESHOLD;
  const writes: HeadWrite[] = [];
  let prevCharCount = input.baseCharCount;
  let segment: Segment | null = null;
  let foldableHead =
    input.latestHead !== null &&
    input.latestHead.kind === 'NORMAL' &&
    input.latestHead.bucketMs === input.bucketMs &&
    !input.latestHead.hasExcludedContributor
      ? input.latestHead
      : null;

  const flushSegment = () => {
    if (!segment) {
      return;
    }

    const contributorUserIds = [...segment.userIds].filter((id) => id !== input.systemUserId);

    if (foldableHead) {
      writes.push({
        action: 'update',
        headId: foldableHead.id,
        kind: 'NORMAL',
        heads: segment.heads,
        characterCount: segment.charCount,
        isolatedAuthorId: null,
        contributions: toContributions(segment.nets),
        contributorUserIds,
      });

      foldableHead = null;
    } else {
      writes.push({
        action: 'insert',
        headId: null,
        kind: 'NORMAL',
        heads: segment.heads,
        characterCount: segment.charCount,
        isolatedAuthorId: null,
        contributions: toContributions(segment.nets),
        contributorUserIds,
      });
    }

    segment = null;
  };

  for (const entry of input.entries) {
    if (!entry.applied) {
      continue;
    }

    const net = entry.charCount - prevCharCount;
    prevCharCount = entry.charCount;

    const isolated = entry.userId !== input.systemUserId && Math.max(entry.grossInsertions, entry.grossDeletions) >= threshold;

    if (isolated) {
      flushSegment();
      foldableHead = null;

      writes.push({
        action: 'insert',
        headId: null,
        kind: 'ISOLATED',
        heads: entry.heads,
        characterCount: entry.charCount,
        isolatedAuthorId: entry.userId,
        contributions: net === 0 ? [] : [{ userId: entry.userId, additions: Math.max(net, 0), deletions: Math.max(-net, 0) }],
        contributorUserIds: [entry.userId],
      });
    } else {
      if (segment) {
        segment.heads = entry.heads;
        segment.charCount = entry.charCount;
      } else {
        segment = { nets: new Map(), userIds: new Set(), heads: entry.heads, charCount: entry.charCount };
      }

      segment.userIds.add(entry.userId);
      segment.nets.set(entry.userId, (segment.nets.get(entry.userId) ?? 0) + net);
    }
  }

  flushSegment();

  return writes;
};
