import type { Member, Rect } from './types';

export const PANE_MIN_SIZE = 210;
const PANE_RESIZER_SIZE = 4;

export type ResizerInfo = {
  id: string;
  rect: Rect;
  direction: 'horizontal' | 'vertical';
  containerId: string;
  index: number;
  axisSize: number;
};

export function computeLayout(node: Member, bounds: Rect): { panes: Map<string, Rect>; resizers: ResizerInfo[] } {
  const panes = new Map<string, Rect>();
  const resizers: ResizerInfo[] = [];

  function recurse(n: Member, b: Rect) {
    if (n.type === 'pane') {
      panes.set(n.id, b);
      return;
    }

    const { direction, children, flexes } = n;
    if (children.length === 0) return;

    const isHorizontal = direction === 'horizontal';
    const resizerCount = Math.max(0, children.length - 1);
    const resizerTotal = resizerCount * PANE_RESIZER_SIZE;
    const fullAxisSize = isHorizontal ? b.width : b.height;
    const available = fullAxisSize - resizerTotal;

    if (available <= 0) return;

    const totalFlex = flexes.reduce((s: number, f: number) => s + f, 0) || 1;
    const sizes = flexes.map((f: number) => (f / totalFlex) * available);
    const minSizes = children.map((c: Member) => getMinSizeForMember(c, direction));
    for (let i = 0; i < sizes.length; i++) {
      sizes[i] = Math.max(sizes[i], minSizes[i]);
    }

    let offset = isHorizontal ? b.left : b.top;

    for (let i = 0; i < children.length; i++) {
      const childBounds: Rect = isHorizontal
        ? { left: offset, top: b.top, width: sizes[i], height: b.height }
        : { left: b.left, top: offset, width: b.width, height: sizes[i] };

      recurse(children[i], childBounds);
      offset += sizes[i];

      if (i < children.length - 1) {
        const resizerRect: Rect = isHorizontal
          ? { left: offset, top: b.top, width: PANE_RESIZER_SIZE, height: b.height }
          : { left: b.left, top: offset, width: b.width, height: PANE_RESIZER_SIZE };

        resizers.push({
          id: `${children[i].id}:${children[i + 1].id}`,
          rect: resizerRect,
          direction,
          containerId: n.id,
          index: i,
          axisSize: fullAxisSize,
        });

        offset += PANE_RESIZER_SIZE;
      }
    }
  }

  recurse(node, bounds);
  return { panes, resizers };
}

export const getMinSizeForMember = (node: Member, parentDirection: 'horizontal' | 'vertical'): number => {
  if (node.type === 'pane') {
    return PANE_MIN_SIZE;
  }

  // NOTE: 부모와 같은 방향: 자식들의 최소 크기 합산 + Resizer 크기들
  if (node.direction === parentDirection) {
    const childrenMinSize = node.children.reduce((sum: number, child: Member) => sum + getMinSizeForMember(child, parentDirection), 0);
    const resizerCount = Math.max(0, node.children.length - 1);
    return childrenMinSize + resizerCount * PANE_RESIZER_SIZE;
  }
  // NOTE: 부모와 다른 방향: 자식들 중 최대 크기
  return Math.max(...node.children.map((child: Member) => getMinSizeForMember(child, parentDirection)));
};
