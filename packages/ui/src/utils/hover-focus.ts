export const createHoverFocusHandler = (selector: string) => {
  let lastPosition: { x: number; y: number } | undefined;

  // Focus follows hover only on real pointer movement, not on layout/scroll changes under a stationary pointer.
  return (e: PointerEvent) => {
    if (lastPosition?.x === e.screenX && lastPosition?.y === e.screenY) {
      return;
    }
    lastPosition = { x: e.screenX, y: e.screenY };

    const target = e.target as HTMLElement;
    const item = target.closest(selector);
    if (!(item instanceof HTMLElement) || !(e.currentTarget instanceof Node) || !e.currentTarget.contains(item)) {
      return;
    }

    // Inside a submenu safezone only the open trigger may take focus; other items stay inert while the pointer travels toward the submenu.
    if (target.closest('[data-submenu-safezone]') && item.getAttribute('aria-expanded') !== 'true') {
      return;
    }

    if (document.activeElement !== item) {
      item.focus({ preventScroll: true });
    }
  };
};
