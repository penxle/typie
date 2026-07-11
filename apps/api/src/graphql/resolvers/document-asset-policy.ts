import { TableCode } from '../../db/schemas/codes.ts';
import { decodeDbId } from '../../db/schemas/id.ts';

export const MAX_DOCUMENT_ASSET_IDS = 50;

const supportedTableCodes = new Set<string>([TableCode.IMAGES, TableCode.FILES, TableCode.EMBEDS, TableCode.DOCUMENT_ARCHIVED_NODES]);

const dbIdPattern = /^[A-Z]+0[A-Z0-9]+$/;

export function validateAndCanonicalizeDocumentAssetIds(requestedIds: readonly string[]): string[] {
  if (requestedIds.length > MAX_DOCUMENT_ASSET_IDS) {
    throw new Error('Too many document asset ids');
  }

  const canonicalIds: string[] = [];
  const seen = new Set<string>();
  for (const id of requestedIds) {
    if (!dbIdPattern.test(id) || !supportedTableCodes.has(decodeDbId(id))) {
      throw new Error('Invalid document asset id');
    }
    if (!seen.has(id)) {
      seen.add(id);
      canonicalIds.push(id);
    }
  }

  return canonicalIds;
}

export function selectAuthorizedDocumentAssetIds({
  canonicalIds,
  ownedIds,
  referencedIds,
}: {
  canonicalIds: readonly string[];
  ownedIds: readonly string[];
  referencedIds: readonly string[];
}): string[] {
  const authorizedIds = new Set([...ownedIds, ...referencedIds]);
  return canonicalIds.filter((id) => authorizedIds.has(id));
}

export function isOwnershipCapableDocumentAssetId(id: string): boolean {
  const tableCode = decodeDbId(id);
  return tableCode === TableCode.IMAGES || tableCode === TableCode.FILES || tableCode === TableCode.EMBEDS;
}
