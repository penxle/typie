import { arrow as arrowMiddleware, autoUpdate, computePosition, flip, hide, offset, shift } from '@floating-ui/dom';
import { mount, tick, unmount } from 'svelte';
import TooltipComponent from './TooltipComponent.svelte';
import type { Placement } from '@floating-ui/dom';
import type { SystemStyleObject } from '@typie/styled-system/types';
import type { Component, Snippet } from 'svelte';
import type { Action } from 'svelte/action';

type ModifierKey = 'Mod' | 'Ctrl' | 'Alt' | 'Shift';

export type TooltipPresentation =
  | {
      kind: 'action';
      message?: string | null;
      trailing?: string;
      trailingIcon?: Component;
      keys?: [...ModifierKey[], string];
    }
  | {
      kind: 'wrapper';
      message?: string | Snippet;
      tooltipStyle?: SystemStyleObject;
    };

export type TooltipTriggerDescription = {
  element: HTMLElement;
  container: Element;
  eligible: boolean;
  pinned: boolean;
  suppressed: boolean;
  delay: number;
  placement: Placement;
  offset: number;
  arrow: boolean;
  presentation: TooltipPresentation;
};

export type TooltipTriggerRegistration = {
  enter: () => void;
  leave: () => void;
  close: () => void;
  update: (description: TooltipTriggerDescription) => void;
  destroy: () => void;
};

type TooltipHostProps = {
  presentation: TooltipPresentation;
  outgoingPresentation?: TooltipPresentation;
  contentHidden: boolean;
  floating: Action<HTMLElement>;
  presenceAction: Action<HTMLElement>;
  surfaceAction: Action<HTMLElement>;
  arrowAction: Action<HTMLElement>;
  contentAction: Action<HTMLElement>;
  outgoingContentAction: Action<HTMLElement>;
  showArrow: boolean;
  motion: 'idle' | 'travel' | 'crossfade';
};

type TriggerRecord = {
  description: TooltipTriggerDescription;
  hovered: boolean;
  destroyed: boolean;
};

type ResolvedSide = 'top' | 'right' | 'bottom' | 'left';

type PositionSnapshot = {
  record: TriggerRecord;
  x: number;
  y: number;
  side: ResolvedSide;
  container: Element;
};

type RenderedTooltipSnapshot = Pick<TooltipHostProps, 'presentation' | 'showArrow'> & {
  record: TriggerRecord;
  position?: PositionSnapshot;
};

type HostMotion =
  | { kind: 'idle' }
  | { kind: 'travel'; animation: Animation; referenceRect: DOMRect }
  | { kind: 'crossfade-out'; animation: Animation; target: TriggerRecord }
  | { kind: 'crossfade-in'; animation: Animation; target: TriggerRecord };

type PresenceMotion = { kind: 'idle' } | { kind: 'intro' | 'outro'; animation: Animation };

type SizeSnapshot = {
  width: number;
  height: number;
};

type ContentTransitionGeometry = {
  previousSurface: SizeSnapshot;
  nextSurface: SizeSnapshot;
  previousContent: SizeSnapshot;
  nextContent: SizeSnapshot;
};

type CalculatedPosition = Awaited<ReturnType<typeof computePosition>>;

class ReactiveProps<T extends object> {
  value: T = $state() as T;

  constructor(value: T) {
    this.value = value;
  }
}

const reactiveProps = <T extends object>(value: T): T => new ReactiveProps(value).value;

const TOOLTIP_SKIP_DELAY_MS = 300;
const TOOLTIP_LEAVE_GRACE_MS = 80;
const TOOLTIP_INTRO_MS = 200;
const TOOLTIP_OUTRO_MS = 100;
const TOOLTIP_PRESENCE_SCALE = 0.9;
const TOOLTIP_MAX_TRAVEL_PX = 160;
const TOOLTIP_TRAVEL_MS = 120;
const TOOLTIP_CONTENT_CROSSFADE_MS = 120;
const TOOLTIP_CROSSFADE_HALF_MS = 40;

const coordinators = new WeakMap<Document, TooltipCoordinator>();

const resolvedSide = (placement: Placement): ResolvedSide => placement.split('-')[0] as ResolvedSide;

const oppositeSide = {
  top: 'bottom',
  right: 'left',
  bottom: 'top',
  left: 'right',
} as const;

const arrowTransform = {
  top: 'rotate(-135deg)',
  right: 'rotate(-45deg)',
  bottom: 'rotate(45deg)',
  left: 'rotate(135deg)',
} as const;

const isAvailable = (record: TriggerRecord): boolean => !record.destroyed && record.description.eligible && !record.description.suppressed;

const snapshotPosition = (record: TriggerRecord, x: number, y: number, placement: Placement): PositionSnapshot => ({
  record,
  x,
  y,
  side: resolvedSide(placement),
  container: record.description.container,
});

const samePosition = (left: PositionSnapshot, right: PositionSnapshot): boolean =>
  Math.abs(left.x - right.x) < 0.5 && Math.abs(left.y - right.y) < 0.5 && left.side === right.side && left.container === right.container;

