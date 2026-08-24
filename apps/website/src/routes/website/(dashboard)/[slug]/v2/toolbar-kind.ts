export type ToolbarKind = 'insert' | 'format';

export const isToolbarKind = (value: unknown): value is ToolbarKind => value === 'insert' || value === 'format';

export const otherToolbarKind = (kind: ToolbarKind): ToolbarKind => (kind === 'insert' ? 'format' : 'insert');

export const primaryToolbarStorageKey = (documentId: string) => `typie:primary-toolbar:${documentId}`;

export const readPrimaryToolbar = (documentId: string, storage: Pick<Storage, 'getItem'> = localStorage): ToolbarKind | null => {
  const value = storage.getItem(primaryToolbarStorageKey(documentId));
  return isToolbarKind(value) ? value : null;
};

export const writePrimaryToolbar = (documentId: string, kind: ToolbarKind, storage: Pick<Storage, 'setItem'> = localStorage) => {
  storage.setItem(primaryToolbarStorageKey(documentId), kind);
};
