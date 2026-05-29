import type { EditorEventHandler } from '../types';

export const handleContextMenu: EditorEventHandler<HTMLElement, MouseEvent> = (editor, e) => {
  if (editor.gesture.shouldSuppressNativeContextMenu()) {
    e.preventDefault();
    return;
  }

  const local = editor.clientToLocal(e.clientX, e.clientY);
  const hit = local ? editor.interactiveHitTest(local.page, local.x, local.y) : undefined;
  if (local) {
    const keepSelection = !editor.isSelectionCollapsed && editor.selectionHitTest(local.page, local.x, local.y);
    if (!keepSelection) {
      editor.enqueue({
        type: 'selection',
        op: editor.readOnly
          ? { type: 'select_unit_at', page: local.page, x: local.x, y: local.y, unit: 'word' }
          : { type: 'set_at', page: local.page, x: local.x, y: local.y },
      });
    }
    editor.flush();
  }
  const extraItems = editor.collectContextMenuContributions({ hit, clientX: e.clientX, clientY: e.clientY });

  editor.openContextMenu({
    x: e.clientX,
    y: e.clientY,
    source: 'mouse',
    placement: 'bottom-start',
    extraItems,
  });
  e.preventDefault();
};
