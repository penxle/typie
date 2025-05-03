export const maxDepth = 3;

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
