import { TEXT_NODE_TYPES, WRAPPING_NODE_TYPES } from '../tiptap/extensions/node-commands';
import { clamp } from '../utils';
import type { Editor } from '@tiptap/core';
import type * as Y from 'yjs';

const LIST_NODE_TYPES = ['bullet_list', 'ordered_list'];
const ANCHORABLE_NODE_TYPES = new Set([...WRAPPING_NODE_TYPES, ...TEXT_NODE_TYPES, ...LIST_NODE_TYPES]);

const displayCache = new WeakMap<HTMLElement, string>();

export const getLastNodeOffsetTop = (editorEl: HTMLElement): number | null => {
  const allNodes = [...editorEl.querySelectorAll(`[data-node-id]`)];
  if (allNodes.length === 0) return null;

  // NOTE: inline 노드(특히 hard_break의 br)는 offsetTop이 0이거나 부정확한 값을 가질 수 있음
  const blockNodes = allNodes.filter((node) => {
    const element = node as HTMLElement;

    let display = displayCache.get(element);
    if (!display) {
      display = window.getComputedStyle(element).display;
      displayCache.set(element, display);
    }

    if (display === 'inline' || display === 'none') {
      return false;
    }

    return true;
  });

  if (blockNodes.length === 0) return null;

  const lastNode = blockNodes.at(-1) as HTMLElement;
  return lastNode.offsetTop;
};

export const getAnchorElements = (editor: Editor, anchorIds: string[]): Record<string, HTMLElement> => {
  const elements: Record<string, HTMLElement> = {};

  for (const nodeId of anchorIds) {
    const element = editor.view.dom.querySelector(`[data-node-id="${nodeId}"]`);
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
  name: string | null;
  excerpt: string;
};

export const calculateAnchorPositions = (
  editor: Editor,
  anchorElements: Record<string, HTMLElement>,
  anchors: Record<string, string | null>,
): AnchorPosition[] => {
  const lastNodeOffsetTop = getLastNodeOffsetTop(editor.view.dom);
  if (lastNodeOffsetTop === null) return [];

  return Object.entries(anchorElements)
    .map(([nodeId, element]) => {
      const offsetTop = element.offsetTop;
      const position = lastNodeOffsetTop > 0 ? clamp(offsetTop / lastNodeOffsetTop, 0, 1) : 0;

      return {
        nodeId,
        element,
        position,
        name: anchors[nodeId],
        excerpt: element.textContent
          ? element.textContent.length > 20
            ? element.textContent.slice(0, 20) + '...'
            : element.textContent
          : '(내용 없음)',
      };
    })
    .sort((a, b) => a.position - b.position);
};

export const findAnchorableNode = (editor: Editor, position?: number): { nodeId: string | null; pos: number | null } => {
  const pos = position ?? editor.state.selection.from;
  const resolvedPos = editor.state.doc.resolve(pos);

  let newPos = null;
  let nodeId = null;

  // 현재 위치의 노드부터 확인
  const currentNodeAtPos = editor.state.doc.nodeAt(pos);
  if (currentNodeAtPos) {
    const nodeType = currentNodeAtPos.type.name;
    if (ANCHORABLE_NODE_TYPES.has(nodeType)) {
      newPos = pos;
      nodeId = currentNodeAtPos.attrs.nodeId;
    }
  }

  // depth를 거슬러 올라가며 찾기
  if (newPos === null) {
    for (let depth = resolvedPos.depth; depth > 2; depth--) {
      const node = resolvedPos.node(depth);
      const nodeType = node.type.name;
      if (ANCHORABLE_NODE_TYPES.has(nodeType)) {
        newPos = resolvedPos.before(depth);
        nodeId = node.attrs.nodeId;
        break;
      }
    }
  }

  // depth 2 확인
  if (newPos === null) {
    newPos = resolvedPos.before(2);
    const node = editor.state.doc.nodeAt(newPos);
    if (node) {
      nodeId = node.attrs.nodeId;
    }
  }

  return { nodeId, pos: newPos };
};

// NOTE: 존재하지 않는 노드의 앵커를 제거
export const cleanOrphanAnchors = (editor: Editor, doc: Y.Doc): number => {
  const attrsMap = doc.getMap('attrs');
  const anchors = attrsMap.get('anchors') as Record<string, string | null> | undefined;

  if (!anchors || Object.keys(anchors).length === 0) {
    return 0;
  }

  const existingNodeIds = new Set<string>();

  editor.state.doc.descendants((node) => {
    if (node.attrs.nodeId) {
      existingNodeIds.add(node.attrs.nodeId);
    }
  });

  const orphanNodeIds: string[] = [];
  const cleanedAnchors: Record<string, string | null> = {};

  for (const [nodeId, name] of Object.entries(anchors)) {
    if (existingNodeIds.has(nodeId)) {
      cleanedAnchors[nodeId] = name;
    } else {
      orphanNodeIds.push(nodeId);
    }
  }

  if (orphanNodeIds.length > 0) {
    attrsMap.set('anchors', cleanedAnchors);
  }

  return orphanNodeIds.length;
};
