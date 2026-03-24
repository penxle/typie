#!/usr/bin/env node

import { readFile } from 'node:fs/promises';
import { DocumentSyncType } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { eq, sql } from 'drizzle-orm';
import { LoroDoc } from 'loro-crdt';
import { db, DocumentContents, DocumentVersions, firstOrThrow } from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { compressZstd } from '#/utils/compression.ts';
import { extractLoroDocContents } from '#/utils/index.ts';
import { wasm } from '#/utils/wasm.ts';

if (!process.argv[2] || !process.argv[3]) {
  console.error('Usage: node scripts/overwrite-document.ts <documentId> <json-file>');
  process.exit(1);
}

const documentId = process.argv[2];
const jsonFilePath = process.argv[3];

const jsonContent = JSON.parse(await readFile(jsonFilePath, 'utf8'));

const freshSnapshot = await wasm.jsonToSnapshot(jsonContent);
const freshDoc = new LoroDoc();
freshDoc.import(freshSnapshot);
const freshVersion = freshDoc.version().encode();
const { json, text, characterCount, blobSize } = await extractLoroDocContents(freshDoc);

await db.select({ id: DocumentContents.id }).from(DocumentContents).where(eq(DocumentContents.documentId, documentId)).then(firstOrThrow);

const updatedContent = await db.transaction(async (tx) => {
  // DocumentVersionContributors는 ON DELETE CASCADE로 자동 삭제
  await tx.delete(DocumentVersions).where(eq(DocumentVersions.documentId, documentId));

  const content = await tx
    .update(DocumentContents)
    .set({
      json,
      text,
      characterCount,
      blobSize,
      snapshot: freshSnapshot,
      version: freshVersion,
      generation: sql`${DocumentContents.generation} + 1`,
      updatedAt: dayjs(),
    })
    .where(eq(DocumentContents.documentId, documentId))
    .returning({ generation: DocumentContents.generation })
    .then(firstOrThrow);

  await tx.insert(DocumentVersions).values({
    documentId,
    version: await compressZstd(freshVersion),
  });

  return content;
});

pubsub.publish('document:sync', documentId, {
  target: '*',
  type: DocumentSyncType.RESET,
  data: JSON.stringify({
    snapshot: freshSnapshot.toBase64(),
    version: freshVersion.toBase64(),
    generation: updatedContent.generation,
  }),
});

console.log(`Document ${documentId} overwritten (generation: ${updatedContent.generation})`);
process.exit(0);
