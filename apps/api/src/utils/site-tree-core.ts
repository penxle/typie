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

  return { byId, childrenOf, visible };
};

export const visibleChildren = (tree: SiteTree, parentId: string | null): SiteTreeRow[] =>
  (tree.childrenOf.get(parentId) ?? []).filter((row) => tree.visible.has(row.id));

export const visibleAncestors = (tree: SiteTree, id: string): SiteTreeRow[] => {
  const chain: SiteTreeRow[] = [];
  let current = tree.byId.get(id)?.parentId ?? null;
  while (current !== null) {
    const row = tree.byId.get(current);
    if (!row) break;
    chain.unshift(row);
    current = row.parentId;
  }
  return chain;
};

export const visibleNeighbors = (tree: SiteTree, id: string): { prev: SiteTreeRow | null; next: SiteTreeRow | null } => {
  const row = tree.byId.get(id);
  if (!row) return { prev: null, next: null };
  const siblings = visibleChildren(tree, row.parentId).filter((sibling) => sibling.type === EntityType.DOCUMENT);
  const index = siblings.findIndex((sibling) => sibling.id === id);
  if (index === -1) return { prev: null, next: null };
  return { prev: siblings[index - 1] ?? null, next: siblings[index + 1] ?? null };
};

export const visibleChildCounts = (tree: SiteTree, id: string): { folders: number; documents: number } => {
  let folders = 0;
  let documents = 0;
  for (const child of visibleChildren(tree, id)) {
    if (child.type === EntityType.DOCUMENT) documents += 1;
    else folders += 1;
  }
  return { folders, documents };
};

export const visibleFolderIds = (tree: SiteTree): string[] =>
  [...tree.visible].filter((id) => tree.byId.get(id)?.type === EntityType.FOLDER);

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
