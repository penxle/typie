export type FrameProof = {
  revision: number;
  surfaceKey: number;
  frameKey: number;
};

export type PublicationTarget = {
  key: number;
  requiredRevision: number | undefined;
  proof: FrameProof | undefined;
  available: boolean;
};

export type VisualHostFacts = {
  targets: ReadonlyMap<number, PublicationTarget>;
};

type PublishedFrameFacts = {
  surfaceKey: number;
};

function matchesTargets(frames: ReadonlyMap<number, PublishedFrameFacts>, targets: ReadonlyMap<number, PublicationTarget>): boolean {
  if (frames.size !== targets.size) return false;
  for (const [page, target] of targets) {
    if (!target.available || frames.get(page)?.surfaceKey !== target.key) return false;
  }
  return true;
}

export function proofSatisfies(target: PublicationTarget): boolean {
  if (!target.available || target.requiredRevision === undefined) return false;
  return target.proof !== undefined && target.proof.revision >= target.requiredRevision && target.proof.surfaceKey === target.key;
}

export function satisfiesWaiter(
  requestedRevision: number,
  publishedRevision: number | undefined,
  frames: ReadonlyMap<number, PublishedFrameFacts>,
  host: VisualHostFacts | undefined,
  requireFrame = false,
): boolean {
  if (!host || publishedRevision === undefined || publishedRevision < requestedRevision) return false;
  if (requireFrame && frames.size === 0) return false;

  return matchesTargets(frames, host.targets) || (!requireFrame && host.targets.size === 0 && frames.size > 0);
}

export function preparingPage({
  hasPublishedFrames,
  appliedRevision,
  publishedRevision,
  appliedPageCount,
  publishedPageCount,
  targets,
}: {
  hasPublishedFrames: boolean;
  appliedRevision: number;
  publishedRevision: number | undefined;
  appliedPageCount: number;
  publishedPageCount: number;
  targets: Pick<ReadonlyMap<number, PublicationTarget>, 'size' | 'has'> | undefined;
}): number | undefined {
  if (
    !targets ||
    !hasPublishedFrames ||
    publishedRevision === undefined ||
    appliedRevision <= publishedRevision ||
    appliedPageCount === 0
  ) {
    return undefined;
  }
  if (targets.size === 0) return 0;
  return appliedPageCount > publishedPageCount && !targets.has(publishedPageCount) ? publishedPageCount : undefined;
}

export function canPublish(
  appliedRevision: number,
  publishedRevision: number | undefined,
  host: VisualHostFacts | undefined,
  targetSetChanged = false,
  hasPublishedFrames = false,
): boolean {
  if (!host || (publishedRevision !== undefined && appliedRevision < publishedRevision)) return false;
  if (host.targets.size === 0 && hasPublishedFrames) return false;
  let hasRequirement = false;
  for (const target of host.targets.values()) {
    if (target.requiredRevision === undefined) continue;
    hasRequirement = true;
    if (!proofSatisfies(target)) return false;
  }
  return publishedRevision === undefined || appliedRevision > publishedRevision || hasRequirement || targetSetChanged;
}
