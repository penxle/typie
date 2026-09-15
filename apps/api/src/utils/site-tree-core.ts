import { EntityState, EntityType, EntityVisibility } from '@typie/lib/enums';
import { and, inArray, ne } from 'drizzle-orm';
import { Entities } from '#/db/schemas/tables.ts';
import type { Database, Transaction } from '#/db/index.ts';

type Executor = Database | Transaction;

export type SiteTreeRow = {
  id: string;
  siteId: string;
  parentId: string | null;
  type: EntityType;
  visibility: EntityVisibility;
  order: string;
};

export type SiteTree = {
  byId: Map<string, SiteTreeRow>;
  childrenOf: Map<string | null, SiteTreeRow[]>;
  visible: Set<string>;
  pathChildrenOf: Map<string | null, SiteTreeRow[]>;
};

export const buildSiteTreeRowsQuery = (executor: Executor, input: { siteIds: string[] }) =>
  executor
    .select({
      id: Entities.id,
      siteId: Entities.siteId,
      parentId: Entities.parentId,
      type: Entities.type,
      visibility: Entities.visibility,
      order: Entities.order,
    })
    .from(Entities)
    .where(
      and(inArray(Entities.siteId, input.siteIds), inArray(Entities.state, [EntityState.ACTIVE]), ne(Entities.type, EntityType.DIVIDER)),
    );

export const buildSiteTree = (rows: readonly SiteTreeRow[]): SiteTree => {
  const byId = new Map(rows.map((row) => [row.id, row]));
  const childrenOf = new Map<string | null, SiteTreeRow[]>();
  for (const row of rows) {
    const parentId = row.parentId !== null && byId.has(row.parentId) ? row.parentId : null;
    const siblings = childrenOf.get(parentId) ?? [];
    siblings.push(row);
    childrenOf.set(parentId, siblings);
  }
  for (const siblings of childrenOf.values()) {
    siblings.sort((a, b) => a.order.localeCompare(b.order));
  }

  const visible = new Set<string>();
  const mark = (id: string): boolean => {
    const row = byId.get(id);
    if (!row) return false;
    if (row.type === EntityType.DOCUMENT) {
      const shown = row.visibility === EntityVisibility.PUBLIC;
      if (shown) visible.add(id);
      return shown;
    }
    let shown = false;
    for (const child of childrenOf.get(id) ?? []) {
      if (mark(child.id)) shown = true;
    }
    if (shown) visible.add(id);
    return shown;
  };
  for (const root of childrenOf.get(null) ?? []) mark(root.id);

  const isPathStep = (row: SiteTreeRow) =>
    row.type === EntityType.FOLDER && visible.has(row.id) && row.visibility === EntityVisibility.PUBLIC;

  const collect = (parentId: string | null): SiteTreeRow[] => {
    const out: SiteTreeRow[] = [];
    for (const child of childrenOf.get(parentId) ?? []) {
      if (!visible.has(child.id)) continue;
      if (child.type === EntityType.DOCUMENT || isPathStep(child)) out.push(child);
      else out.push(...collect(child.id));
    }
    return out;
  };

  const pathChildrenOf = new Map<string | null, SiteTreeRow[]>();
  const fill = (parentId: string | null) => {
    const children = collect(parentId);
    pathChildrenOf.set(parentId, children);
    for (const child of children) {
      if (child.type === EntityType.FOLDER) fill(child.id);
    }
  };
  fill(null);

  return { byId, childrenOf, visible, pathChildrenOf };
};

export const pathChildren = (tree: SiteTree, parentId: string | null): SiteTreeRow[] => tree.pathChildrenOf.get(parentId) ?? [];

export const isPathFolder = (tree: SiteTree, id: string): boolean => tree.pathChildrenOf.has(id);

export const pathAncestors = (tree: SiteTree, id: string): SiteTreeRow[] => {
  const chain: SiteTreeRow[] = [];
  let current = tree.byId.get(id)?.parentId ?? null;
  while (current !== null) {
    const row = tree.byId.get(current);
    if (!row) break;
    if (isPathFolder(tree, row.id)) chain.unshift(row);
    current = row.parentId;
  }
  return chain;
};

export const pathNeighbors = (tree: SiteTree, id: string): { prev: SiteTreeRow | null; next: SiteTreeRow | null } => {
  if (!tree.byId.has(id)) return { prev: null, next: null };
  const parentId = pathAncestors(tree, id).at(-1)?.id ?? null;
  const siblings = pathChildren(tree, parentId).filter((sibling) => sibling.type === EntityType.DOCUMENT);
  const index = siblings.findIndex((sibling) => sibling.id === id);
  if (index === -1) return { prev: null, next: null };
  return { prev: siblings[index - 1] ?? null, next: siblings[index + 1] ?? null };
};

export const pathChildCounts = (tree: SiteTree, id: string): { folders: number; documents: number } => {
  let folders = 0;
  let documents = 0;
  for (const child of pathChildren(tree, id)) {
    if (child.type === EntityType.DOCUMENT) documents += 1;
    else folders += 1;
  }
  return { folders, documents };
};

export const pathFolderIds = (tree: SiteTree): string[] => [...tree.pathChildrenOf.keys()].filter((key): key is string => key !== null);

export const descendantDocumentEntityIds = (tree: SiteTree, ids: readonly string[]): string[] => {
  const out: string[] = [];
  const walk = (parentId: string) => {
    for (const child of tree.childrenOf.get(parentId) ?? []) {
      if (child.type === EntityType.DOCUMENT) out.push(child.id);
      else walk(child.id);
    }
  };
  for (const id of ids) walk(id);
  return out;
};
