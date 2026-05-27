import type { EditorEventHandler } from '../types';

export const handleContextMenu: EditorEventHandler<HTMLElement, MouseEvent> = (editor, e) => {
  if (editor.gesture.shouldSuppressNativeContextMenu()) {
    e.preventDefault();
    return;
  }

  const local = editor.clientToLocal(e.clientX, e.clientY);
  const hit = local ? editor.interactiveHitTest(local.page, local.x, local.y) : undefined;
  if (local) {
    editor.enqueue({
      type: 'pointer',
      event: {
        type: 'secondary_down',
        page: local.page,
        x: local.x,
        y: local.y,
      },
    });
    if (editor.readOnly) {
      // TODO: 엔진이 read only 상태를 알게 되면 이건 엔진 쪽 handle_pointer_down에서 처리하는 게 좋을듯
      editor.enqueue({ type: 'selection', op: { type: 'expand', unit: 'word' } });
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