const sameRect = (left: DOMRect, right: DOMRect): boolean =>
  Math.abs(left.x - right.x) < 0.5 &&
  Math.abs(left.y - right.y) < 0.5 &&
  Math.abs(left.width - right.width) < 0.5 &&
  Math.abs(left.height - right.height) < 0.5;

const sameSurfaceStyle = (left: TooltipPresentation, right: TooltipPresentation): boolean => {
  const leftStyle = left.kind === 'wrapper' ? left.tooltipStyle : undefined;
  const rightStyle = right.kind === 'wrapper' ? right.tooltipStyle : undefined;
  return leftStyle === rightStyle;
};

const captureSize = (element: HTMLElement | undefined): SizeSnapshot | undefined => {
  if (!element) return;
  const style = element.ownerDocument.defaultView?.getComputedStyle(element);
  const rect = element.getBoundingClientRect();
  const width = Number.parseFloat(style?.width ?? '');
  const height = Number.parseFloat(style?.height ?? '');
  return {
    width: Number.isFinite(width) && width > 0 ? width : rect.width || element.offsetWidth,
    height: Number.isFinite(height) && height > 0 ? height : rect.height || element.offsetHeight,
  };
};

const captureContentSize = (element: HTMLElement | undefined): SizeSnapshot | undefined => {
  const size = captureSize(element);
  if (!element || !size) return size;
  return {
    width: Math.max(size.width, element.scrollWidth),
    height: Math.max(size.height, element.scrollHeight),
  };
};

const setSize = (element: HTMLElement | undefined, size: SizeSnapshot | undefined): void => {
  if (!element || !size) return;
  element.style.width = `${size.width}px`;
  element.style.height = `${size.height}px`;
};

const clearSize = (element: HTMLElement | undefined): void => {
  element?.style.removeProperty('width');
  element?.style.removeProperty('height');
};

class TooltipCoordinator {
  readonly #document: Document;
  readonly #records = new Set<TriggerRecord>();

  #active: TriggerRecord | undefined;
  #pending: TriggerRecord | undefined;
  #pendingTimer: ReturnType<typeof setTimeout> | undefined;
  #leaveTimer: ReturnType<typeof setTimeout> | undefined;
  #warmTimer: ReturnType<typeof setTimeout> | undefined;
  #warm = false;

  #component: Record<string, unknown> | undefined;
  #hostProps: TooltipHostProps | undefined;
  #rendered: RenderedTooltipSnapshot | undefined;
  #floatingElement: HTMLElement | undefined;
  #presenceElement: HTMLElement | undefined;
  #surfaceElement: HTMLElement | undefined;
  #arrowElement: HTMLElement | undefined;
  #contentElement: HTMLElement | undefined;
  #outgoingContentElement: HTMLElement | undefined;
  #cleanupAutoUpdate: (() => void) | undefined;
  #positionVersion = 0;
  #hostMotion: HostMotion = { kind: 'idle' };
  #presenceMotion: PresenceMotion = { kind: 'idle' };
  #introPending = false;
  #closing = false;
  #sizeAnimation: Animation | undefined;
  #arrowAnimation: Animation | undefined;
  #contentAnimations: Animation[] = [];
  #contentMotionToken = 0;

  readonly #floatingAction: Action<HTMLElement> = (element) => {
    this.#floatingElement = element;
    Object.assign(element.style, { position: 'absolute', top: '0', left: '0', visibility: 'hidden' });
    this.#restartPositioning();

