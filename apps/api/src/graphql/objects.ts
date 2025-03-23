import { asc, inArray } from 'drizzle-orm';
import { db } from '@/db';
import * as T from '@/db/schemas/tables';
import { builder } from './builder';
import type { DataLoaderOptions } from '@pothos/plugin-dataloader';
import type { AnyPgColumn, AnyPgTable, PgTable, TableConfig } from 'drizzle-orm/pg-core';
import type { Builder } from './builder';

type IdColumn = AnyPgColumn<{ data: string; notNull: true }>;
type TableWithIdColumn<T extends TableConfig> = AnyPgTable<{ columns: { id: IdColumn } }> & {
  id: IdColumn;
} & PgTable<T>;

type SchemaTypes = Builder extends PothosSchemaTypes.SchemaBuilder<infer T> ? T : never;

const makeLoadableFields = <T extends TableConfig>(
  table: TableWithIdColumn<T>,
): DataLoaderOptions<SchemaTypes, typeof table.$inferSelect, string, string, typeof table.$inferSelect> => ({
  load: (ids) => db.select().from(table).where(inArray(table.id, ids)).orderBy(asc(table.id)),
  toKey: (parent) => parent.id,
  sort: true,
  cacheResolved: true,
  loaderOptions: {
    cache: false,
  },
});

const createObjectRef = <T extends TableConfig>(name: string, table: TableWithIdColumn<T>) => {
  return builder.loadableObjectRef(name, {
    ...makeLoadableFields(table),
  });
};

// const createInterfaceRef = <T extends TableConfig>(name: string, table: TableWithIdColumn<T>) => {
//   return builder.loadableInterfaceRef(name, {
//     ...makeLoadableFields(table),
//   });
// };

export const Embed = createObjectRef('Embed', T.Embeds);
export const File = createObjectRef('File', T.Files);
export const Image = createObjectRef('Image', T.Images);
export const PreorderPayment = createObjectRef('PreorderPayment', T.PreorderPayments);
export const PreorderUser = createObjectRef('PreorderUser', T.PreorderUsers);
export const User = createObjectRef('User', T.Users);

type BlobShape = { id: string; path: string };
export const Blob = builder.interfaceRef<BlobShape>('Blob');
