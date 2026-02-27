import { nanoid } from 'nanoid';
import type { Member, Pane, PaneInit, PanePlacement, PaneSide } from './types';

export const collectPanes = (node: Member | null): Pane[] => {
  if (!node) return [];
  if (node.type === 'pane') return [node];
  return node.children.flatMap((child: Member) => collectPanes(child));
};

// 제거될 pane의 직계 부모 axis에서 물리적으로 가장 가까운 sibling pane을 반환
export const findAdjacentPane = (root: Member, paneId: string): Pane | null => {
  const walk = (node: Member): Pane | null => {
    if (node.type === 'pane') return null;

    for (let i = 0; i < node.children.length; i++) {
      const child = node.children[i];

      if (child.type === 'pane' && child.id === paneId) {
        const prev = node.children[i - 1];
        const next = node.children[i + 1];
        if (prev) {
          const panes = collectPanes(prev);
          return panes.at(-1) ?? null;
        }
        if (next) {
          return collectPanes(next)[0] ?? null;
        }
        return null;
      }

      if (child.type === 'axis') {
        const result = walk(child);
        if (result) return result;
      }
    }
    return null;
  };

  return walk(root);
};

export const findMemberById = (node: Member, id: string): Member | null => {
  if (node.id === id) return node;

  if (node.type === 'axis') {
    for (const child of node.children) {
      const found = findMemberById(child, id);
      if (found) return found;
    }
  }

  return null;
};

export const removePane = (node: Member, paneId: string): Member | null => {
  if (node.type === 'pane' && node.id === paneId) {
    return null;
  }

  if (node.type === 'axis') {
    const pairs: { child: Member; flex: number }[] = [];

    for (let i = 0; i < node.children.length; i++) {
      const result = removePane(node.children[i], paneId);
      if (result) {
        pairs.push({ child: result, flex: node.flexes[i] ?? 1 });
      }
    }

    return flattenMember({
      ...node,
      children: pairs.map((p) => p.child),
      flexes: pairs.map((p) => p.flex),
    });
  }

  return node;
};

export const replacePane = (node: Member, id: string, pane: PaneInit): { root: Member; newPaneId: string } => {
  const newPaneId = nanoid();
  const walk = (n: Member): Member => {
    if (n.type === 'pane') {
      return n.id === id ? ({ id: newPaneId, type: 'pane', ...pane } as Pane) : n;
    }
    let changed = false;
    const children = n.children.map((child) => {
      const next = walk(child);
      if (next !== child) changed = true;
      return next;
    });
    return changed ? { ...n, children } : n;
  };
  return { root: walk(node), newPaneId };
};

export const swapPanes = (root: Member, firstId: string, secondId: string): Member => {
  const first = findMemberById(root, firstId);
  const second = findMemberById(root, secondId);
  if (!first || first.type !== 'pane' || !second || second.type !== 'pane') return root;

  const walk = (node: Member): Member => {
    if (node.type === 'pane') {
      if (node.id === firstId) return second;
      if (node.id === secondId) return first;
      return node;
    }
    let changed = false;
    const children = node.children.map((child) => {
      const next = walk(child);
      if (next !== child) changed = true;
      return next;
    });
    return changed ? { ...node, children } : node;
  };

  return walk(root);
};

const moveOrAddPane = (
  root: Member,
  source: { paneId: string } | { pane: PaneInit },
  target: { paneId: string; side: PaneSide },
): { root: Member; focusedPaneId: string } | null => {
  const isMove = 'paneId' in source;
  const newPaneId = isMove ? source.paneId : nanoid();
  const { paneId: targetPaneId, side } = target;
  const axisDirection = side === 'left' || side === 'right' ? 'horizontal' : 'vertical';

  let sourcePaneId: string | null = null;
  let sourcePaneData: Pane | null = null;

  if (isMove) {
    sourcePaneId = source.paneId;
    const found = findMemberById(root, sourcePaneId);
    if (!found || found.type !== 'pane') return null;
    sourcePaneData = found;
  }

  const processNode = (node: Member): Member | null => {
    if (node.type === 'pane') {
      if (sourcePaneId && node.id === sourcePaneId) {
        return null;
      }

      // NOTE: 타겟 pane은 새 pane과 함께 axis로 변환
      if (node.id === targetPaneId) {
        const newPane =
          isMove && sourcePaneData
            ? ({ ...sourcePaneData, id: newPaneId } as Pane)
            : ({ type: 'pane', ...(source as { pane: PaneInit }).pane, id: newPaneId } as Pane);

        const children = side === 'left' || side === 'top' ? [newPane, node] : [node, newPane];

        return {
          id: nanoid(),
          type: 'axis',
          direction: axisDirection,
          children,
          flexes: [1, 1],
        };
      }

      return node;
    }

    const pairs: { child: Member; flex: number }[] = [];
    for (let i = 0; i < node.children.length; i++) {
      const result = processNode(node.children[i]);
      if (result) {
        pairs.push({ child: result, flex: node.flexes[i] ?? 1 });
      }
    }

    return {
      ...node,
      children: pairs.map((p) => p.child),
      flexes: pairs.map((p) => p.flex),
    };
  };

  const result = processNode(root);
  if (!result) return null;

  const flattened = flattenMember(result);
  if (!flattened) return null;

  return { root: flattened, focusedPaneId: newPaneId };
};

export const addPane = (root: Member, pane: PaneInit, placement: PanePlacement): { root: Member; focusedPaneId: string } | null =>
  moveOrAddPane(root, { pane }, placement);

export const movePane = (root: Member, paneId: string, placement: PanePlacement): { root: Member; focusedPaneId: string } | null =>
  moveOrAddPane(root, { paneId }, placement);

const flattenMember = (node: Member): Member | null => {
  if (node.type === 'pane') return node;

  const pairs: { child: Member; flex: number }[] = [];

  for (let i = 0; i < node.children.length; i++) {
    const flatChild = flattenMember(node.children[i]);
    if (!flatChild) continue;

    const childFlex = node.flexes[i] ?? 1;

    // NOTE: 방향이 같은 자식 axis를 부모 레벨로 병합 (flex 비례 보존)
    if (flatChild.type === 'axis' && flatChild.direction === node.direction) {
      const childTotalFlex = flatChild.flexes.reduce((s, f) => s + f, 0) || 1;
      for (let j = 0; j < flatChild.children.length; j++) {
        pairs.push({
          child: flatChild.children[j],
          flex: childFlex * ((flatChild.flexes[j] ?? 1) / childTotalFlex),
        });
      }
    } else {
      pairs.push({ child: flatChild, flex: childFlex });
    }
  }

  if (pairs.length === 0) return null;
  if (pairs.length === 1) return pairs[0].child;

  return {
    ...node,
    children: pairs.map((p) => p.child),
    flexes: pairs.map((p) => p.flex),
  };
};
