import { eq } from 'drizzle-orm';
import { db, DocumentStates, first } from '#/db/index.ts';
import { builder } from '../builder.ts';
import { Document, DocumentState } from '../objects.ts';

/**
 * * Types
 */

DocumentState.implement({
  fields: (t) => ({
    json: t.expose('json', { type: 'JSON' }),
    text: t.exposeString('text'),
    characterCount: t.exposeInt('characterCount'),
    blobSize: t.field({ type: 'BigInt', resolve: (self) => String(self.blobSize) }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
  }),
});

builder.objectFields(Document, (t) => ({
  state: t.field({
    type: DocumentState,
    nullable: true,
    resolve: async (document) => db.select().from(DocumentStates).where(eq(DocumentStates.documentId, document.id)).then(first),
  }),
}));
