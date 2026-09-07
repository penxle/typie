import { inArray } from 'drizzle-orm';
import { db, decodeDbId, DocumentArchivedNodes, Embeds, Files, Images, TableCode } from '#/db/index.ts';

export type GroupedAssetIds = { imageIds: string[]; fileIds: string[]; embedIds: string[]; archivedIds: string[] };

export const groupAssetIds = (ids: string[]): GroupedAssetIds => ({
  imageIds: ids.filter((id) => decodeDbId(id) === TableCode.IMAGES),
  fileIds: ids.filter((id) => decodeDbId(id) === TableCode.FILES),
  embedIds: ids.filter((id) => decodeDbId(id) === TableCode.EMBEDS),
  archivedIds: ids.filter((id) => decodeDbId(id) === TableCode.DOCUMENT_ARCHIVED_NODES),
});

export async function loadExistingDocumentAssetIds({ imageIds, fileIds, embedIds, archivedIds }: GroupedAssetIds): Promise<string[]> {
  const [existingImageIds, existingFileIds, existingEmbedIds, existingArchivedIds] = await Promise.all([
    imageIds.length > 0
      ? db
          .select({ id: Images.id })
          .from(Images)
          .where(inArray(Images.id, imageIds))
          .then((rows) => rows.map(({ id }) => id))
      : [],
    fileIds.length > 0
      ? db
          .select({ id: Files.id })
          .from(Files)
          .where(inArray(Files.id, fileIds))
          .then((rows) => rows.map(({ id }) => id))
      : [],
    embedIds.length > 0
      ? db
          .select({ id: Embeds.id })
          .from(Embeds)
          .where(inArray(Embeds.id, embedIds))
          .then((rows) => rows.map(({ id }) => id))
      : [],
    archivedIds.length > 0
      ? db
          .select({ id: DocumentArchivedNodes.id })
          .from(DocumentArchivedNodes)
          .where(inArray(DocumentArchivedNodes.id, archivedIds))
          .then((rows) => rows.map(({ id }) => id))
      : [],
  ]);

  return [...existingImageIds, ...existingFileIds, ...existingEmbedIds, ...existingArchivedIds];
}
