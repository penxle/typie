import type { ActionReturn } from 'svelte/action';
import type { PaneChromeAttachmentHandle } from './zen-mode-pane-chrome.svelte';

export const paneChromeAttachment = (node: HTMLElement, handle: PaneChromeAttachmentHandle): ActionReturn<PaneChromeAttachmentHandle> => {
  let current = handle;
  let hovered = false;
  let focused = false;

  const enter = (event: PointerEvent) => {
    hovered = true;
    current.hold(event);
  };
  const leave = () => {
    hovered = false;
    if (!focused) current.release();
  };
  const focusIn = (event: FocusEvent) => {
    if (event.relatedTarget instanceof Node && node.contains(event.relatedTarget)) return;
    focused = true;
    current.hold();
  };
  const focusOut = (event: FocusEvent) => {
    if (event.relatedTarget instanceof Node && node.contains(event.relatedTarget)) return;
    focused = false;
    if (!hovered) current.release();
  };

  node.addEventListener('pointerenter', enter);
  node.addEventListener('pointerleave', leave);
  node.addEventListener('focusin', focusIn);
  node.addEventListener('focusout', focusOut);

  return {
    update(next) {
      if (next === current) return;
      current.release();
      current = next;
      if (hovered || focused) current.hold();
    },
    destroy() {
      node.removeEventListener('pointerenter', enter);
      node.removeEventListener('pointerleave', leave);
      node.removeEventListener('focusin', focusIn);
      node.removeEventListener('focusout', focusOut);
      current.release();
    },
  };
};
