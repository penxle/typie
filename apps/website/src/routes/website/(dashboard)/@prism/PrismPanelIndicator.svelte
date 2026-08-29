<script lang="ts">
  import themeData from '@typie/assets/theme.json' with { type: 'json' };
  import { PRISM_ICON_DURATION_SECONDS } from '@typie/prism-ui';
  import { token } from '@typie/styled-system/tokens';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { onDestroy, onMount, untrack } from 'svelte';
  import PrismIcon from '~icons/typie/prism';
  import PrismObject from '$lib/prism-ui/PrismObject.svelte';
  import { createPrismIndicatorPath, samplePrismIndicatorPath } from './lib/prism-indicator-path.ts';
  import PrismPanelWelcomeMessage from './PrismPanelWelcomeMessage.svelte';
  import type { PrismRuntimeSnapshot, PrismTarget } from '@typie/prism-ui';
  import type { ThemeVariant } from '@typie/ui/context';
  import type { PrismIndicatorPath, PrismIndicatorPoint } from './lib/prism-indicator-path.ts';

  export type PrismIndicatorPhase = 'answered' | 'failed' | 'hidden' | 'submitting' | 'welcome';

  const PRISM_TO_SPINNER_DURATION_MS = 2200;
  const PRISM_TO_SPINNER_PRESENTATION_END_PROGRESS = 5 / 11;
  const WELCOME_MESSAGE_MORPH_LEAD_MS = 500;
  const WELCOME_MESSAGE_DELAY_MS = PRISM_ICON_DURATION_SECONDS * 1000 - WELCOME_MESSAGE_MORPH_LEAD_MS;

  type Props = {
    destination?: HTMLElement;
    phase: PrismIndicatorPhase;
    prismEnabled?: boolean;
    reducedMotion?: boolean;
    rowSpinnerPlaybackStartedAt?: number | null;
    themeVariant?: ThemeVariant;
    welcomeAdmission?: boolean;
  };

  type Mode = 'arrived' | 'candidate' | 'fallback' | 'idle' | 'morphing';

  let {
    destination,
    phase,
    prismEnabled = true,
    reducedMotion = untrack(() => prefersReducedMotion.current),
    rowSpinnerPlaybackStartedAt,
    themeVariant,
    welcomeAdmission = true,
  }: Props = $props();
  let actor = $state<HTMLDivElement>();
  let actorMounted = $state(true);
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
  let targetSpinnerPlaybackStartedAt: number | undefined = $state();
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
  let hiddenRowSpinner: HTMLElement | undefined;
  const welcomeMessageEligible = $derived(!prismEnabled || target === 'prism' || reducedMotion || snapshot.readiness === 'unavailable');
  let welcomeMessageAdmitted = $state(false);
  const showWelcomeMessage = $derived(phase === 'welcome' && (welcomeMessageEligible || welcomeMessageAdmitted));

  const center = (element: HTMLElement): PrismIndicatorPoint => {
    const bounds = element.getBoundingClientRect();
    return { x: bounds.left + bounds.width / 2, y: bounds.top + bounds.height / 2 };
  };

  const hideRowSpinner = (element: HTMLElement | undefined) => {
    if (!element || hiddenRowSpinner === element) return;
    hiddenRowSpinner?.style.removeProperty('opacity');
    hiddenRowSpinner = element;
    hiddenRowSpinner.style.setProperty('opacity', '0');
  };

  const showRowSpinner = () => {
    hiddenRowSpinner?.style.removeProperty('opacity');
    hiddenRowSpinner = undefined;
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
    actorMounted = false;
    if (actor) actor.style.transform = 'translate3d(0px, 0px, 0px)';
    showRowSpinner();
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
      hideRowSpinner(destination);
      return;
    }

    mode = 'idle';
    if (actor) actor.style.transform = 'translate3d(0px, 0px, 0px)';
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
    followDestination();
  };

  const admit = (nextDestination: HTMLElement) => {
    admissionFrame = 0;
    if (mode !== 'candidate' || phase !== 'submitting') return;
    if (reducedMotion || snapshot.readiness !== 'ready') {
      fallbackToRow();
      return;
    }

    const playbackStartedAt = rowSpinnerPlaybackStartedAt;
    if (playbackStartedAt === undefined) {
      return;
    }
    if (playbackStartedAt === null) {
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
    targetSpinnerPlaybackStartedAt = playbackStartedAt;
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
      if (!actorMounted || !prismEnabled || reducedMotion || snapshot.readiness !== 'ready') {
        fallbackToRow();
        return;
      }
      mode = 'candidate';
    }
    scheduleAdmission(nextDestination);
  };

  const finishSubmission = () => {
    clearDwell();
    cancelAdmission();
    stopFollowing();
    targetDurationMs = undefined;
    targetSpinnerPlaybackStartedAt = undefined;
    path = null;
    mode = 'idle';
    actorMounted = false;
    hideRowSpinner(destination);
  };

  const completeHandoff = () => {
    stopFollowing();
    actorMounted = false;
    showRowSpinner();
  };

  const handleSnapshot = (next: PrismRuntimeSnapshot) => {
    snapshot = next;
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
      if (progress === 1) {
        if (next.settledTarget === 'spinner') completeHandoff();
        else arrive();
      }
      return;
    }
    if (mode === 'arrived' && next.settledTarget === 'spinner') completeHandoff();
    if (mode === 'candidate') scheduleAdmission(destination);
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
    window.removeEventListener('load', observeLoadedBrowser);
    if (stabilityIdle !== 0) cancelIdleCallback(stabilityIdle);
    if (stabilityFrame !== 0) cancelAnimationFrame(stabilityFrame);
    if (stabilityTimer !== undefined) clearTimeout(stabilityTimer);
    clearDwell();
    cancelAdmission();
    stopFollowing();
    showRowSpinner();
  });

  $effect(() => {
    const useStaticPresentation = reducedMotion;
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
    if (nextPhase !== 'submitting') {
      if (nextPhase !== 'welcome') untrack(finishSubmission);
      return;
    }
    const nextDestination = destination;
    const nextRowSpinnerPlaybackStartedAt = rowSpinnerPlaybackStartedAt;
    untrack(() => {
      if (mode !== 'fallback') hideRowSpinner(nextDestination);
      if (nextRowSpinnerPlaybackStartedAt === null) {
        fallbackToRow();
        return;
      }
      beginSubmission(nextDestination);
    });
  });
</script>

<div class="indicator" aria-hidden="true">
  <span bind:this={source} class="source" data-prism-indicator-source>
    {#if actorMounted}
      <div
        bind:this={actor}
        style:color={token('colors.border.default')}
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
            spinnerPlaybackStartedAt={targetSpinnerPlaybackStartedAt}
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
