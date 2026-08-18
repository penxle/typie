import { clipboard, Menu, shell } from 'electron';
import { CHROME_HEIGHT } from './window-manager';
import type { BaseWindow, MenuItemConstructorOptions } from 'electron';

export type ContextMenuRequest = { x: number; y: number; linkURL: string; selectionText: string; isEditable: boolean };

export const showContextMenu = (window: BaseWindow, request: ContextMenuRequest) => {
  const items: MenuItemConstructorOptions[] = [];

  let protocol: string | null;
  try {
    protocol = new URL(request.linkURL).protocol;
  } catch {
    protocol = null;
  }

  if (protocol === 'http:' || protocol === 'https:') {
    items.push(
      { label: '브라우저에서 열기', click: () => shell.openExternal(request.linkURL).catch(() => null) },
      { label: '링크 복사', click: () => clipboard.writeText(request.linkURL) },
      { type: 'separator' },
    );
  }

  if (request.isEditable) {
    items.push(
      { role: 'undo', label: '실행 취소' },
      { role: 'redo', label: '다시 실행' },
      { type: 'separator' },
      { role: 'cut', label: '잘라내기' },
    );
  }

  if (request.isEditable || request.selectionText) {
    items.push({ role: 'copy', label: '복사' });
  }

  if (request.isEditable) {
    items.push({ role: 'paste', label: '붙여넣기' }, { type: 'separator' }, { role: 'selectAll', label: '전체 선택' });
  }

  if (items.length === 0) return;

  Menu.buildFromTemplate(items).popup({ window, x: Math.round(request.x), y: Math.round(request.y + CHROME_HEIGHT) });
};
