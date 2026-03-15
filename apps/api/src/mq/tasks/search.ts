import { eq } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities, firstOrThrow, Folders } from '#/db/index.ts';
import { EntityState } from '#/enums.ts';
import { elasticsearch, esIndex } from '#/search.ts';
import { getAncestorEntityIds } from '#/utils/entity.ts';
import { decompose } from '#/utils/text.ts';
import { defineJob } from '../types.ts';

export const DocumentIndexJob = defineJob('search:index:document', async (documentId: string) => {
  const document = await db
    .select({
      id: Documents.id,
      state: Entities.state,
      siteId: Entities.siteId,
      entityId: Entities.id,
      parentId: Entities.parentId,
      title: Documents.title,
      subtitle: Documents.subtitle,
      text: DocumentContents.text,
      updatedAt: Documents.updatedAt,
    })
    .from(Documents)
    .innerJoin(DocumentContents, eq(Documents.id, DocumentContents.documentId))
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(eq(Documents.id, documentId))
    .then(firstOrThrow);

  if (document.state === EntityState.ACTIVE) {
    const ancestorIds = await getAncestorEntityIds(document.entityId);

    await elasticsearch.index({
      index: esIndex.documents,
      id: document.id,
      document: {
        site_id: document.siteId,
        title: document.title,
        title_decomposed: decompose(document.title),
        subtitle: document.subtitle,
        subtitle_decomposed: decompose(document.subtitle),
        text: document.text,
        ancestor_ids: ancestorIds,
        updated_at: document.updatedAt,
      },
    });
  } else {
    await elasticsearch.delete({ index: esIndex.documents, id: document.id }, { ignore: [404] });
  }
});

export const FolderIndexJob = defineJob('search:index:folder', async (folderId: string) => {
  const folder = await db
    .select({
      id: Folders.id,
      state: Entities.state,
      siteId: Entities.siteId,
      entityId: Entities.id,
      parentId: Entities.parentId,
      name: Folders.name,
      createdAt: Folders.createdAt,
    })
    .from(Folders)
    .innerJoin(Entities, eq(Folders.entityId, Entities.id))
    .where(eq(Folders.id, folderId))
    .then(firstOrThrow);

  if (folder.state === EntityState.ACTIVE) {
    const ancestorIds = await getAncestorEntityIds(folder.entityId);

    await elasticsearch.index({
      index: esIndex.folders,
      id: folder.id,
      document: {
        site_id: folder.siteId,
        name: folder.name,
        name_decomposed: decompose(folder.name),
        ancestor_ids: ancestorIds,
        updated_at: folder.createdAt,
      },
    });
  } else {
    await elasticsearch.delete({ index: esIndex.folders, id: folder.id }, { ignore: [404] });
  }
});
