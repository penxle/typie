<script lang="ts">
  import { prefersReducedMotion } from '@typie/ui/state/reduced-motion';
  import { onDestroy } from 'svelte';
  import type { Snippet } from 'svelte';
  import type { NotePresence } from './note-list-state.svelte';

  type Props = {
    presence: NotePresence;
    gap?: string;
    children: Snippet;
    onentercomplete?: () => void;
    onexitcomplete?: () => void;
  };

  let { presence, gap = '0px', children, onentercomplete, onexitcomplete }: Props = $props();

  let collapsed = $state(presence === 'entering');
  let opacity = $state(presence === 'entering' ? 0 : 1);
  let transitioning = $state<Exclude<NotePresence, 'settled'> | null>(null);
  let activePresence: NotePresence | null = null;
  let cancelActiveTransition: (() => void) | null = null;

  const complete = (kind: Exclude<NotePresence, 'settled'>) => {
    if (transitioning !== kind) return;

    transitioning = null;
    if (kind === 'entering') onentercomplete?.();
    else onexitcomplete?.();
  };

  $effect(() => {
    const nextPresence = presence;
    if (nextPresence === activePresence) return;

    activePresence = nextPresence;
    cancelActiveTransition?.();
    let frame: number | null = null;
    let exitTimer: ReturnType<typeof setTimeout> | null = null;
    let completionTimer: ReturnType<typeof setTimeout> | null = null;
    let cancelled = false;
    const reducedMotion = prefersReducedMotion.current;

    transitioning = null;

    if (nextPresence === 'settled') {
      collapsed = false;
      opacity = 1;
    } else if (reducedMotion) {
      transitioning = nextPresence;
      collapsed = nextPresence === 'exiting';
      opacity = nextPresence === 'exiting' ? 0 : 1;
      queueMicrotask(() => {
        if (!cancelled) complete(nextPresence);
      });
    } else if (nextPresence === 'entering') {
      collapsed = true;
      opacity = 0;
      frame = requestAnimationFrame(() => {
        if (cancelled) return;
        transitioning = 'entering';
        collapsed = false;
        opacity = 1;
        completionTimer = setTimeout(() => {
          if (!cancelled) complete('entering');
        }, 220);
      });
    } else {
      collapsed = false;
      opacity = 1;
      exitTimer = setTimeout(() => {
        if (cancelled) return;
        transitioning = 'exiting';
        collapsed = true;
        opacity = 0;
        completionTimer = setTimeout(() => {
          if (!cancelled) complete('exiting');
        }, 180);
      }, 250);
    }

    cancelActiveTransition = () => {
      if (cancelled) return;
      cancelled = true;
      if (frame !== null) cancelAnimationFrame(frame);
      if (exitTimer !== null) clearTimeout(exitTimer);
      if (completionTimer !== null) clearTimeout(completionTimer);
    };
  });

  onDestroy(() => cancelActiveTransition?.());
</script>

<div
  style:display="grid"
  style:grid-template-rows={collapsed ? '0fr' : '1fr'}
  style:margin-bottom={collapsed && presence !== 'settled' ? `calc(-1 * ${gap})` : '0px'}
  style:opacity
  style:transition={transitioning === 'entering'
    ? 'grid-template-rows 220ms ease, margin-bottom 220ms ease, opacity 220ms ease'
    : transitioning === 'exiting'
      ? 'grid-template-rows 180ms ease, margin-bottom 180ms ease, opacity 180ms ease'
      : 'none'}
>
  <div style:min-height="0" style:overflow={transitioning === null ? 'visible' : 'hidden'}>
    {@render children()}
  </div>
</div>
