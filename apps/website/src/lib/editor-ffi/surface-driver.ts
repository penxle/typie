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

type Slot<C> = {
  canvas: C;
  removeListeners: () => void;
};

// Web surface effect boundary: owns canvas activation/lifetime, attach/recovery,
// and published-canvas promotion. Editor owns publication policy and proof acceptance.
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
    setActive(active: boolean): void {
      wantsLive = active;
      if (!active) {
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
    freeze(): void {
      // Terminal cleanup must not park the last published canvas. The Host has
      // already stopped surface work, so only the unpublished target is released.
      wantsLive = false;
      replacingUnavailableTarget = false;
      ownsHostTarget = false;
      const current = target;
      if (!current) return;
      current.removeListeners();
      target = undefined;
      if (current.canvas !== displayed) {
        effects.releaseCpuBacking(current.canvas);
        effects.removeNode(current.canvas);
      }
    },
    isAttached(): boolean {
      return target !== undefined;
    },
    hasSurface(): boolean {
      return target !== undefined || displayed !== undefined;
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
