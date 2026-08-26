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

type SizeSnapshot = {
  width: number;
  height: number;
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
  const rect = element.getBoundingClientRect();
  return { width: rect.width || element.offsetWidth, height: rect.height || element.offsetHeight };
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
  #surfaceElement: HTMLElement | undefined;
  #arrowElement: HTMLElement | undefined;
  #contentElement: HTMLElement | undefined;
  #outgoingContentElement: HTMLElement | undefined;
  #cleanupAutoUpdate: (() => void) | undefined;
  #positionVersion = 0;
  #hostMotion: HostMotion = { kind: 'idle' };
  #sizeAnimation: Animation | undefined;
  #contentAnimations: Animation[] = [];
  #contentMotionToken = 0;

  readonly #floatingAction: Action<HTMLElement> = (element) => {
    this.#floatingElement = element;
    Object.assign(element.style, { position: 'absolute', top: '0', left: '0', visibility: 'hidden' });
    this.#restartPositioning();

    return {
      destroy: () => {
        if (this.#floatingElement === element) this.#floatingElement = undefined;
        this.#stopPositioning();
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
    this.#active = record;

    if (!this.#component) {
      const hostProps = reactiveProps<TooltipHostProps>({
        presentation: record.description.presentation,
        outgoingPresentation: undefined,
        contentHidden: false,
        floating: this.#floatingAction,
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
    } else if (previous && previous !== record) {
      void this.#switchTarget(record);
    } else {
      this.#syncHost(record);
    }
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
    if (!this.#active && !this.#component) return;
    this.#active = undefined;
    this.#clearLeaveTimer();
    this.#stopPositioning();
    this.#cancelMotion();
    this.#clearContentTransition();
    const component = this.#component;
    this.#component = undefined;
    this.#hostProps = undefined;
    this.#rendered = undefined;
    this.#floatingElement = undefined;
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
    if (this.#records.size > 0 || this.#active || this.#pending || this.#warm) return;
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

    const previousContentSize = captureSize(this.#contentElement);
    const contentMotionToken = this.#prepareContentCrossfade(record, previousPresentation);
    await tick();
    if (this.#active !== record) return;
    setSize(this.#outgoingContentElement, previousContentSize);
    clearSize(this.#surfaceElement);

    const result = await this.#calculatePosition(record);
    if (!result || this.#active !== record) return;
    const destination = snapshotPosition(record, result.x, result.y, result.placement);
    const nextSurfaceSize = captureSize(this.#surfaceElement);
    const nextContentSize = captureSize(this.#contentElement);
    setSize(this.#contentElement, nextContentSize);
    setSize(this.#outgoingContentElement, previousContentSize);

    const displacement = Math.hypot(destination.x - previousPosition.x, destination.y - previousPosition.y);
    const travels = displacement <= TOOLTIP_MAX_TRAVEL_PX && destination.side === previousPosition.side;

    if (travels) {
      this.#presentHostDescription(record);
      this.#applyPosition(record, result);
      this.#startContentCrossfade(record, contentMotionToken);
      this.#startSizeTransition(previousSurfaceSize, nextSurfaceSize);
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

  #startContentCrossfade(record: TriggerRecord, token: number): void {
    const outgoingContentElement = this.#outgoingContentElement;
    const contentElement = this.#contentElement;
    if (!outgoingContentElement || !contentElement || this.#active !== record || this.#contentMotionToken !== token) {
      this.#clearContentTransition();
      return;
    }
    const outgoing = outgoingContentElement.animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: TOOLTIP_CONTENT_CROSSFADE_MS,
      easing: 'ease-out',
      fill: 'forwards',
    });
    const incoming = contentElement.animate([{ opacity: 0 }, { opacity: 1 }], {
      duration: TOOLTIP_CONTENT_CROSSFADE_MS,
      easing: 'ease-out',
      fill: 'forwards',
    });
    this.#contentAnimations = [outgoing, incoming];
    incoming.onfinish = () => void this.#finishContentCrossfade(record, token, outgoing, incoming);
  }

  async #finishContentCrossfade(record: TriggerRecord, token: number, outgoing: Animation, incoming: Animation): Promise<void> {
    if (this.#contentMotionToken !== token || !this.#contentAnimations.includes(incoming) || this.#active !== record) return;
    if (this.#hostProps) {
      this.#hostProps.contentHidden = false;
      this.#hostProps.outgoingPresentation = undefined;
    }
    await tick();
    if (this.#contentMotionToken !== token || this.#active !== record) return;
    outgoing.cancel();
    incoming.cancel();
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
