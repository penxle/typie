export type AttachResult = 'none' | 'cpu' | 'cpu-oversized';

export type SurfaceDriverEffects<C> = {
  createCanvas: () => C;
  styleCanvas: (canvas: C) => void;
  attach: (canvas: C) => AttachResult;
  detach: () => void;
  recover: () => void;
  addContextListeners: (canvas: C, isCurrent: () => boolean) => () => void;
  releaseCpuBacking: (canvas: C) => void;
  promote: (next: C, previous: C | undefined) => void;
  removeNode: (canvas: C) => void;
  replacementFailed: () => void;
};

export type VisibilityState = { inAcquire: boolean; inRelease: boolean };

type Slot<C> = {
  canvas: C;
  removeListeners: () => void;
};

// Owns canvas lifetime, visibility, and delivery only. Editor owns publication facts,
// requirements, proofs, waiters, and scheduling.
export function createSurfaceDriver<C>(effects: SurfaceDriverEffects<C>) {
  let target: Slot<C> | undefined;
  let displayed: C | undefined;
  let wantsLive = false;
  let ownsHostTarget = false;
  let replacingUnavailableTarget = false;

  const disposeCanvas = (canvas: C) => {
    if (target?.canvas === canvas) {
      target.removeListeners();
      target = undefined;
    }
    effects.releaseCpuBacking(canvas);
    effects.removeNode(canvas);
    if (displayed === canvas) displayed = undefined;
  };

  const mount = (replaceCurrent = false) => {
    const previousSlot = target;
    const previousTarget = previousSlot?.canvas;
    const previousDisplayed = displayed;
    if (replaceCurrent && previousSlot) {
      if (previousTarget === previousDisplayed) {
        previousSlot.removeListeners();
        target = undefined;
      } else {
        disposeCanvas(previousSlot.canvas);
      }
    }

    const canvas = effects.createCanvas();
    effects.styleCanvas(canvas);
    const slot: Slot<C> = {
      canvas,
      removeListeners: effects.addContextListeners(canvas, () => target?.canvas === canvas),
    };
    target = slot;

    const result = effects.attach(canvas);
    ownsHostTarget = result === 'cpu';
    if (result !== 'cpu') {
      if (result === 'cpu-oversized') effects.detach();
      ownsHostTarget = false;
      disposeCanvas(canvas);
      if (result === 'cpu-oversized') {
        if (replacingUnavailableTarget) effects.replacementFailed();
        else replacingUnavailableTarget = true;
      }
    }
  };

  const park = () => {
    if (ownsHostTarget) {
      effects.detach();
      ownsHostTarget = false;
    }
    if (target) disposeCanvas(target.canvas);
    if (displayed) disposeCanvas(displayed);
    replacingUnavailableTarget = false;
  };

  return {
    reconcile(state: VisibilityState): void {
      const shouldBeLive = target || displayed ? state.inRelease : state.inAcquire;
      wantsLive = shouldBeLive;
      if (!shouldBeLive) {
        park();
      } else if (!target) {
        mount();
      }
    },
    syncPublished(canvas: C | undefined): void {
      if (!canvas || canvas !== target?.canvas) return;
      replacingUnavailableTarget = false;
      if (displayed === canvas) {
        effects.promote(canvas, displayed);
        return;
      }
      const previous = displayed;
      effects.promote(canvas, previous);
      displayed = canvas;
      if (previous && previous !== canvas) disposeCanvas(previous);
    },
    resume(): void {
      if (!wantsLive) return;
      if (target) {
        effects.recover();
      } else {
        mount();
      }
    },
    restyle(): void {
      if (target) effects.styleCanvas(target.canvas);
      if (displayed && displayed !== target?.canvas) effects.styleCanvas(displayed);
    },
    replace(): void {
      if (!wantsLive) return;
      mount(true);
    },
    isAttached(): boolean {
      return target !== undefined;
    },
    destroy(): void {
      wantsLive = false;
      park();
    },
    debug() {
      return { target: target?.canvas, displayed, wantsLive };
    },
  };
}
