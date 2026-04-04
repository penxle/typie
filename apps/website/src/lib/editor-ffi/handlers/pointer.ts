import type { EditorEventHandler } from '../types';

export const handlePointerDown: EditorEventHandler<HTMLElement, PointerEvent> = (editor, e) => {
  const rect = e.currentTarget.getBoundingClientRect();
  const global = { x: e.clientX - rect.x, y: e.clientY - rect.y };
  const local = editor.globalToLocal(global.x, global.y);
  if (!local) {
    return;
  }

  const { page, x, y } = local;

  editor.enqueue({ type: 'pointer', event: { type: 'down', page, x, y, count: e.detail } });
};
