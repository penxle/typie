import { NodeSelection, Selection, TextSelection } from '@tiptap/pm/state';
import type { EditorState } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';

function moveCaretBetweenBlocks(state: EditorState, dir: number) {
  const { $anchor, $head } = state.selection;
  const $side = dir > 0 ? $anchor.max($head) : $anchor.min($head);
  const $start = $side.parent.inlineContent ? ($side.depth ? state.doc.resolve(dir > 0 ? $side.after() : $side.before()) : null) : $side;
  return $start && Selection.findFrom($start, dir);
}

function applyCaretSelection(view: EditorView, sel: Selection) {
  view.dispatch(view.state.tr.setSelection(sel));
  return true;
}

function skipNonDraggableNodes(view: EditorView, dir: number) {
  const sel = view.state.selection;
  const $head = sel instanceof TextSelection ? sel.$head : sel.$from;
  const skip = dir > 0 ? $head.nodeAfter : $head.nodeBefore;

  if (skip && skip.type.spec.draggable === false) {
    const pos = dir > 0 ? $head.pos + skip.nodeSize : $head.pos - skip.nodeSize;
    try {
      return applyCaretSelection(view, TextSelection.create(view.state.doc, pos));
    } catch {
      return false;
    }
  }
  return false;
}

function moveCaretInText(view: EditorView, dir: number): boolean {
  const { selection, doc } = view.state;

  if (!(selection instanceof TextSelection) || !selection.empty) return false;

  const targetPos = (dir < 0 ? selection.from : selection.to) + dir;

  if (targetPos < 0 || targetPos > doc.content.size) return false;

  try {
    const resolvedPos = doc.resolve(targetPos);

    if (resolvedPos.parent.inlineContent) {
      view.dispatch(view.state.tr.setSelection(TextSelection.create(doc, targetPos)));
      return true;
    } else {
      view.dispatch(view.state.tr.setSelection(Selection.near(resolvedPos, dir)));
      return true;
    }
  } catch {
    try {
      view.dispatch(view.state.tr.setSelection(Selection.near(doc.resolve(targetPos), dir)));
      return true;
    } catch {
      return false;
    }
  }
}

function moveCaretHorizontally(view: EditorView, dir: number): boolean {
  const sel = view.state.selection;

  if (sel instanceof TextSelection) {
    if (!sel.empty) return false;

    if (view.endOfTextblock(dir > 0 ? 'forward' : 'backward')) {
      const next = moveCaretBetweenBlocks(view.state, dir);
      if (next && next instanceof NodeSelection) {
        return applyCaretSelection(view, next);
      }
      return false;
    } else {
      const $head = sel.$head;
      const node = $head.textOffset ? null : dir < 0 ? $head.nodeBefore : $head.nodeAfter;

      if (!node || node.isText) return false;

      const nodePos = dir < 0 ? $head.pos - node.nodeSize : $head.pos;

      if (node.isAtom || !node.isLeaf) {
        if (NodeSelection.isSelectable?.(node)) {
          const resolvedPos = dir < 0 ? view.state.doc.resolve($head.pos - node.nodeSize) : $head;
          return applyCaretSelection(view, new NodeSelection(resolvedPos));
        }
      } else if (typeof navigator !== 'undefined' && /WebKit/.test(navigator.userAgent)) {
        const targetPos = dir < 0 ? nodePos : nodePos + node.nodeSize;
        try {
          return applyCaretSelection(view, TextSelection.create(view.state.doc, targetPos));
        } catch {
          return false;
        }
      }
      return false;
    }
  } else if (sel instanceof NodeSelection && sel.node.isInline) {
    return applyCaretSelection(view, TextSelection.create(view.state.doc, dir > 0 ? sel.$to.pos : sel.$from.pos));
  } else {
    const next = moveCaretBetweenBlocks(view.state, dir);
    return next ? applyCaretSelection(view, next) : false;
  }
}

export function handleCaretMovement(view: EditorView, dir: number): boolean {
  return moveCaretHorizontally(view, dir) || skipNonDraggableNodes(view, dir) || moveCaretInText(view, dir);
}
