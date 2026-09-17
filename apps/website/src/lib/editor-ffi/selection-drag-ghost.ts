import { css } from '@typie/styled-system/css';
import { flushSync, mount, unmount } from 'svelte';
import SelectionDragGhost from './components/SelectionDragGhost.svelte';
import type { DragGhost } from '@typie/editor-ffi/browser';

let cleanupGhost: (() => void) | undefined;

export const clearSelectionDragGhost = (): void => cleanupGhost?.();

export const setSelectionDragGhost = (transfer: DataTransfer, ghost: DragGhost): void => {
  clearSelectionDragGhost();
  const host = document.createElement('div');
  host.className = css({ position: 'fixed', top: '0', left: '0', pointerEvents: 'none', userSelect: 'none', zIndex: 'ghost' });
  host.setAttribute('aria-hidden', 'true');
  let component: ReturnType<typeof mount> | undefined;
  let frame: number | undefined;
  const cleanup = () => {
    if (frame !== undefined) cancelAnimationFrame(frame);
    if (component) void unmount(component);
    component = undefined;
    host.remove();
    if (cleanupGhost === cleanup) cleanupGhost = undefined;
  };
  cleanupGhost = cleanup;
  try {
    document.body.append(host);
    component = mount(SelectionDragGhost, { target: host, props: { ghost } });
    flushSync();
    const badge = host.querySelector<HTMLElement>('[data-selection-drag-ghost]');
    if (!badge) {
      cleanup();
      return;
    }
    if (badge.getBoundingClientRect().height > 32) badge.style.borderRadius = '12px';
    transfer.setDragImage(badge, -8, -12);
    frame = requestAnimationFrame(cleanup);
  } catch {
    // A ghost failure must not cancel a valid native drag or its payload.
    cleanup();
  }
};
