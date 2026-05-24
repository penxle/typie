import type { EditorEventHandler } from '../types';

export const handleContextMenu: EditorEventHandler<HTMLElement, MouseEvent> = (editor, e) => {
  if (editor.gesture.shouldSuppressNativeContextMenu()) {
    e.preventDefault();
    return;
  }

  const local = editor.clientToLocal(e.clientX, e.clientY);
  const hit = local ? editor.interactiveHitTest(local.page, local.x, local.y) : undefined;
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
