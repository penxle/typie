import { findChildren } from '@tiptap/core';
import type { EditorState } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';

export const getNodeView = (view: EditorView, pos: number) => {
  const node = view.nodeDOM(pos);
  if (!node) {
    return null;
  }

  return node.__nodeview__;
};

export const getNodeViewByNodeId = (view: EditorView, nodeId: string) => {
  const children = findChildren(view.state.doc, (node) => node.attrs.nodeId === nodeId);
  if (children.length === 0) {
    return null;
  }

  return getNodeView(view, children[0].pos);
};

export const isCodeActive = (state: EditorState) => {
  const { selection } = state;
  const { empty, from, to } = selection;

  let hasCodeMark = false;

  if (empty) {
    const marks = state.storedMarks || state.doc.nodeAt(from)?.marks || [];
    hasCodeMark = marks.some((mark) => mark.type.spec.code === true);
  } else {
    state.doc.nodesBetween(from, to, (node) => {
      if (node.marks && node.marks.some((mark) => mark.type.spec.code === true)) {
        hasCodeMark = true;
        return false;
      }
      return true;
    });
  }

  if (hasCodeMark) {
    return true;
  }

  let isInCodeBlock = false;

  state.doc.nodesBetween(selection.from, selection.to, (node) => {
    if (node.type.spec.code === true) {
      isInCodeBlock = true;
      return false;
    }

    return true;
  });

  return isInCodeBlock;
};
