import { arrow, autoUpdate, computePosition, detectOverflow, flip, offset, shift, size } from '@floating-ui/dom';
import { on } from 'svelte/events';
import { match } from 'ts-pattern';
import type {
  Derivable,
  DetectOverflowOptions,
  FloatingElement,
  Middleware,
  MiddlewareState,
  OffsetOptions,
  Placement,
  Rect,
  ReferenceElement,
  SideObject,
} from '@floating-ui/dom';
import type { Action } from 'svelte/action';

export type ReferenceAction = Action<ReferenceElement>;
export type FloatingAction = Action<FloatingElement, { appendTo?: Element | null } | undefined>;
export type ArrowAction = Action<HTMLElement>;
export type UpdatePosition = () => Promise<void>;

type PlacementRect = Pick<Rect, 'x' | 'y' | 'width' | 'height'>;
type PlacementSize = Pick<Rect, 'width' | 'height'>;

type CenteredFallbackInput = {
  reference: PlacementRect;
  floating: PlacementSize;
  clippingRect: PlacementRect;
  gap: number;
};

type CenterWhenReferenceDoesNotFitOptions = {
  gap: number;
  overflow?: DetectOverflowOptions | Derivable<DetectOverflowOptions>;
};

type CenterWhenReferenceDoesNotFitMiddleware = {
  captureReferenceBounds: Middleware;
  centerWhenNeitherSideFits: Middleware;
};

const REFERENCE_BOUNDS_MIDDLEWARE = 'referenceBoundsForCenteredFallback';
const VIEWPORT_PADDING = 8;

export const FLOATING_AVAILABLE_WIDTH_VAR = '--floating-available-width';
export const FLOATING_AVAILABLE_HEIGHT_VAR = '--floating-available-height';

const createDefaultMiddleware = (): Middleware[] => [
  shift({ padding: VIEWPORT_PADDING }),
  flip({ padding: VIEWPORT_PADDING }),
  size({
    padding: VIEWPORT_PADDING,
    apply({ availableWidth, availableHeight, elements }) {
      elements.floating.style.setProperty(FLOATING_AVAILABLE_WIDTH_VAR, `${Math.max(0, availableWidth)}px`);
      elements.floating.style.setProperty(FLOATING_AVAILABLE_HEIGHT_VAR, `${Math.max(0, availableHeight)}px`);
    },
  }),
];

export function resolveFloatingCenteredFallback({
  reference,
  floating,
  clippingRect,
  gap,
}: CenteredFallbackInput): { x: number; y: number } | null {
  const requiredSideSpace = floating.height + gap;
  const fitsAbove = reference.y - clippingRect.y >= requiredSideSpace;
  const fitsBelow = clippingRect.y + clippingRect.height - (reference.y + reference.height) >= requiredSideSpace;
  if (fitsAbove || fitsBelow) return null;

  return {
    x: Math.max(clippingRect.x, clippingRect.x + (clippingRect.width - floating.width) / 2),
    y: Math.max(clippingRect.y, clippingRect.y + (clippingRect.height - floating.height) / 2),
  };
}

function clippingRectFromOverflow(state: MiddlewareState, overflow: SideObject): PlacementRect {
  return {
    x: state.x + overflow.left,
    y: state.y + overflow.top,
    width: state.rects.floating.width - overflow.left - overflow.right,
    height: state.rects.floating.height - overflow.top - overflow.bottom,
  };
}

export function createCenterWhenReferenceDoesNotFitMiddleware({
  gap,
  overflow: overflowOptions,
}: CenterWhenReferenceDoesNotFitOptions): CenterWhenReferenceDoesNotFitMiddleware {
  return {
    captureReferenceBounds: {
      name: REFERENCE_BOUNDS_MIDDLEWARE,
      async fn({ elements, platform, strategy }) {
        const rects = await platform.getElementRects({ reference: elements.reference, floating: elements.floating, strategy });
        return { data: { reference: rects.reference } };
      },
    },
    centerWhenNeitherSideFits: {
      name: 'centerWhenReferenceDoesNotFit',
      async fn(state) {
        const reference = (state.middlewareData[REFERENCE_BOUNDS_MIDDLEWARE] as { reference?: PlacementRect } | undefined)?.reference;
        if (!reference) return {};

        const overflow = await detectOverflow(state, overflowOptions);
        const position = resolveFloatingCenteredFallback({
          reference,
          floating: state.rects.floating,
          clippingRect: clippingRectFromOverflow(state, overflow),
          gap,
        });
        return position ?? {};
      },
    },
  };
}

type CreateFloatingActionsOptions = {
  placement: Placement;
  offset?: OffsetOptions;
  arrow?: boolean;
  middleware?: Middleware[];
  disableAutoUpdate?: boolean;
  onClickOutside?: (event: Event) => void;
};

type CreateFloatingActionsReturn = {
  anchor: ReferenceAction;
  floating: FloatingAction;
  arrow: ArrowAction;
  update: UpdatePosition;
};

