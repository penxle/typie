import { FontFamilySource, FontFamilyState, FontState } from '@typie/lib/enums';
import { asc, inArray } from 'drizzle-orm';
import stringify from 'fast-json-stable-stringify';
import { db, decodeDbId } from '#/db/index.ts';
import * as T from '#/db/schemas/tables.ts';
import { builder } from './builder.ts';
import type { DataLoaderOptions } from '@pothos/plugin-dataloader';
import type { AnyPgColumn, AnyPgTable, PgTable, TableConfig } from 'drizzle-orm/pg-core';
import type { PlanRules } from '#/db/schemas/json.ts';
import type { Builder } from './builder.ts';

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
  const id = typeof self === 'string' ? self : (self as { id: string }).id;
  return decodeDbId(id) === tableCode;
};

export const IEntity = createInterfaceRef('IEntity', T.Entities);
export const IDocument = createInterfaceRef('IDocument', T.Documents);
export const IFolder = createInterfaceRef('IFolder', T.Folders);
export const IPost = createInterfaceRef('IPost', T.Posts);
export const ISite = createInterfaceRef('ISite', T.Sites);
export const IUser = createInterfaceRef('IUser', T.Users);

export const CreditCode = createObjectRef('CreditCode', T.CreditCodes);
export const Document = createObjectRef('Document', T.Documents);
export const DocumentArchivedNode = createObjectRef('DocumentArchivedNode', T.DocumentArchivedNodes);
export const DocumentState = createObjectRef('DocumentState', T.DocumentStates);
export const Embed = createObjectRef('Embed', T.Embeds);
export const Entity = createObjectRef('Entity', T.Entities);
export const File = createObjectRef('File', T.Files);
export const Folder = createObjectRef('Folder', T.Folders);
export const Font = createObjectRef('Font', T.Fonts);
export const FontFamily = createObjectRef('FontFamily', T.FontFamilies);
export const Image = createObjectRef('Image', T.Images);
export const Note = createObjectRef('Note', T.Notes);
export const PaymentInvoice = createObjectRef('PaymentInvoice', T.PaymentInvoices);
export const PaymentRecord = createObjectRef('PaymentRecord', T.PaymentRecords);
export const Plan = createObjectRef('Plan', T.Plans);
export const Post = createObjectRef('Post', T.Posts);
export const DocumentReaction = createObjectRef('DocumentReaction', T.DocumentReactions);
export const PostReaction = createObjectRef('PostReaction', T.PostReactions);
export const PostSnapshot = createObjectRef('PostSnapshot', T.PostSnapshots);
export const DocumentVersion = createObjectRef('DocumentVersion', T.DocumentVersions);
export const Redirect = createObjectRef('Redirect', T.Redirects);
export const Referral = createObjectRef('Referral', T.Referrals);
export const Site = createObjectRef('Site', T.Sites);
export const Subscription = createObjectRef('Subscription_', T.Subscriptions);
export const TextReplacement = createObjectRef('TextReplacement', T.TextReplacements);
export const TextReplacementPreference = createObjectRef('TextReplacementPreference', T.TextReplacementPreferences);
export const User = createObjectRef('User', T.Users);
export const UserBillingKey = createObjectRef('UserBillingKey', T.UserBillingKeys);
export const UserDevice = createObjectRef('UserDevice', T.UserDevices);
export const UserPersonalIdentity = createObjectRef('UserPersonalIdentity', T.UserPersonalIdentities);
export const UserSingleSignOn = createObjectRef('UserSingleSignOn', T.UserSingleSignOns);
export const UserTrial = createObjectRef('UserTrial', T.UserTrials);
export const Widget = createObjectRef('Widget', T.Widgets);

export const DocumentView = createObjectRef('DocumentView', T.Documents);
export const EntityView = createObjectRef('EntityView', T.Entities);
export const FolderView = createObjectRef('FolderView', T.Folders);
export const PostView = createObjectRef('PostView', T.Posts);
export const SiteView = createObjectRef('SiteView', T.Sites);
export const UserView = createObjectRef('UserView', T.Users);

type BlobShape = { id: string; size: number; path: string };
export const Blob = builder.interfaceRef<BlobShape>('Blob');

export const EntityContainer = builder.unionType('EntityContainer', {
  types: [Site, Entity],
});

export const EntityNode = builder.unionType('EntityNode', {
  types: [Document, Folder, Post],
});

export const EntityViewNode = builder.unionType('EntityViewNode', {
  types: [DocumentView, FolderView, PostView],
});

export const CharacterCountChange = builder.simpleObject('CharacterCountChange', {
  fields: (t) => ({
    date: t.field({ type: 'DateTime' }),
    additions: t.int(),
    deletions: t.int(),
  }),
});

export const PlanRule = builder.objectRef<Partial<PlanRules>>('PlanRule');

export const DocumentFont = builder.simpleObject('DocumentFont', {
  fields: (t) => ({
    id: t.id(),
    weight: t.int(),
    subfamilyDisplayName: t.string({ nullable: true }),
    url: t.string(),
    state: t.field({ type: FontState }),
    path: t.string(),
    hash: t.string(),
    chunks: t.field({ type: 'JSON' }),
  }),
});

export const DocumentFontFamily = builder.simpleObject('DocumentFontFamily', {
  fields: (t) => ({
    id: t.id(),
    displayName: t.string(),
    familyName: t.string(),
    source: t.field({ type: FontFamilySource }),
    state: t.field({ type: FontFamilyState }),
    fonts: t.field({ type: [DocumentFont] }),
  }),
});
