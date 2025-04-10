import type { EditorState } from '@tiptap/pm/state';

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
