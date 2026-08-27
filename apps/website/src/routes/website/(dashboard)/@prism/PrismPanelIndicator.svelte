<script lang="ts">
  import themeData from '@typie/assets/theme.json' with { type: 'json' };
  import { PRISM_ICON_DURATION_SECONDS } from '@typie/prism-ui';
  import { token } from '@typie/styled-system/tokens';
  import { onDestroy, onMount, untrack } from 'svelte';
  import PrismIcon from '~icons/typie/prism';
  import PrismObject from '$lib/prism-ui/PrismObject.svelte';
  import { createPrismIndicatorPath, samplePrismIndicatorPath } from './lib/prism-indicator-path.ts';
  import PrismPanelWelcomeMessage from './PrismPanelWelcomeMessage.svelte';
  import type { PrismRuntimeSnapshot, PrismTarget } from '@typie/prism-ui';
  import type { ThemeVariant } from '@typie/ui/context';
  import type { PrismIndicatorPath, PrismIndicatorPoint } from './lib/prism-indicator-path.ts';

  export type PrismIndicatorPhase = 'answered' | 'failed' | 'hidden' | 'submitting' | 'welcome';
  export type PrismSpinnerOwner = 'panel' | 'row';

  const PRISM_TO_SPINNER_DURATION_MS = 2200;
  const PRISM_TO_SPINNER_PRESENTATION_END_PROGRESS = 5 / 11;
  const WELCOME_MESSAGE_MORPH_LEAD_MS = 500;
  const WELCOME_MESSAGE_DELAY_MS = PRISM_ICON_DURATION_SECONDS * 1000 - WELCOME_MESSAGE_MORPH_LEAD_MS;

  type Props = {
    destination?: HTMLElement;
    onPrismAvailabilityChange?: (available: boolean) => void;
    onSpinnerOwnerChange?: (owner: PrismSpinnerOwner) => void;
    phase: PrismIndicatorPhase;
    prismEnabled?: boolean;
    reducedMotion?: boolean;
    themeVariant?: ThemeVariant;
    welcomeAdmission?: boolean;
  };

  type Mode = 'arrived' | 'candidate' | 'fallback' | 'idle' | 'morphing' | 'returning';

  let {
    destination,
    onPrismAvailabilityChange,
    onSpinnerOwnerChange,
    phase,
    prismEnabled = true,
    reducedMotion = typeof window !== 'undefined' &&
      typeof window.matchMedia === 'function' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches,
    themeVariant,
    welcomeAdmission = true,
  }: Props = $props();
  let actor = $state<HTMLDivElement>();
  let actorMounted = $state(true);
  let actorVisible = $state(true);
  let prismRendererMounted = $state(!reducedMotion);
  const edgeColor = $derived(themeVariant ? themeData.variants[themeVariant]['ui.border.default'] : undefined);
  let mode = $state<Mode>('idle');
  let path: PrismIndicatorPath | null = null;
  let snapshot = $state<PrismRuntimeSnapshot>({
    journeyProgress: null,
    owner: 'svg',
    readiness: 'loading',
    requestedTarget: 'icon',
    settledTarget: 'icon',
  });
  let source: HTMLSpanElement;
  let target = $state<PrismTarget>('icon');
  let targetDurationMs: number | undefined = $state();
  const interactive = $derived(!reducedMotion && mode === 'idle' && target === 'prism' && snapshot.settledTarget === 'prism');
  let dwellTimer: ReturnType<typeof setTimeout> | undefined;
  let dwellElapsed = false;
  let welcomeFrame = 0;
  let browserStable = false;
  let stabilityIdle = 0;
  let stabilityFrame = 0;
  let stabilityTimer: ReturnType<typeof setTimeout> | undefined;
  let destroyed = false;
  let admissionFrame = 0;
  let followerFrame = 0;
  let returnStartProgress = 0;
  let prismAvailable: boolean | undefined;
  let spinnerOwner: PrismSpinnerOwner | undefined;
  const welcomeMessageEligible = $derived(!prismEnabled || target === 'prism' || reducedMotion || snapshot.readiness === 'unavailable');
  let welcomeMessageAdmitted = $state(false);
  const showWelcomeMessage = $derived(phase === 'welcome' && (welcomeMessageEligible || welcomeMessageAdmitted));

  const center = (element: HTMLElement): PrismIndicatorPoint => {
    const bounds = element.getBoundingClientRect();
    return { x: bounds.left + bounds.width / 2, y: bounds.top + bounds.height / 2 };
  };

  const setSpinnerOwner = (owner: PrismSpinnerOwner) => {
    if (spinnerOwner === owner) return;
    spinnerOwner = owner;
    onSpinnerOwnerChange?.(owner);
  };

  const setPrismAvailable = (available: boolean) => {
    if (prismAvailable === available) return;
    prismAvailable = available;
    onPrismAvailabilityChange?.(available);
  };

  const clearDwell = () => {
    if (dwellTimer !== undefined) {
      clearTimeout(dwellTimer);
      dwellTimer = undefined;
    }
    if (welcomeFrame !== 0) {
      cancelAnimationFrame(welcomeFrame);
      welcomeFrame = 0;
    }
    dwellElapsed = false;
  };

  const scheduleWelcomeMorph = () => {
    if (
      phase !== 'welcome' ||
      !prismEnabled ||
      !welcomeAdmission ||
      mode !== 'idle' ||
      target !== 'icon' ||
      reducedMotion ||
      !browserStable ||
      !dwellElapsed ||
      welcomeFrame !== 0 ||
      snapshot.readiness !== 'ready'
    ) {
      return;
    }
    welcomeFrame = requestAnimationFrame(() => {
      welcomeFrame = 0;
      if (
        phase !== 'welcome' ||
        !prismEnabled ||
        !welcomeAdmission ||
        mode !== 'idle' ||
        target !== 'icon' ||
        reducedMotion ||
        !browserStable ||
        !dwellElapsed ||
        snapshot.readiness !== 'ready'
      )
        return;
      targetDurationMs = undefined;
      target = 'prism';
    });
  };

  const observeStablePaints = () => {
    if (destroyed || browserStable || stabilityFrame !== 0) return;
    stabilityFrame = requestAnimationFrame(() => {
      stabilityFrame = requestAnimationFrame(() => {
        stabilityFrame = 0;
        if (destroyed) return;
        browserStable = true;
        scheduleWelcomeMorph();
      });
    });
  };

  const observeBrowserIdle = () => {
    if (destroyed || browserStable || stabilityIdle !== 0 || stabilityTimer !== undefined) return;
    if (typeof requestIdleCallback === 'function') {
      stabilityIdle = requestIdleCallback(() => {
        stabilityIdle = 0;
        observeStablePaints();
      });
    } else {
      stabilityTimer = setTimeout(() => {
        stabilityTimer = undefined;
        observeStablePaints();
      }, 0);
    }
  };

  const observeLoadedBrowser = () => {
    const fontsReady = document.fonts?.ready;
    if (!fontsReady) {
      observeBrowserIdle();
      return;
    }
    void fontsReady.then(observeBrowserIdle).catch(observeBrowserIdle);
  };

  const cancelAdmission = () => {
    if (admissionFrame === 0) return;
    cancelAnimationFrame(admissionFrame);
    admissionFrame = 0;
  };

  const stopFollowing = () => {
    if (followerFrame === 0) return;
    cancelAnimationFrame(followerFrame);
    followerFrame = 0;
  };

  const placeAt = (point: PrismIndicatorPoint) => {
    if (!actor) return;
    const origin = center(source);
    actor.style.transform = `translate3d(${point.x - origin.x}px, ${point.y - origin.y}px, 0px)`;
  };

  const placeAtProgress = (progress: number) => {
    if (path) placeAt(samplePrismIndicatorPath(path, progress));
  };

  const presentationProgress = (progress: number) => Math.min(progress / PRISM_TO_SPINNER_PRESENTATION_END_PROGRESS, 1);

  const fallbackToRow = () => {
    cancelAdmission();
    stopFollowing();
    mode = 'fallback';
    actorVisible = false;
    if (actor) actor.style.transform = 'translate3d(0px, 0px, 0px)';
    setSpinnerOwner('row');
  };

  const settleStaticPresentation = () => {
    clearDwell();
    cancelAdmission();
    stopFollowing();
    targetDurationMs = undefined;
    target = 'icon';
    path = null;

    if (phase === 'submitting') {
      fallbackToRow();
      return;
    }
    if (phase === 'answered' || phase === 'hidden') {
      actorMounted = false;
      setSpinnerOwner('row');
      return;
    }

    mode = 'idle';
    actorVisible = true;
    if (actor) actor.style.transform = 'translate3d(0px, 0px, 0px)';
    if (spinnerOwner === 'panel') setSpinnerOwner('row');
  };

  const followDestination = () => {
    stopFollowing();
    const step = () => {
      if (mode !== 'arrived' || !destination) {
        followerFrame = 0;
        return;
      }
      try {
        path = createPrismIndicatorPath(path?.p0 ?? center(source), center(destination));
      } catch {
        fallbackToRow();
        return;
      }
      placeAt(path.p3);
      followerFrame = requestAnimationFrame(step);
    };
    followerFrame = requestAnimationFrame(step);
  };

  const arrive = () => {
    if (!path || mode !== 'morphing') return;
    placeAt(path.p3);
    mode = 'arrived';
    if (phase === 'answered') {
      actorMounted = false;
      setSpinnerOwner('row');
      return;
    }
    followDestination();
  };

  const startMorphReturn = (durationMs: number | undefined, screenProgress: number) => {
    returnStartProgress = screenProgress;
    mode = 'returning';
    targetDurationMs = durationMs;
    target = 'prism';
  };

  const admit = (nextDestination: HTMLElement) => {
    admissionFrame = 0;
    if (mode !== 'candidate' || phase !== 'submitting') return;
    if (reducedMotion || snapshot.readiness !== 'ready') {
      fallbackToRow();
      return;
    }

    try {
      path = createPrismIndicatorPath(center(source), center(nextDestination));
    } catch {
      fallbackToRow();
      return;
    }

    mode = 'morphing';
    returnStartProgress = 0;
    actorVisible = true;
    setSpinnerOwner('panel');
    targetDurationMs = PRISM_TO_SPINNER_DURATION_MS;
    target = 'spinner';
  };

  const scheduleAdmission = (nextDestination: HTMLElement | undefined) => {
    if (mode !== 'candidate' || !nextDestination || admissionFrame !== 0) return;
    admissionFrame = requestAnimationFrame(() => admit(nextDestination));
  };

  const beginSubmission = (nextDestination: HTMLElement | undefined) => {
    clearDwell();
    if (mode !== 'idle' && mode !== 'candidate') return;
    if (mode === 'idle') {
      if (!prismEnabled || reducedMotion || snapshot.readiness !== 'ready') {
        fallbackToRow();
        return;
      }
      mode = 'candidate';
      setSpinnerOwner('panel');
    }
    scheduleAdmission(nextDestination);
  };

  const fail = () => {
    clearDwell();
    cancelAdmission();

    if (mode === 'fallback' || mode === 'candidate' || mode === 'idle') {
      mode = 'idle';
      actorVisible = true;
      if (actor) actor.style.transform = 'translate3d(0px, 0px, 0px)';
      targetDurationMs = undefined;
      target = !prismEnabled || reducedMotion || snapshot.readiness === 'unavailable' ? 'icon' : 'prism';
      return;
    }

    if (mode === 'morphing') {
      const progress = Math.max(0, Math.min(1, snapshot.journeyProgress ?? 0));
      startMorphReturn(progress === 0 ? undefined : PRISM_TO_SPINNER_DURATION_MS * progress, presentationProgress(progress));
      return;
    }
    if (mode === 'arrived') {
      stopFollowing();
      startMorphReturn(2200, 1);
    }
  };

  const answer = () => {
    clearDwell();
    cancelAdmission();
    if (mode === 'morphing') return;
    stopFollowing();
    actorMounted = false;
    setSpinnerOwner('row');
  };

  const hide = () => {
    clearDwell();
    cancelAdmission();
    stopFollowing();
    actorMounted = false;
    setSpinnerOwner('row');
  };

  const handleSnapshot = (next: PrismRuntimeSnapshot) => {
    snapshot = next;
    setPrismAvailable(!reducedMotion && next.readiness === 'ready');
    if (next.readiness === 'unavailable') {
      settleStaticPresentation();
      prismRendererMounted = false;
      return;
    }
    if (!prismEnabled && next.readiness === 'ready' && next.owner === 'svg' && next.settledTarget === 'icon') {
      prismRendererMounted = false;
    }
    scheduleWelcomeMorph();
    if (mode === 'morphing') {
      const progress = next.settledTarget === 'spinner' ? 1 : Math.max(0, Math.min(1, next.journeyProgress ?? 0));
      if (phase !== 'answered' && destination && path) {
        try {
          path = createPrismIndicatorPath(path.p0, center(destination));
        } catch {
          fallbackToRow();
          return;
        }
      }
      placeAtProgress(presentationProgress(progress));
      if (progress === 1) arrive();
      return;
    }

    if (mode === 'returning') {
      const progress = next.settledTarget === 'prism' ? 1 : Math.max(0, Math.min(1, next.journeyProgress ?? 0));
      placeAtProgress(returnStartProgress * (1 - presentationProgress(progress)));
      if (progress === 1) {
        placeAtProgress(0);
        mode = 'idle';
        actorVisible = true;
      }
    }
  };

  onMount(() => {
    destroyed = false;
    if (reducedMotion) return;
    if (document.readyState === 'complete') observeLoadedBrowser();
    else window.addEventListener('load', observeLoadedBrowser, { once: true });
    if (phase === 'welcome') {
      dwellTimer = setTimeout(() => {
        dwellTimer = undefined;
        dwellElapsed = true;
        scheduleWelcomeMorph();
      }, 700);
    }
  });

  onDestroy(() => {
    destroyed = true;
    setPrismAvailable(false);
    window.removeEventListener('load', observeLoadedBrowser);
    if (stabilityIdle !== 0) cancelIdleCallback(stabilityIdle);
    if (stabilityFrame !== 0) cancelAnimationFrame(stabilityFrame);
    if (stabilityTimer !== undefined) clearTimeout(stabilityTimer);
    clearDwell();
    cancelAdmission();
    stopFollowing();
  });

  $effect(() => {
    const useStaticPresentation = reducedMotion;
    setPrismAvailable(!useStaticPresentation && snapshot.readiness === 'ready');
    if (useStaticPresentation) {
      prismRendererMounted = false;
      untrack(settleStaticPresentation);
    } else if (prismEnabled && snapshot.readiness !== 'unavailable') {
      prismRendererMounted = true;
    }
  });

  $effect(() => {
    if (phase !== 'welcome') welcomeMessageAdmitted = false;
    else if (welcomeMessageEligible) welcomeMessageAdmitted = true;
  });

  $effect(() => {
    const enabled = prismEnabled;
    const admitted = welcomeAdmission;
    untrack(() => {
      if (!enabled) {
        const alreadySettledAtIcon = target === 'icon' && snapshot.owner === 'svg' && snapshot.settledTarget === 'icon';
        targetDurationMs = undefined;
        target = 'icon';
        if (alreadySettledAtIcon && snapshot.readiness !== 'loading') prismRendererMounted = false;
      } else if (admitted) {
        prismRendererMounted = true;
        scheduleWelcomeMorph();
      }
    });
  });

  $effect(() => {
    const nextPhase = phase;
    const nextDestination = destination;
    untrack(() => {
      if (nextPhase === 'submitting') beginSubmission(nextDestination);
      else if (nextPhase === 'failed') fail();
      else if (nextPhase === 'answered') answer();
      else if (nextPhase === 'hidden') hide();
    });
  });
