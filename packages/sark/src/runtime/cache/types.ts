type Arrayable<T> = T | T[];

export const RootFieldKey: unique symbol = Symbol('ROOT');
export const EntityLinkKey: unique symbol = Symbol('LINK');

export type Typename = string;
export type ID = string;

export type KeyableEntity = {
  [key: string]: unknown;
  __typename?: Typename;
  id: ID;
};

export type EntityKey = `${Typename}:${ID}`;
export type EntityLink = { [EntityLinkKey]: EntityKey };

export type StorageKey = EntityKey | typeof RootFieldKey;
export type Storage = Record<StorageKey, Fields>;

export type FieldKey = string | `${string}$${string}`;
export type Scalar = string | number | boolean | null | undefined;
export type FieldValue = Arrayable<Scalar | EntityLink | { [key: FieldKey]: FieldValue }>;
export type Fields = Record<FieldKey, FieldValue>;

export type Data = { [key: string]: Arrayable<Scalar | Data> };
export type Variables = Record<string, unknown>;

export type QueryKey = `${string}$${string}`;

export type DependencyKey = `${EntityKey}:${FieldKey}` | `${EntityKey}:*`;
