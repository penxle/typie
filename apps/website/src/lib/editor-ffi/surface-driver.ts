export type AttachResult = 'none' | 'cpu' | 'cpu-oversized';

export type SurfaceDriverEffects<C> = {
  createSurface: () => C;
  styleSurface: (surface: C) => void;
  attach: (surface: C) => AttachResult;
  detach: () => void;
  recover: () => void;
  addContextListeners: (surface: C, isCurrent: () => boolean) => () => void;
  releaseCpuBacking: (surface: C) => void;
  promote: (next: C, previous: C | undefined) => void;
  removeNode: (surface: C) => void;
  replacementFailed: () => void;
};

type Slot<C> = {
  surface: C;
  removeListeners: () => void;
};

// Web surface effect boundary: owns surface activation/lifetime, attach/recovery,
// and published-surface promotion. Editor owns publication policy and proof acceptance.
export function createSurfaceDriver<C>(effects: SurfaceDriverEffects<C>) {
  let target: Slot<C> | undefined;
  let displayed: C | undefined;
  let wantsLive = false;
  let ownsHostTarget = false;
  let replacingUnavailableTarget = false;

  const disposeSurface = (surface: C) => {
    if (target?.surface === surface) {
      target.removeListeners();
      target = undefined;
    }
    effects.releaseCpuBacking(surface);
    effects.removeNode(surface);
    if (displayed === surface) displayed = undefined;
  };

  const mount = (replaceCurrent = false) => {
    const previousSlot = target;
    const previousTarget = previousSlot?.surface;
    const previousDisplayed = displayed;
    if (replaceCurrent && previousSlot) {
      if (previousTarget === previousDisplayed) {
        previousSlot.removeListeners();
        target = undefined;
      } else {
        disposeSurface(previousSlot.surface);
      }
    }

    const surface = effects.createSurface();
    effects.styleSurface(surface);
    const slot: Slot<C> = {
      surface,
      removeListeners: effects.addContextListeners(surface, () => target?.surface === surface),
    };
    target = slot;

    const result = effects.attach(surface);
    ownsHostTarget = result === 'cpu';
    if (result !== 'cpu') {
      if (result === 'cpu-oversized') effects.detach();
      ownsHostTarget = false;
      disposeSurface(surface);
      if (result === 'cpu-oversized') {
        if (replacingUnavailableTarget) effects.replacementFailed();
        else replacingUnavailableTarget = true;
      }
    }
  };

  const deactivate = () => {
    wantsLive = false;
    if (ownsHostTarget) effects.detach();
    ownsHostTarget = false;
    const current = target;
    if (current) {
      current.removeListeners();
      target = undefined;
      if (current.surface !== displayed) {
        effects.releaseCpuBacking(current.surface);
        effects.removeNode(current.surface);
      }
    }
    replacingUnavailableTarget = false;
  };

  const disposeAll = () => {
    deactivate();
    if (displayed) disposeSurface(displayed);
  };

  return {
    setActive(active: boolean): void {
      wantsLive = active;
      if (!active) {
        deactivate();
      } else if (!target) {
        mount();
      }
    },
    syncPublished(surface: C | undefined): void {
      if (!surface || surface !== target?.surface) return;
      replacingUnavailableTarget = false;
      if (displayed === surface) {
        effects.promote(surface, displayed);
        return;
      }
      const previous = displayed;
      effects.promote(surface, previous);
      displayed = surface;
      if (previous && previous !== surface) disposeSurface(previous);
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
      if (target) effects.styleSurface(target.surface);
      if (displayed && displayed !== target?.surface) effects.styleSurface(displayed);
    },
    replace(): void {
      if (!wantsLive) return;
      mount(true);
    },
    freeze(): void {
      // Terminal cleanup must not park the last published surface. The Host has
      // already stopped surface work, so only the unpublished target is released.
      wantsLive = false;
      replacingUnavailableTarget = false;
      ownsHostTarget = false;
      const current = target;
      if (!current) return;
      current.removeListeners();
      target = undefined;
      if (current.surface !== displayed) {
        effects.releaseCpuBacking(current.surface);
        effects.removeNode(current.surface);
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
      disposeAll();
    },
    debug() {
      return { target: target?.surface, displayed, wantsLive };
    },
  };
}
