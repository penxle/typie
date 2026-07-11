import {
  isOwnershipCapableDocumentAssetId,
  selectAuthorizedDocumentAssetIds,
  validateAndCanonicalizeDocumentAssetIds,
} from './document-asset-policy.ts';

export type DocumentAssetAccess = {
  loadOwnedIds(input: { userId: string; ids: string[] }): Promise<readonly string[]>;
  loadReferencedIds(input: { documentId: string }): Promise<readonly string[]>;
};

export async function resolveDocumentAssetsByIds({
  documentId,
  userId,
  requestedIds,
  access,
}: {
  documentId: string;
  userId: string | null;
  requestedIds: readonly string[];
  access: DocumentAssetAccess;
}): Promise<string[]> {
  const canonicalIds = validateAndCanonicalizeDocumentAssetIds(requestedIds);
  if (canonicalIds.length === 0) {
    return [];
  }
  const ownershipCandidateIds = canonicalIds.filter(isOwnershipCapableDocumentAssetId);

  const [ownedIds, referencedIds] = await Promise.all([
    userId !== null && ownershipCandidateIds.length > 0 ? access.loadOwnedIds({ userId, ids: ownershipCandidateIds }) : Promise.resolve([]),
    access.loadReferencedIds({ documentId }),
  ]);

  return selectAuthorizedDocumentAssetIds({
    canonicalIds,
    ownedIds,
    referencedIds,
  });
}
