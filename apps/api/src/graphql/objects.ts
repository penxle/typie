import { asc, inArray } from 'drizzle-orm';
import stringify from 'fast-json-stable-stringify';
import { db, decodeDbId } from '@/db';
import * as T from '@/db/schemas/tables';
import { builder } from './builder';
import type { DataLoaderOptions } from '@pothos/plugin-dataloader';
import type { AnyPgColumn, AnyPgTable, PgTable, TableConfig } from 'drizzle-orm/pg-core';
import type { PlanRules } from '@/db/schemas/json';
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
    cacheKeyFn: (key) => stringify(key),
  },
});

const createObjectRef = <T extends TableConfig>(name: string, table: TableWithIdColumn<T>) => {
  return builder.loadableObjectRef(name, {
    ...makeLoadableFields(table),
  });
};

const createInterfaceRef = <T extends TableConfig>(name: string, table: TableWithIdColumn<T>) => {
  return builder.loadableInterfaceRef(name, {
    ...makeLoadableFields(table),
  });
};

export const isTypeOf = (tableCode: string) => (self: unknown) => {
  return decodeDbId((self as { id: string }).id) === tableCode;
};

export const IEntity = createInterfaceRef('IEntity', T.Entities);
export const ICanvas = createInterfaceRef('ICanvas', T.Canvases);
export const IFolder = createInterfaceRef('IFolder', T.Folders);
export const IPost = createInterfaceRef('IPost', T.Posts);
export const ISite = createInterfaceRef('ISite', T.Sites);

export const Canvas = createObjectRef('Canvas', T.Canvases);
export const CanvasSnapshot = createObjectRef('CanvasSnapshot', T.CanvasSnapshots);
export const Comment = createObjectRef('Comment', T.Comments);
export const CreditCode = createObjectRef('CreditCode', T.CreditCodes);
export const Embed = createObjectRef('Embed', T.Embeds);
export const Entity = createObjectRef('Entity', T.Entities);
export const File = createObjectRef('File', T.Files);
export const Folder = createObjectRef('Folder', T.Folders);
export const Font = createObjectRef('Font', T.Fonts);
export const FontFamily = createObjectRef('FontFamily', T.FontFamilies);
export const Image = createObjectRef('Image', T.Images);
export const Notification = createObjectRef('Notification', T.Notifications);
export const PaymentInvoice = createObjectRef('PaymentInvoice', T.PaymentInvoices);
export const Plan = createObjectRef('Plan', T.Plans);
export const Post = createObjectRef('Post', T.Posts);
export const PostReaction = createObjectRef('PostReaction', T.PostReactions);
export const PostSnapshot = createObjectRef('PostSnapshot', T.PostSnapshots);
export const Referral = createObjectRef('Referral', T.Referrals);
export const Site = createObjectRef('Site', T.Sites);
export const Subscription = createObjectRef('Subscription_', T.Subscriptions);
export const User = createObjectRef('User', T.Users);
export const UserBillingKey = createObjectRef('UserBillingKey', T.UserBillingKeys);
export const UserPersonalIdentity = createObjectRef('UserPersonalIdentity', T.UserPersonalIdentities);
export const UserSingleSignOn = createObjectRef('UserSingleSignOn', T.UserSingleSignOns);

export const CanvasView = createObjectRef('CanvasView', T.Canvases);
export const EntityView = createObjectRef('EntityView', T.Entities);
export const FolderView = createObjectRef('FolderView', T.Folders);
export const PostView = createObjectRef('PostView', T.Posts);
export const SiteView = createObjectRef('SiteView', T.Sites);

type BlobShape = { id: string; size: number; path: string };
export const Blob = builder.interfaceRef<BlobShape>('Blob');

export const EntityNode = builder.unionType('EntityNode', {
  types: [Canvas, Folder, Post],
});

export const EntityViewNode = builder.unionType('EntityViewNode', {
  types: [CanvasView, FolderView, PostView],
});

export const CharacterCountChange = builder.simpleObject('CharacterCountChange', {
  fields: (t) => ({
    date: t.field({ type: 'DateTime' }),
    additions: t.int(),
    deletions: t.int(),
  }),
});

export const PlanRule = builder.objectRef<Partial<PlanRules>>('PlanRule');
