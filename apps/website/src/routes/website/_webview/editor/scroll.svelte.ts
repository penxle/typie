import { on } from 'svelte/events';
import type { Action } from 'svelte/action';

type Attributes = {
  onmomentumscrollstart?: (e: CustomEvent) => void;
  onmomentumscroll?: (e: CustomEvent) => void;
  onmomentumscrollend?: (e: CustomEvent) => void;
};

export const scroll: Action<HTMLElement, undefined, Attributes> = (container) => {
  const FRICTION = 0.95;
  const MIN_VELOCITY = 0.5;

  let scrolling = false;
  let panning = false;
  let native = false;
  let initialY = 0;
  let initialTranslateY = 0;
  let currentTranslateY = 0;
  let maxTranslateY = 0;
  let velocity = 0;
  let lastEventAt = 0;
  let lastY = 0;
  let raf = 0;

  const content = container.firstElementChild as HTMLElement;
  if (!content) {
    throw new Error('Scroll action target must have at least one child element');
  }

  content.style.willChange = 'transform';
  content.style.transform = 'translate3d(0, 0, 0)';

  const updateMaxTranslate = () => {
    const contentHeight = content.scrollHeight;
    const containerHeight = container.clientHeight;
    maxTranslateY = Math.min(0, containerHeight - contentHeight);
  };

  const setTranslateY = (y: number) => {
    currentTranslateY = Math.min(0, Math.max(maxTranslateY, y));
    content.style.transform = `translate3d(0, ${currentTranslateY}px, 0)`;
    container.dispatchEvent(new CustomEvent('momentumscroll'));
  };

  const momentumScroll = () => {
    if (Math.abs(velocity) < MIN_VELOCITY) {
      raf = 0;
      scrolling = false;
      container.dispatchEvent(new CustomEvent('momentumscrollend'));
      return;
    }

    const newTranslateY = currentTranslateY - velocity;

    if (newTranslateY > 0 || newTranslateY < maxTranslateY) {
      setTranslateY(newTranslateY > 0 ? 0 : maxTranslateY);
      velocity = 0;
      raf = 0;
      scrolling = false;
      container.dispatchEvent(new CustomEvent('momentumscrollend'));
      return;
    }

    setTranslateY(newTranslateY);
    velocity *= FRICTION;
    raf = requestAnimationFrame(momentumScroll);
  };

  const stopMomentumScroll = () => {
    if (raf) {
      cancelAnimationFrame(raf);
      raf = 0;
      if (scrolling) {
        scrolling = false;
        container.dispatchEvent(new CustomEvent('momentumscrollend'));
      }
    }
  };

  const handleTouchStart = (e: TouchEvent) => {
    if (e.touches.length !== 1) return;
    stopMomentumScroll();

    const touch = e.touches[0];
    panning = true;
    scrolling = true;
    initialY = touch.clientY;
    initialTranslateY = currentTranslateY;
    lastEventAt = Date.now();
    lastY = touch.clientY;
    velocity = 0;
    container.dispatchEvent(new CustomEvent('momentumscrollstart'));
  };

  const handleTouchMove = (e: TouchEvent) => {
    if (!panning || e.touches.length !== 1) return;
    e.preventDefault();
    e.stopPropagation();

    const touch = e.touches[0];
    const currentTime = Date.now();
    const currentY = touch.clientY;
    const deltaTime = currentTime - lastEventAt;

    if (deltaTime > 0) {
      velocity = ((lastY - currentY) / deltaTime) * 16;
    }

    const deltaY = currentY - initialY;
    setTranslateY(initialTranslateY + deltaY);
    lastEventAt = currentTime;
    lastY = currentY;
  };

  const handleTouchEnd = () => {
    panning = false;
    if (Math.abs(velocity) > MIN_VELOCITY) {
      raf = requestAnimationFrame(momentumScroll);
    } else {
      scrolling = false;
      container.dispatchEvent(new CustomEvent('momentumscrollend'));
    }
  };

  const handleResize = () => {
    updateMaxTranslate();
    if (currentTranslateY < maxTranslateY) {
      setTranslateY(maxTranslateY);
    }
  };

  const handleScroll = () => {
    if (native) return;

    native = true;
    stopMomentumScroll();

    const scrollTop = container.scrollTop;
    if (scrollTop !== 0) {
      currentTranslateY = Math.min(0, Math.max(maxTranslateY, currentTranslateY - scrollTop));
      content.style.transform = `translate3d(0, ${currentTranslateY}px, 0)`;
      container.scrollTop = 0;
    }

    native = false;
  };

  $effect(() => {
    updateMaxTranslate();

    const cleanups = [
      on(container, 'touchstart', handleTouchStart),
      on(container, 'touchmove', handleTouchMove, { passive: false }),
      on(container, 'touchend', handleTouchEnd),
      on(container, 'scroll', handleScroll),
    ];

    const observer = new ResizeObserver(handleResize);
    observer.observe(container);
    observer.observe(content);

    return () => {
      cleanups.forEach((cleanup) => cleanup());
      observer.disconnect();
      stopMomentumScroll();
    };
  });
};
