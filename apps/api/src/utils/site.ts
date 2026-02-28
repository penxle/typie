import { and, eq } from 'drizzle-orm';
import { LoroDoc } from 'loro-crdt';
import {
  DocumentContents,
  Documents,
  DocumentVersionContributors,
  DocumentVersions,
  Entities,
  first,
  firstOrThrow,
  Folders,
  Sites,
} from '@/db';
import { EntityState, EntityType } from '@/enums';
import { compressZstd } from './compression';
import { generatePermalink, generateSlug } from './entity';
import { wasm } from './wasm';
import type { Transaction } from '@/db';

type CreateSiteParams = {
  userId: string;
  name: string;
  slug: string;
  logoId: string;
  tx: Transaction;
};

export const createSite = async ({ userId, name, slug, logoId, tx }: CreateSiteParams) => {
  const site = await tx
    .insert(Sites)
    .values({
      userId,
      slug,
      name,
      logoId,
    })
    .returning({
      id: Sites.id,
    })
    .then(firstOrThrow);

  const templateSite = await tx
    .select({
      id: Sites.id,
    })
    .from(Sites)
    .where(eq(Sites.id, 'S0TEMPLATE'))
    .then(first);

  if (templateSite) {
    const templateFolders = await tx
      .select({
        id: Folders.id,
        name: Folders.name,
        entity: {
          id: Entities.id,
          depth: Entities.depth,
          parentId: Entities.parentId,
          order: Entities.order,
        },
      })
      .from(Folders)
      .innerJoin(Entities, eq(Folders.entityId, Entities.id))
      .where(and(eq(Entities.siteId, templateSite.id), eq(Entities.state, EntityState.ACTIVE)))
      .orderBy(Entities.depth);

    const folderEntityIdMap = new Map<string, string>();

    for (const folder of templateFolders) {
      const entity = await tx
        .insert(Entities)
        .values({
          userId,
          siteId: site.id,
          slug: generateSlug(),
          permalink: generatePermalink(),
          type: EntityType.FOLDER,
          depth: folder.entity.depth,
          parentId: folder.entity.parentId ? folderEntityIdMap.get(folder.entity.parentId) : null,
          order: folder.entity.order,
        })
        .returning({
          id: Entities.id,
        })
        .then(firstOrThrow);

      folderEntityIdMap.set(folder.entity.id, entity.id);

      await tx.insert(Folders).values({
        entityId: entity.id,
        name: folder.name,
      });
    }

    const templateDocuments = await tx
      .select({
        title: Documents.title,
        subtitle: Documents.subtitle,
        content: {
          json: DocumentContents.json,
          text: DocumentContents.text,
          characterCount: DocumentContents.characterCount,
          blobSize: DocumentContents.blobSize,
          snapshot: DocumentContents.snapshot,
          version: DocumentContents.version,
        },
        entity: {
          depth: Entities.depth,
          parentId: Entities.parentId,
          order: Entities.order,
        },
      })
      .from(Documents)
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .innerJoin(DocumentContents, eq(Documents.id, DocumentContents.documentId))
      .where(and(eq(Entities.siteId, templateSite.id), eq(Entities.state, EntityState.ACTIVE)));

    for (const doc of templateDocuments) {
      const newEntity = await tx
        .insert(Entities)
        .values({
          userId,
          siteId: site.id,
          parentId: doc.entity.parentId ? folderEntityIdMap.get(doc.entity.parentId) : null,
          slug: generateSlug(),
          permalink: generatePermalink(),
          type: EntityType.DOCUMENT,
          order: doc.entity.order,
          depth: doc.entity.depth,
        })
        .returning({ id: Entities.id })
        .then(firstOrThrow);

      const newDocument = await tx
        .insert(Documents)
        .values({
          entityId: newEntity.id,
          title: doc.title,
          subtitle: doc.subtitle,
        })
        .returning({ id: Documents.id })
        .then(firstOrThrow);

      const json = await wasm.snapshotToJson(new Uint8Array(doc.content.snapshot));
      const freshSnapshot = await wasm.jsonToSnapshot(json);
      const freshDoc = new LoroDoc();
      freshDoc.import(freshSnapshot);
      const freshVersion = freshDoc.version().encode();

      await tx.insert(DocumentContents).values({
        documentId: newDocument.id,
        json,
        text: doc.content.text,
        characterCount: doc.content.characterCount,
        blobSize: doc.content.blobSize,
        snapshot: freshSnapshot,
        version: freshVersion,
      });

      const documentVersion = await tx
        .insert(DocumentVersions)
        .values({
          documentId: newDocument.id,
          version: await compressZstd(freshVersion),
        })
        .returning({ id: DocumentVersions.id })
        .then(firstOrThrow);

      await tx.insert(DocumentVersionContributors).values({
        versionId: documentVersion.id,
        userId,
      });
    }
  }
};