</script>

<div class="indicator" aria-hidden="true">
  <span bind:this={source} class="source" data-prism-indicator-source>
    {#if actorMounted}
      <div
        bind:this={actor}
        style:color={token('colors.border.default')}
        style:visibility={actorVisible ? undefined : 'hidden'}
        style:transform="translate3d(0px, 0px, 0px)"
        class="actor"
        data-prism-indicator-actor
      >
        {#if prismRendererMounted}
          <PrismObject
            {edgeColor}
            {interactive}
            onStateChange={handleSnapshot}
            preload={!reducedMotion}
            {reducedMotion}
            {target}
            {targetDurationMs}
          />
        {:else}
          <PrismIcon aria-hidden="true" data-prism-indicator-static-icon height="44" width="44" />
        {/if}
      </div>
    {/if}
  </span>
</div>

<PrismPanelWelcomeMessage
  delayMs={snapshot.readiness === 'unavailable' ? 0 : WELCOME_MESSAGE_DELAY_MS}
  immediate={reducedMotion || !prismEnabled}
  visible={showWelcomeMessage}
/>

<style>
  .indicator {
    position: absolute;
    inset: 0;
    z-index: 1;
    overflow: hidden;
    pointer-events: none;
  }

  .source {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 0;
    height: 0;
  }

  .actor {
    position: absolute;
    top: -160px;
    left: -160px;
    display: grid;
    width: 320px;
    height: 320px;
    place-items: center;
    will-change: transform;
  }
</style>