export function createFloatingActions(options?: CreateFloatingActionsOptions): CreateFloatingActionsReturn {
  let referenceElement: ReferenceElement | undefined;
  let floatingElement: FloatingElement | undefined;
  let arrowElement: HTMLElement | undefined;
  let cleanupAutoUpdate: (() => void) | undefined;
  let cleanupClickHandler: (() => void) | undefined;

  const updatePosition: UpdatePosition = async () => {
    const reference = referenceElement;
    const floating = floatingElement;
    const arrowEl = arrowElement;
    if (!reference || !floating) {
      return;
    }

    const middleware = options?.middleware ?? createDefaultMiddleware();

    const { x, y, placement, strategy, middlewareData } = await computePosition(reference, floating, {
      strategy: 'absolute',
      placement: options?.placement,
      middleware: [
        !!options?.offset && offset(options.offset),
        ...middleware,
        !!options?.arrow && arrowEl && arrow({ element: arrowEl, padding: 16 }),
      ],
    });

    if (referenceElement !== reference || floatingElement !== floating || arrowElement !== arrowEl) {
      return;
    }

    Object.assign(floating.style, {
      position: strategy,
      top: `${y}px`,
      left: `${x}px`,
    });

    if (middlewareData.hide) {
      const isHidden = middlewareData.hide.referenceHidden || middlewareData.hide.escaped;
      Object.assign(floating.style, {
        visibility: isHidden ? 'hidden' : 'visible',
        // Reset position when hidden to prevent overflow
        top: isHidden ? '0' : `${y}px`,
        left: isHidden ? '0' : `${x}px`,
      });
    }

    if (arrowEl && middlewareData.arrow) {
      const { x, y } = middlewareData.arrow;

      const side = match(placement)
        .with('top', 'top-start', 'top-end', () => 'bottom')
        .with('bottom', 'bottom-start', 'bottom-end', () => 'top')
        .with('left', 'left-start', 'left-end', () => 'right')
        .with('right', 'right-start', 'right-end', () => 'left')
        .exhaustive();

      const transform = match(placement)
        .with('top', 'top-start', 'top-end', () => 'rotate(-135deg)')
        .with('bottom', 'bottom-start', 'bottom-end', () => 'rotate(45deg)')
        .with('left', 'left-start', 'left-end', () => 'rotate(135deg)')
        .with('right', 'right-start', 'right-end', () => 'rotate(-45deg)')
        .exhaustive();

      Object.assign(arrowEl.style, {
        left: x === undefined ? '' : `${x}px`,
        top: y === undefined ? '' : `${y}px`,
        [side]: `${-arrowEl.offsetHeight / 2}px`,
        transform,
        visibility: middlewareData.bubble?.bubbled ? 'hidden' : 'visible',
      });
    }
  };

  const handleClick = (event: Event) => {
    if (event.target instanceof Node && !event.target.isConnected) {
      return;
    }

    if (event.target instanceof Element && event.target.closest('[data-floating-keep-open]')) {
      return;
    }

    // NOTE: 메뉴 내부 클릭 무시
    if (event.target instanceof Element && event.target.closest('[role="menu"]')) {
      return;
    }

    // NOTE: 참조 요소를 포함하지 않는 외부 포탈 내부 클릭 무시
    if (event.target instanceof Element) {
      const portalElement = event.target.closest('[data-portal]');
      if (portalElement && !portalElement.contains(referenceElement as Node)) {
        return;
      }
    }

    if (options?.onClickOutside && !floatingElement?.contains(event.target as Node)) {
      options.onClickOutside(event);
    }
  };

  const mount = async () => {
    const reference = referenceElement;
    const floating = floatingElement;
    const arrowEl = arrowElement;
    if (!reference || !floating) {
      return;
    }

    await updatePosition();
    if (referenceElement !== reference || floatingElement !== floating || arrowElement !== arrowEl) {
      return;
    }

    if (options?.disableAutoUpdate !== true) {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = autoUpdate(reference, floating, updatePosition, { animationFrame: true });
    }

    setTimeout(() => {
      if (referenceElement !== reference || floatingElement !== floating || arrowElement !== arrowEl) {
        return;
      }

      cleanupClickHandler?.();

      cleanupClickHandler = on(window, 'click', handleClick);
    }, 0);
  };

  const unmount = () => {
    if (cleanupAutoUpdate) {
      cleanupAutoUpdate();
      cleanupAutoUpdate = undefined;
    }

    cleanupClickHandler?.();
    cleanupClickHandler = undefined;
  };

  const referenceAction: ReferenceAction = (element) => {
    $effect(() => {
      referenceElement = element;
      mount();

      return () => {
        unmount();
        referenceElement = undefined;
      };
    });
  };

  const floatingAction: FloatingAction = (element, options = {}) => {
    $effect(() => {
      if (options.appendTo) {
        options.appendTo.append(element);
      } else {
        // NOTE: top layer에 표시되는 조상 요소가 있다면 그 요소에 추가해서 floating element와 상호작용이 되도록 함
        const topLayerElem = element.closest('dialog, [popover]');
        if (topLayerElem) {
          topLayerElem.append(element);
        } else {
          document.body.append(element);
        }
      }

      Object.assign(element.style, {
        position: 'absolute',
        top: '0',
        left: '0',
      });

      floatingElement = element;
      mount();

      return () => {
        unmount();
        floatingElement?.remove();
        floatingElement = undefined;
      };
    });
  };

  const arrowAction: ArrowAction = (element) => {
    $effect(() => {
      Object.assign(element.style, {
        position: 'absolute',
      });

      arrowElement = element;
      mount();

      return () => {
        unmount();
        arrowElement = undefined;
      };
    });
  };

  return {
    anchor: referenceAction,
    floating: floatingAction,
    arrow: arrowAction,
    update: updatePosition,
  };
}