    return {
      destroy: () => {
        if (this.#floatingElement === element) {
          this.#floatingElement = undefined;
          this.#stopPositioning();
        }
      },
    };
  };

  readonly #presenceAction: Action<HTMLElement> = (element) => {
    this.#presenceElement = element;
    return {
      destroy: () => {
        if (this.#presenceElement === element) this.#presenceElement = undefined;
      },
    };
  };

  readonly #arrowAction: Action<HTMLElement> = (element) => {
    this.#arrowElement = element;
    Object.assign(element.style, { position: 'absolute' });

    return {
      destroy: () => {
        if (this.#arrowElement === element) this.#arrowElement = undefined;
      },
    };
  };

  readonly #surfaceAction: Action<HTMLElement> = (element) => {
    this.#surfaceElement = element;
    return {
      destroy: () => {
        if (this.#surfaceElement === element) this.#surfaceElement = undefined;
      },
    };
  };

  readonly #contentAction: Action<HTMLElement> = (element) => {
    this.#contentElement = element;
    return {
      destroy: () => {
        if (this.#contentElement === element) this.#contentElement = undefined;
      },
    };
  };

  readonly #outgoingContentAction: Action<HTMLElement> = (element) => {
    this.#outgoingContentElement = element;
    return {
      destroy: () => {
        if (this.#outgoingContentElement === element) this.#outgoingContentElement = undefined;
      },
    };
  };

  constructor(document: Document) {
    this.#document = document;
  }

  #request(record: TriggerRecord): void {
    if (!isAvailable(record)) return;
    if (this.#active?.description.pinned && this.#active !== record) return;

    this.#clearLeaveTimer();
    if (this.#active) {
      if (this.#active === record) {
        this.#syncHost(record);
      } else {
        this.#show(record);
      }
      return;
    }

    if (this.#closing) {
      this.#show(record);
      return;
    }

    if (record.description.pinned || record.description.delay <= 0 || this.#warm) {
      this.#show(record);
      return;
    }

    this.#startPending(record);
  }

  #startPending(record: TriggerRecord): void {
    this.#clearPending();
    this.#pending = record;
    this.#pendingTimer = setTimeout(() => {
      this.#pendingTimer = undefined;
      if (this.#pending !== record || !isAvailable(record) || !record.hovered) return;
      this.#pending = undefined;
      this.#show(record);
    }, record.description.delay);
  }

  #leave(record: TriggerRecord): void {
    if (this.#pending === record) this.#clearPending();
    if (this.#active !== record || record.description.pinned) return;

    this.#clearLeaveTimer();
    this.#leaveTimer = setTimeout(() => {
      this.#leaveTimer = undefined;
      if (this.#active === record && !record.hovered && !record.description.pinned) this.#closeVisible();
    }, TOOLTIP_LEAVE_GRACE_MS);
  }

  #close(record: TriggerRecord): void {
    if (this.#pending === record) this.#clearPending();
    if (this.#active === record && !record.description.pinned) this.#closeVisible();
  }

  #update(record: TriggerRecord, description: TooltipTriggerDescription): void {
    const previousDescription = record.description;
    const wasAvailable = isAvailable(record);
    const wasPinned = previousDescription.pinned;
    const previousDelay = previousDescription.delay;
    record.description = description;
    const available = isAvailable(record);

    if (this.#active === record) {
      if (!available) {
        this.#closeVisible();
        return;
      }
      if (wasPinned && !description.pinned && !record.hovered) {
        this.#closeVisible();
        return;
      }
      if (this.#rendered?.record !== record) return;
      const geometryChanged =
        description.element !== previousDescription.element ||
        description.container !== previousDescription.container ||
        description.placement !== previousDescription.placement ||
        description.offset !== previousDescription.offset ||
        description.arrow !== previousDescription.arrow;
      if (geometryChanged) this.#syncHost(record);
      else this.#presentHostDescription(record);
      return;
    }

    if (this.#pending === record) {
      if (!available || !record.hovered) this.#clearPending();
      else if (description.pinned || description.delay <= 0 || this.#warm) this.#show(record);
      else if (description.delay !== previousDelay) this.#startPending(record);
      return;
    }

    if (!available) return;
    if (description.pinned || (!wasAvailable && record.hovered)) this.#request(record);
  }

  #destroy(record: TriggerRecord): void {
    if (record.destroyed) return;
    record.destroyed = true;
    this.#records.delete(record);
    if (this.#pending === record) this.#clearPending();
    if (this.#active === record) this.#closeVisible();
    this.#cleanupIfUnused();
  }

  #show(record: TriggerRecord): void {
    this.#clearPending();
    this.#clearLeaveTimer();
    this.#clearWarmTimer();
    const previous = this.#active;
    const reopening = this.#closing;
    this.#active = record;

    if (this.#component) {
      if (reopening) this.#reopenPresence();
      if ((previous && previous !== record) || (reopening && this.#rendered?.record !== record)) {
        if (!reopening) this.#finishIntro();
        void this.#switchTarget(record);
      } else {
        this.#syncHost(record);
      }
      return;
    }

    this.#closing = false;
    this.#introPending = true;
    const hostProps = reactiveProps<TooltipHostProps>({
      presentation: record.description.presentation,
      outgoingPresentation: undefined,
      contentHidden: false,
      floating: this.#floatingAction,
      presenceAction: this.#presenceAction,
      surfaceAction: this.#surfaceAction,
      arrowAction: this.#arrowAction,
      contentAction: this.#contentAction,
      outgoingContentAction: this.#outgoingContentAction,
      showArrow: record.description.arrow,
      motion: 'idle',
    });
    this.#hostProps = hostProps;
    this.#rendered = {
      record,
      presentation: record.description.presentation,
      showArrow: record.description.arrow,
    };
    this.#component = mount(TooltipComponent, {
      target: record.description.container,
      props: hostProps,
    }) as Record<string, unknown>;
  }

  #syncHost(record: TriggerRecord): void {
    this.#cancelMotion();
    this.#clearContentTransition();
    this.#presentHostDescription(record);
    this.#attachHost(record);
    this.#restartPositioning();
  }

  #attachHost(record: TriggerRecord): void {
    const floatingElement = this.#floatingElement;
    if (floatingElement && floatingElement.parentElement !== record.description.container)
      record.description.container.append(floatingElement);
  }

  #applyHostDescription(record: TriggerRecord): void {
    if (!this.#hostProps) return;
    this.#hostProps.presentation = record.description.presentation;
    this.#hostProps.showArrow = record.description.arrow;
  }

  #presentHostDescription(record: TriggerRecord): void {
    this.#applyHostDescription(record);
    this.#rendered = {
      record,
      presentation: record.description.presentation,
      showArrow: record.description.arrow,
      position: this.#rendered?.record === record ? this.#rendered.position : undefined,
    };
  }

  #restoreRenderedDescription(): void {
    if (!this.#hostProps || !this.#rendered) return;
    this.#hostProps.presentation = this.#rendered.presentation;
    this.#hostProps.showArrow = this.#rendered.showArrow;
  }

  #closeVisible(): void {
    if (this.#closing || (!this.#active && !this.#component)) return;
    this.#active = undefined;
    this.#closing = true;
    this.#clearLeaveTimer();
    this.#stopPositioning();
    this.#cancelMotion();
    this.#clearContentTransition();
    this.#restoreRenderedDescription();
    if (!this.#component || !this.#presenceElement || this.#prefersReducedMotion()) {
      this.#finishClose();
      return;
    }
    this.#startOutro();
  }

  #startIntro(): void {
    if (!this.#presenceElement || !this.#introPending) return;
    this.#introPending = false;
    if (this.#prefersReducedMotion()) return;

    this.#animatePresenceIn('0', `scale(${TOOLTIP_PRESENCE_SCALE})`, TOOLTIP_INTRO_MS);
  }

  #animatePresenceIn(opacity: string, transform: string, duration: number): void {
    const element = this.#presenceElement;
    if (!element) return;
    this.#cancelPresenceMotion();
    const animation = element.animate(
      [
        { opacity, transform },
        { opacity: 1, transform: 'scale(1)' },
      ],
      { duration, easing: 'cubic-bezier(0.2, 0, 0, 1)', fill: 'both' },
    );
    this.#trackPresenceMotion('intro', animation);
  }

  #startOutro(): void {
    const element = this.#presenceElement;
    if (!element) {
      this.#finishClose();
      return;
    }

    this.#introPending = false;
    const style = element.ownerDocument.defaultView?.getComputedStyle(element);
    const opacity = style?.opacity || '1';
    const transform = !style?.transform || style.transform === 'none' ? 'scale(1)' : style.transform;
    this.#cancelPresenceMotion();
    const animation = element.animate(
      [
        { opacity, transform },
        { opacity: 0, transform: `scale(${TOOLTIP_PRESENCE_SCALE})` },
      ],
      { duration: TOOLTIP_OUTRO_MS, easing: 'cubic-bezier(0.4, 0, 1, 1)', fill: 'both' },
    );
    this.#trackPresenceMotion('outro', animation);
  }

  #trackPresenceMotion(kind: 'intro' | 'outro', animation: Animation): void {
    const motion: PresenceMotion = { kind, animation };
    this.#presenceMotion = motion;
    animation.onfinish = () => {
      if (this.#presenceMotion !== motion) return;
      if (this.#closing) this.#finishClose();
      else this.#finishPresenceMotion(motion);
    };
  }

  #reopenPresence(): void {
    this.#closing = false;
    if (this.#presenceMotion.kind !== 'outro') return;
    const element = this.#presenceElement;
    if (!element) return;
    const style = element.ownerDocument.defaultView?.getComputedStyle(element);
    const opacity = style?.opacity || '0';
    const transform = !style?.transform || style.transform === 'none' ? `scale(${TOOLTIP_PRESENCE_SCALE})` : style.transform;
    this.#introPending = false;
    this.#animatePresenceIn(opacity, transform, TOOLTIP_OUTRO_MS);
  }

  #finishIntro(): void {
    if (this.#presenceMotion.kind === 'intro') this.#finishPresenceMotion(this.#presenceMotion);
  }

  #finishPresenceMotion(motion: Exclude<PresenceMotion, { kind: 'idle' }>): void {
    if (this.#presenceMotion !== motion) return;
    motion.animation.cancel();
    this.#presenceMotion = { kind: 'idle' };
  }

  #cancelPresenceMotion(): void {
    if (this.#presenceMotion.kind !== 'idle') this.#presenceMotion.animation.cancel();
    this.#presenceMotion = { kind: 'idle' };
  }

  #finishClose(): void {
    if (!this.#closing) return;
    this.#cancelPresenceMotion();
    const component = this.#component;
    this.#closing = false;
    this.#introPending = false;
    this.#component = undefined;
    this.#hostProps = undefined;
    this.#rendered = undefined;
    this.#floatingElement = undefined;
    this.#presenceElement = undefined;
    this.#surfaceElement = undefined;
    this.#arrowElement = undefined;
    this.#contentElement = undefined;
    this.#outgoingContentElement = undefined;
    if (component) void unmount(component);
    this.#startWarmTimer();
  }

  #startWarmTimer(): void {
    this.#clearWarmTimer();
    this.#warm = true;
    this.#warmTimer = setTimeout(() => {
      this.#warmTimer = undefined;
      this.#warm = false;
      this.#cleanupIfUnused();
    }, TOOLTIP_SKIP_DELAY_MS);
  }

  #clearPending(): void {
    if (this.#pendingTimer) clearTimeout(this.#pendingTimer);
    this.#pendingTimer = undefined;
    this.#pending = undefined;
  }

  #clearLeaveTimer(): void {
    if (this.#leaveTimer) clearTimeout(this.#leaveTimer);
    this.#leaveTimer = undefined;
  }

  #clearWarmTimer(): void {
    if (this.#warmTimer) clearTimeout(this.#warmTimer);
    this.#warmTimer = undefined;
    this.#warm = false;
  }

  #cleanupIfUnused(): void {
    if (this.#records.size > 0 || this.#active || this.#pending || this.#warm || this.#component) return;
    this.#clearPending();
    this.#clearLeaveTimer();
    this.#stopPositioning();
    coordinators.delete(this.#document);
  }

  #restartPositioning(): void {
    this.#stopPositioning();
    const record = this.#active;
    if (record) this.#startTracking(record);
  }

  #startTracking(record: TriggerRecord, elementResize = true): void {
    this.#cleanupAutoUpdate?.();
    const floatingElement = this.#floatingElement;
    if (!floatingElement || this.#active !== record) return;
    const update = () => void this.#updatePosition(record);
    this.#cleanupAutoUpdate = autoUpdate(record.description.element, floatingElement, update, { animationFrame: true, elementResize });
  }

  #stopPositioning(): void {
    this.#positionVersion++;
    this.#cleanupAutoUpdate?.();
    this.#cleanupAutoUpdate = undefined;
  }

  async #switchTarget(record: TriggerRecord): Promise<void> {
    const previousPosition = this.#captureVisualPosition();
    const previousSurfaceSize = captureSize(this.#surfaceElement);
    const previousContentSize = this.#captureRenderedContentSize();
    this.#stopPositioning();
    this.#cancelMotion(previousPosition, previousSurfaceSize);
    this.#clearContentTransition();
    this.#restoreRenderedDescription();
    await tick();
    if (this.#active !== record) return;

    const previousPresentation = this.#rendered?.presentation;
    if (!previousPosition || !previousPresentation || this.#prefersReducedMotion()) {
      await this.#settleImmediately(record);
      return;
    }

    if (
      previousPosition.container !== record.description.container ||
      !sameSurfaceStyle(previousPresentation, record.description.presentation)
    ) {
      this.#startCrossfade(record);
      return;
    }

    const contentMotionToken = this.#prepareContentCrossfade(record, previousPresentation);
    await tick();
    if (this.#active !== record) return;
    setSize(this.#outgoingContentElement, previousContentSize);
    clearSize(this.#surfaceElement);

    const result = await this.#calculatePosition(record);
    if (!result || this.#active !== record) return;
    const destination = snapshotPosition(record, result.x, result.y, result.placement);
    const nextSurfaceSize = captureSize(this.#surfaceElement);
    const nextContentSize = captureContentSize(this.#contentElement);
    setSize(this.#contentElement, nextContentSize);
    setSize(this.#outgoingContentElement, previousContentSize);

    const displacement = Math.hypot(destination.x - previousPosition.x, destination.y - previousPosition.y);
    const travels = displacement <= TOOLTIP_MAX_TRAVEL_PX && destination.side === previousPosition.side;

    if (travels) {
      this.#presentHostDescription(record);
      this.#applyPosition(record, result);
      const contentGeometry =
        previousSurfaceSize && nextSurfaceSize && previousContentSize && nextContentSize
          ? {
              previousSurface: previousSurfaceSize,
              nextSurface: nextSurfaceSize,
              previousContent: previousContentSize,
              nextContent: nextContentSize,
            }
          : undefined;
      this.#startContentCrossfade(record, contentMotionToken, contentGeometry);
      this.#startSizeTransition(previousSurfaceSize, nextSurfaceSize);
      this.#pinArrowDuringTravel(previousPosition, destination);
      this.#startTravel(previousPosition, destination);
      this.#startTracking(record, false);
      return;
    }

    this.#restoreRenderedDescription();
    this.#clearContentTransition();
    setSize(this.#surfaceElement, previousSurfaceSize);
    await tick();
    if (this.#active !== record) return;
    this.#startCrossfade(record);
  }

  async #settleImmediately(record: TriggerRecord): Promise<void> {
    this.#presentHostDescription(record);
    this.#clearContentTransition();
    clearSize(this.#surfaceElement);
    this.#attachHost(record);
    const result = await this.#calculatePosition(record);
    if (!result || this.#active !== record) return;
    this.#applyPosition(record, result);
    this.#startTracking(record);
  }

  #startTravel(previous: PositionSnapshot, destination: PositionSnapshot): void {
    const floatingElement = this.#floatingElement;
    if (!floatingElement || !this.#hostProps) return;
    const animation = floatingElement.animate(
      [
        { left: `${previous.x}px`, top: `${previous.y}px` },
        { left: `${destination.x}px`, top: `${destination.y}px` },
      ],
      { duration: TOOLTIP_TRAVEL_MS, easing: 'cubic-bezier(0.2, 0, 0, 1)' },
    );
    const motion: HostMotion = {
      kind: 'travel',
      animation,
      referenceRect: destination.record.description.element.getBoundingClientRect(),
    };
    this.#setHostMotion(motion);
    animation.onfinish = () => {
      if (this.#hostMotion !== motion) return;
      this.#setHostMotion({ kind: 'idle' });
      this.#startTracking(destination.record);
    };
  }

  #captureVisualPosition(): PositionSnapshot | undefined {
    const position = this.#rendered?.position;
    const floatingElement = this.#floatingElement;
    if (!position || !floatingElement) return position;
    const style = floatingElement.ownerDocument.defaultView?.getComputedStyle(floatingElement);
    const x = Number.parseFloat(style?.left ?? '');
    const y = Number.parseFloat(style?.top ?? '');
    return { ...position, x: Number.isFinite(x) ? x : position.x, y: Number.isFinite(y) ? y : position.y };
  }

  #captureRenderedContentSize(): SizeSnapshot | undefined {
    const element =
      this.#hostProps?.outgoingPresentation === this.#rendered?.presentation ? this.#outgoingContentElement : this.#contentElement;
    return captureContentSize(element);
  }

  #startSizeTransition(previous: SizeSnapshot | undefined, next: SizeSnapshot | undefined): void {
    const surfaceElement = this.#surfaceElement;
    if (!surfaceElement || !previous || !next || (previous.width === next.width && previous.height === next.height)) {
      clearSize(this.#surfaceElement);
      return;
    }
    setSize(surfaceElement, next);
    const animation = surfaceElement.animate(
      [
        { width: `${previous.width}px`, height: `${previous.height}px` },
        { width: `${next.width}px`, height: `${next.height}px` },
      ],
      { duration: TOOLTIP_TRAVEL_MS, easing: 'cubic-bezier(0.2, 0, 0, 1)' },
    );
    this.#sizeAnimation = animation;
    animation.onfinish = () => {
      if (this.#sizeAnimation !== animation) return;
      animation.cancel();
      this.#sizeAnimation = undefined;
      clearSize(this.#surfaceElement);
    };
  }

  #pinArrowDuringTravel(previous: PositionSnapshot, destination: PositionSnapshot): void {
    const element = this.#arrowElement;
    if (!element) return;
    const axis = destination.side === 'top' || destination.side === 'bottom' ? 'left' : 'top';
    const value = Number.parseFloat(element.ownerDocument.defaultView?.getComputedStyle(element)[axis] ?? '');
    if (!Number.isFinite(value)) return;
    const travel = axis === 'left' ? destination.x - previous.x : destination.y - previous.y;
    if (Math.abs(travel) < 0.5) return;
    const keyframes =
      axis === 'left' ? [{ left: `${value + travel}px` }, { left: `${value}px` }] : [{ top: `${value + travel}px` }, { top: `${value}px` }];
    const animation = element.animate(keyframes, {
      duration: TOOLTIP_TRAVEL_MS,
      easing: 'cubic-bezier(0.2, 0, 0, 1)',
    });
    this.#arrowAnimation = animation;
    animation.onfinish = () => {
      if (this.#arrowAnimation !== animation) return;
      animation.cancel();
      this.#arrowAnimation = undefined;
    };
  }

  #startCrossfade(record: TriggerRecord): void {
    const floatingElement = this.#floatingElement;
    if (!floatingElement || !this.#hostProps) return;
    const fadeOut = floatingElement.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: TOOLTIP_CROSSFADE_HALF_MS,
      easing: 'ease-out',
      fill: 'forwards',
    });
    const motion: HostMotion = { kind: 'crossfade-out', animation: fadeOut, target: record };
    this.#setHostMotion(motion);
    fadeOut.onfinish = () => void this.#finishCrossfade(floatingElement, motion);
  }

  #prepareContentCrossfade(record: TriggerRecord, outgoingPresentation: TooltipPresentation): number {
    const hostProps = this.#hostProps;
    const token = ++this.#contentMotionToken;
    if (!hostProps) return token;
    hostProps.outgoingPresentation = outgoingPresentation;
    hostProps.contentHidden = true;
    this.#applyHostDescription(record);
    return token;
  }

  #startContentCrossfade(record: TriggerRecord, token: number, geometry: ContentTransitionGeometry | undefined): void {
    const outgoingContentElement = this.#outgoingContentElement;
    const contentElement = this.#contentElement;
    if (!outgoingContentElement || !contentElement || this.#active !== record || this.#contentMotionToken !== token) {
      this.#clearContentTransition();
      return;
    }
    const outgoing = outgoingContentElement.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: TOOLTIP_CONTENT_CROSSFADE_MS,
      fill: 'forwards',
    });
    const incoming = contentElement.animate([{ opacity: 0 }, { opacity: 1 }], {
      duration: TOOLTIP_CONTENT_CROSSFADE_MS,
      fill: 'forwards',
    });
    const outgoingPosition = geometry
      ? this.#centerContentDuringResize(outgoingContentElement, geometry.previousContent, geometry.previousSurface, geometry.nextSurface)
      : undefined;
    const incomingPosition = geometry
      ? this.#centerContentDuringResize(contentElement, geometry.nextContent, geometry.previousSurface, geometry.nextSurface)
      : undefined;
    this.#contentAnimations = [outgoing, incoming, outgoingPosition, incomingPosition].filter(
      (animation): animation is Animation => animation !== undefined,
    );
    incoming.onfinish = () => void this.#finishContentCrossfade(record, token, incoming);
  }

  #centerContentDuringResize(
    element: HTMLElement,
    content: SizeSnapshot,
    previousSurface: SizeSnapshot,
    nextSurface: SizeSnapshot,
  ): Animation | undefined {
    const surfaceElement = this.#surfaceElement;
    if (!surfaceElement) return;
    const style = surfaceElement.ownerDocument.defaultView?.getComputedStyle(surfaceElement);
    const horizontalPadding = Number.parseFloat(style?.paddingLeft ?? '') + Number.parseFloat(style?.paddingRight ?? '');
    const verticalPadding = Number.parseFloat(style?.paddingTop ?? '') + Number.parseFloat(style?.paddingBottom ?? '');
    if (!Number.isFinite(horizontalPadding) || !Number.isFinite(verticalPadding)) return;
    const offset = (surface: SizeSnapshot) => ({
      x: (surface.width - horizontalPadding - content.width) / 2,
      y: (surface.height - verticalPadding - content.height) / 2,
    });
    const from = offset(previousSurface);
    const to = offset(nextSurface);
    if (Math.abs(from.x - to.x) < 0.5 && Math.abs(from.y - to.y) < 0.5) return;
    return element.animate([{ transform: `translate(${from.x}px, ${from.y}px)` }, { transform: `translate(${to.x}px, ${to.y}px)` }], {
      duration: TOOLTIP_TRAVEL_MS,
      easing: 'cubic-bezier(0.2, 0, 0, 1)',
    });
  }

  async #finishContentCrossfade(record: TriggerRecord, token: number, incoming: Animation): Promise<void> {
    if (this.#contentMotionToken !== token || !this.#contentAnimations.includes(incoming) || this.#active !== record) return;
    const animations = this.#contentAnimations;
    if (this.#hostProps) {
      this.#hostProps.contentHidden = false;
      this.#hostProps.outgoingPresentation = undefined;
    }
    await tick();
    if (this.#contentMotionToken !== token || this.#active !== record) return;
    for (const animation of animations) animation.cancel();
    this.#contentAnimations = [];
    clearSize(this.#contentElement);
    clearSize(this.#outgoingContentElement);
  }

  #clearContentTransition(): void {
    this.#contentMotionToken++;
    for (const animation of this.#contentAnimations) animation.cancel();
    this.#contentAnimations = [];
    if (this.#hostProps) {
      this.#hostProps.outgoingPresentation = undefined;
      this.#hostProps.contentHidden = false;
    }
    clearSize(this.#contentElement);
    clearSize(this.#outgoingContentElement);
  }

  async #finishCrossfade(floatingElement: HTMLElement, motion: Extract<HostMotion, { kind: 'crossfade-out' }>): Promise<void> {
    const record = motion.target;
    if (this.#hostMotion !== motion || this.#active !== record) return;
    this.#presentHostDescription(record);
    clearSize(this.#surfaceElement);
    this.#attachHost(record);
    const result = await this.#calculatePosition(record);
    if (!result || this.#active !== record || this.#floatingElement !== floatingElement || this.#hostMotion !== motion) return;
    this.#applyPosition(record, result);
    this.#startTracking(record);
    motion.animation.cancel();

    const fadeIn = floatingElement.animate([{ opacity: 0 }, { opacity: 1 }], {
      duration: TOOLTIP_CROSSFADE_HALF_MS,
      easing: 'ease-out',
    });
    const fadeInMotion: HostMotion = { kind: 'crossfade-in', animation: fadeIn, target: record };
    this.#setHostMotion(fadeInMotion);
    fadeIn.onfinish = () => {
      if (this.#hostMotion !== fadeInMotion) return;
      this.#setHostMotion({ kind: 'idle' });
    };
  }

  #setHostMotion(motion: HostMotion): void {
    this.#hostMotion = motion;
    if (!this.#hostProps) return;
    this.#hostProps.motion = motion.kind === 'travel' ? 'travel' : motion.kind === 'idle' ? 'idle' : 'crossfade';
  }

  #cancelMotion(position?: PositionSnapshot, surfaceSize?: SizeSnapshot): void {
    if (this.#hostMotion.kind !== 'idle') this.#hostMotion.animation.cancel();
    this.#setHostMotion({ kind: 'idle' });
    this.#sizeAnimation?.cancel();
    this.#sizeAnimation = undefined;
    this.#arrowAnimation?.cancel();
    this.#arrowAnimation = undefined;
    if (position && this.#floatingElement) {
      this.#floatingElement.style.left = `${position.x}px`;
      this.#floatingElement.style.top = `${position.y}px`;
    }
    if (surfaceSize) setSize(this.#surfaceElement, surfaceSize);
    else clearSize(this.#surfaceElement);
    this.#floatingElement?.style.removeProperty('opacity');
  }

  #prefersReducedMotion(): boolean {
    const view = this.#document.defaultView;
    return view?.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  }

  async #updatePosition(record: TriggerRecord): Promise<void> {
    const result = await this.#calculatePosition(record);
    if (!result) return;
    const snapshot = snapshotPosition(record, result.x, result.y, result.placement);
    const renderedPosition = this.#rendered?.position;
    const interruptedTravel = this.#hostMotion.kind === 'travel' && renderedPosition && !samePosition(snapshot, renderedPosition);
    if (
      interruptedTravel &&
      this.#hostMotion.kind === 'travel' &&
      sameRect(record.description.element.getBoundingClientRect(), this.#hostMotion.referenceRect)
    ) {
      return;
    }
    if (interruptedTravel) this.#cancelMotion();
    this.#applyPosition(record, result);
    if (interruptedTravel) this.#startTracking(record);
  }

  async #calculatePosition(record: TriggerRecord) {
    const version = ++this.#positionVersion;
    await tick();
    const floatingElement = this.#floatingElement;
    const arrowElement = this.#arrowElement;
    if (!floatingElement) return;
    if (version !== this.#positionVersion) return;
    if (this.#active !== record) return;
    if (!record.description.element.isConnected) {
      this.#closeVisible();
      return;
    }

    const result = await computePosition(record.description.element, floatingElement, {
      strategy: 'absolute',
      placement: record.description.placement,
      middleware: [
        offset(record.description.offset),
        flip(),
        shift({ padding: 8 }),
        arrowElement && record.description.arrow ? arrowMiddleware({ element: arrowElement, padding: 16 }) : false,
        hide({ strategy: 'referenceHidden' }),
      ],
    });

    if (version !== this.#positionVersion) return;
    if (this.#active !== record) return;
    if (this.#floatingElement !== floatingElement) return;
    if (result.middlewareData.hide?.referenceHidden) {
      this.#closeVisible();
      return;
    }

    return result;
  }

  #applyPosition(record: TriggerRecord, result: CalculatedPosition): void {
    const floatingElement = this.#floatingElement;
    const arrowElement = this.#arrowElement;
    if (!floatingElement || this.#active !== record) return;
    Object.assign(floatingElement.style, {
      position: result.strategy,
      top: `${result.y}px`,
      left: `${result.x}px`,
      visibility: 'visible',
    });
    if (this.#rendered?.record === record) this.#rendered.position = snapshotPosition(record, result.x, result.y, result.placement);
    this.#startIntro();

    if (!arrowElement || !result.middlewareData.arrow) return;
    const side = resolvedSide(result.placement);
    Object.assign(arrowElement.style, {
      top: '',
      right: '',
      bottom: '',
      left: '',
      transform: arrowTransform[side],
      visibility: 'visible',
    });
    arrowElement.style.left = result.middlewareData.arrow.x === undefined ? '' : `${result.middlewareData.arrow.x}px`;
    arrowElement.style.top = result.middlewareData.arrow.y === undefined ? '' : `${result.middlewareData.arrow.y}px`;
    arrowElement.style[oppositeSide[side]] = `${-arrowElement.offsetHeight / 2}px`;
  }

  register(description: TooltipTriggerDescription): TooltipTriggerRegistration {
    const record: TriggerRecord = { description, hovered: false, destroyed: false };
    this.#records.add(record);

    if (isAvailable(record) && record.description.pinned) this.#request(record);

    return {
      enter: () => {
        if (record.destroyed) return;
        record.hovered = true;
        this.#request(record);
      },
      leave: () => {
        if (record.destroyed) return;
        record.hovered = false;
        this.#leave(record);
      },
      close: () => {
        if (!record.destroyed) this.#close(record);
      },
      update: (next) => {
        if (!record.destroyed) this.#update(record, next);
      },
      destroy: () => this.#destroy(record),
    };
  }
}

export const registerTooltipTrigger = (description: TooltipTriggerDescription): TooltipTriggerRegistration => {
  const document = description.element.ownerDocument;
  let coordinator = coordinators.get(document);
  if (!coordinator) {
    coordinator = new TooltipCoordinator(document);
    coordinators.set(document, coordinator);
  }
  return coordinator.register(description);
};
