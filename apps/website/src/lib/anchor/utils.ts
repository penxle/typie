import { clamp } from '$lib/utils';

export const getLastNodeOffsetTop = (): number | null => {
  const editorEl = document.querySelector('.editor');
  if (!editorEl) return null;

  const allNodes = [...editorEl.querySelectorAll('[data-node-id]')];
  if (allNodes.length === 0) return null;

  const lastNode = allNodes.at(-1) as HTMLElement;
  return lastNode.offsetTop;
};

export const getAnchorElements = (anchorIds: string[]): Record<string, HTMLElement> => {
  const elements: Record<string, HTMLElement> = {};

  for (const nodeId of anchorIds) {
    const element = document.querySelector(`[data-node-id="${nodeId}"]`);
    if (element) {
      elements[nodeId] = element as HTMLElement;
    }
  }

  return elements;
};

export type AnchorPosition = {
  nodeId: string;
  element: HTMLElement;
  position: number;
  name: string;
};

export const calculateAnchorPositions = (
  anchorElements: Record<string, HTMLElement>,
  anchors: Record<string, string | null>,
): AnchorPosition[] => {
  const lastNodeOffsetTop = getLastNodeOffsetTop();
  if (lastNodeOffsetTop === null) return [];

  return Object.entries(anchorElements)
    .map(([nodeId, element]) => {
      const offsetTop = element.offsetTop;
      const position = lastNodeOffsetTop > 0 ? clamp(offsetTop / lastNodeOffsetTop, 0, 1) : 0;

      return {
        nodeId,
        element,
        position,
        name:
          anchors[nodeId] ||
          (element.textContent
            ? element.textContent.length > 20
              ? element.textContent.slice(0, 20) + '...'
              : element.textContent
            : '(내용 없음)'),
      };
    })
    .sort((a, b) => a.position - b.position);
};
