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

export function proofSatisfies(target: PublicationTarget): boolean {
  if (!target.available || target.requiredRevision === undefined) return false;
  return target.proof !== undefined && target.proof.revision >= target.requiredRevision && target.proof.surfaceKey === target.key;
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
