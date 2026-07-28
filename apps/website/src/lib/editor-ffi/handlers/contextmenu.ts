import { isSelectionCollapsed } from '../geometry';
import type { EditorEventHandler } from '../types';

export const handleContextMenu: EditorEventHandler<HTMLElement, MouseEvent> = (editor, e) => {
  if (editor.gesture.shouldSuppressNativeContextMenu()) {
    e.preventDefault();
    return;
  }
  e.preventDefault();

  const local = editor.clientToLocal(e.clientX, e.clientY);
  const hit = local ? editor.interactiveHitTest(local.page, local.x, local.y) : undefined;
  if (local) {
    const keepSelection = !isSelectionCollapsed(editor.appliedSnapshot.selection) && editor.selectionHitTest(local.page, local.x, local.y);
    editor.updateNow(() => {
      if (!keepSelection) {
        editor.enqueue({
          type: 'selection',
          op: editor.readOnly
            ? { type: 'select_unit_at', page: local.page, x: local.x, y: local.y, unit: 'word' }
            : { type: 'set_at', page: local.page, x: local.x, y: local.y },
        });
      }
    });
  }
  const extraItems = editor.collectContextMenuContributions({ hit, clientX: e.clientX, clientY: e.clientY });

  editor.openContextMenu({
    x: e.clientX,
    y: e.clientY,
    source: 'mouse',
    placement: 'bottom-start',
    extraItems,
  });
};
