export const maxDepth = 100;

const HIDDEN_TREE = '[role="tree"][aria-hidden="true"]';
const VISIBLE_TREE = '[role="tree"]:not([aria-hidden="true"])';

const insideHiddenTree = (el: Element): boolean => el.closest(HIDDEN_TREE) !== null;

const flatLevel = (tree: HTMLElement, selector: string) =>
  [...tree.querySelectorAll<HTMLElement>(selector)].filter((el) => el.closest(VISIBLE_TREE) === tree && !insideHiddenTree(el));

export const getNextElement = (root: HTMLElement, current: HTMLElement, selector: string) => {
  if (!root.contains(current) || insideHiddenTree(current)) {
    return null;
  }

  const nestedTree = current.querySelector<HTMLElement>(VISIBLE_TREE);
  if (nestedTree) {
    const firstInNested = flatLevel(nestedTree, selector)[0];
    if (firstInNested) {
      return firstInNested;
    }
  }

  const currentTree = (current.closest(VISIBLE_TREE) as HTMLElement | null) ?? root;
  let sibling = current.nextElementSibling as HTMLElement | null;

  while (sibling) {
    if (sibling.closest(VISIBLE_TREE) === currentTree && !insideHiddenTree(sibling) && sibling.matches(selector)) {
      return sibling;
    }
    sibling = sibling.nextElementSibling as HTMLElement | null;
  }

  return null;
};

export const getPreviousElement = (root: HTMLElement, current: HTMLElement, selector: string) => {
  if (!root.contains(current) || insideHiddenTree(current)) {
    return null;
  }

  const currentTree = (current.closest(VISIBLE_TREE) as HTMLElement | null) ?? root;
  let sibling = current.previousElementSibling as HTMLElement | null;

  while (sibling) {
    if (sibling.closest(VISIBLE_TREE) === currentTree && !insideHiddenTree(sibling) && sibling.matches(selector)) {
      return sibling;
    }
    sibling = sibling.previousElementSibling as HTMLElement | null;
  }

  return null;
};

export const resolveEntityTreeDropTarget = (root: HTMLElement, hitElement: Element | null): HTMLElement | null => {
  if (!hitElement) return null;

  const entity = hitElement.closest<HTMLElement>('[data-id]');
  if (entity && root.contains(entity)) {
    return entity;
  }

  const hitTree = hitElement.closest<HTMLElement>(VISIBLE_TREE);
  if (!hitTree || !root.contains(hitTree)) {
    return null;
  }

  return flatLevel(hitTree, '[data-id]').at(-1) ?? null;
};

export const getEntityTreeElement = () => document.querySelector<HTMLElement>('[data-entity-tree]') ?? undefined;

export const getNextSiblingOrder = (entityId: string, root = getEntityTreeElement()): string | undefined => {
  const el = root?.querySelector<HTMLElement>(`[data-id="${entityId}"]`);
  if (!el) return;

  let nextEl = el.nextElementSibling as HTMLElement | null;
  while (nextEl && !Object.hasOwn(nextEl.dataset, 'id')) {
    nextEl = nextEl.nextElementSibling as HTMLElement | null;
  }
  return nextEl?.dataset.order;
};
