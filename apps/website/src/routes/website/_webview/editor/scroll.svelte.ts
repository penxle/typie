import { on } from 'svelte/events';
import type { Action } from 'svelte/action';

export const scroll: Action<HTMLElement> = (element) => {
  const FRICTION = 0.95;
  const MIN_VELOCITY = 0.5;

  let panning = false;
  let initialY = 0;
  let initialTop = 0;
  let velocity = 0;
  let lastEventAt = 0;
  let lastY = 0;
  let raf = 0;

  const momentumScroll = () => {
    if (!element || Math.abs(velocity) < MIN_VELOCITY) {
      raf = 0;
      return;
    }

    const currentScrollTop = element.scrollTop;
    const newScrollTop = currentScrollTop + velocity;
    const maxScrollTop = element.scrollHeight - element.clientHeight;

    if (newScrollTop < 0 || newScrollTop > maxScrollTop) {
      element.scrollTop = Math.max(0, Math.min(newScrollTop, maxScrollTop));
      raf = 0;
      velocity = 0;
      return;
    }

    element.scrollTop = newScrollTop;
    velocity *= FRICTION;

    raf = requestAnimationFrame(momentumScroll);
  };

  const handleTouchStart = (e: TouchEvent) => {
    if (e.touches.length === 1 && element) {
      if (raf) {
        cancelAnimationFrame(raf);
        raf = 0;
      }

      panning = true;
      initialY = e.touches[0].clientY;
      initialTop = element.scrollTop;

      lastEventAt = Date.now();
      lastY = e.touches[0].clientY;
      velocity = 0;
    }
  };

  const handleTouchMove = (e: TouchEvent) => {
    if (!panning || !element || e.touches.length !== 1) {
      return;
    }

    e.preventDefault();
    e.stopPropagation();

    const currentTime = Date.now();
    const currentY = e.touches[0].clientY;
    const deltaTime = currentTime - lastEventAt;

    if (deltaTime > 0) {
      velocity = ((lastY - currentY) / deltaTime) * 16;
    }

    const top = initialTop + initialY - currentY;
    const max = element.scrollHeight - element.clientHeight;

    element.scrollTop = Math.max(0, Math.min(top, max));

    lastEventAt = currentTime;
    lastY = currentY;
  };

  const handleTouchEnd = () => {
    panning = false;

    if (Math.abs(velocity) > MIN_VELOCITY) {
      raf = requestAnimationFrame(momentumScroll);
    }
  };

  $effect(() => {
    const touchstart = on(window, 'touchstart', handleTouchStart);
    const touchmove = on(window, 'touchmove', handleTouchMove);
    const touchend = on(window, 'touchend', handleTouchEnd);

    return () => {
      touchstart();
      touchmove();
      touchend();

      if (raf) {
        cancelAnimationFrame(raf);
      }
    };
  });
};
