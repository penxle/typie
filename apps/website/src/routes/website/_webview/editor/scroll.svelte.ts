import { on } from 'svelte/events';
import type { Action } from 'svelte/action';

type Attributes = {
  onmomentumscrollstart?: (e: CustomEvent) => void;
  onmomentumscroll?: (e: CustomEvent) => void;
  onmomentumscrollend?: (e: CustomEvent) => void;
};

export const scroll: Action<HTMLElement, undefined, Attributes> = (container) => {
  const FRICTION = 0.92;
  const MIN_VELOCITY = 0.3;
  const VELOCITY_MULTIPLIER = 20;

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

  const VELOCITY_HISTORY_SIZE = 5;
  let velocityHistory: number[] = [];
  let timeHistory: number[] = [];

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

  const calculateVelocity = () => {
    if (velocityHistory.length < 2) return 0;

    let weightedVelocity = 0;
    let totalWeight = 0;

    for (let i = velocityHistory.length - 1; i >= 0; i--) {
      const weight = (i + 1) * (i + 1);
      weightedVelocity += velocityHistory[i] * weight;
      totalWeight += weight;
    }

    return weightedVelocity / totalWeight;
  };

  const momentumScroll = () => {
    if (Math.abs(velocity) < MIN_VELOCITY) {
      stopMomentumScroll();
      return;
    }

    const newTranslateY = currentTranslateY - velocity;

    if (newTranslateY > 0 || newTranslateY < maxTranslateY) {
      setTranslateY(newTranslateY > 0 ? 0 : maxTranslateY);
      stopMomentumScroll();
      return;
    }

    setTranslateY(newTranslateY);
    velocity *= FRICTION;
    raf = requestAnimationFrame(momentumScroll);
  };

  const stopMomentumScroll = () => {
    velocity = 0;

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

    const touch = e.touches[0];
    const currentMomentumVelocity = scrolling ? velocity : 0;

    stopMomentumScroll();

    panning = true;
    scrolling = true;
    initialY = touch.clientY;
    initialTranslateY = currentTranslateY;
    lastEventAt = Date.now();
    lastY = touch.clientY;

    velocityHistory = [];
    timeHistory = [];

    velocity = currentMomentumVelocity * 0.2;

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
      const instantVelocity = ((lastY - currentY) / deltaTime) * VELOCITY_MULTIPLIER;

      velocityHistory.push(instantVelocity);
      timeHistory.push(currentTime);

      if (velocityHistory.length > VELOCITY_HISTORY_SIZE) {
        velocityHistory.shift();
        timeHistory.shift();
      }

      velocity = calculateVelocity();
    }

    const deltaY = currentY - initialY;
    setTranslateY(initialTranslateY + deltaY);
    lastEventAt = currentTime;
    lastY = currentY;
  };

  const handleTouchEnd = () => {
    panning = false;

    const finalVelocity = calculateVelocity();

    if (Math.abs(velocity) > 0 && Math.abs(finalVelocity) > 0) {
      const sameDirection = (velocity > 0 && finalVelocity > 0) || (velocity < 0 && finalVelocity < 0);
      if (sameDirection) {
        velocity = velocity * 0.3 + finalVelocity * 0.7;
      } else {
        velocity = finalVelocity;
      }
    } else {
      velocity = finalVelocity;
    }

    if (Math.abs(velocity) > MIN_VELOCITY) {
      raf = requestAnimationFrame(momentumScroll);
    } else {
      stopMomentumScroll();
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
