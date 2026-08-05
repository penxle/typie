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

type PublishedFrameFacts = {
  surfaceKey: number;
};

export function proofSatisfies(target: PublicationTarget): boolean {
  if (!target.available || target.proof === undefined || target.proof.surfaceKey !== target.key) return false;
  return target.requiredRevision === undefined || target.proof.revision >= target.requiredRevision;
}

export function satisfiesWaiter(
  requestedRevision: number,
  publishedRevision: number | undefined,
  frames: ReadonlyMap<number, PublishedFrameFacts>,
  requireFrame = false,
): boolean {
  if (publishedRevision === undefined || publishedRevision < requestedRevision) return false;
  return !requireFrame || frames.size > 0;
}
