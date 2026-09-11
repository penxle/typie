<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { prefersReducedMotion } from '@typie/ui/state';
  import Caret from '$lib/landing/components/Caret.svelte';

  type Props = { done?: boolean };

  let { done = $bindable(false) }: Props = $props();

  const lines = ['쓰고', '싶어질 때,', '타이피'] as const;
  const total = lines.reduce((sum, line) => sum + line.length, 0);
  const before = lines.map((_, index) => lines.slice(0, index).reduce((sum, line) => sum + line.length, 0));
  const breaks = new Set(before.slice(1));
  const START_MS = 420;
  const STEP_MS = 70;
  const LINE_PAUSE_MS = 180;

  let typed = $state(0);

  const reduced = $derived(prefersReducedMotion.current);
  const shown = $derived(reduced ? total : typed);
  const parts = $derived(lines.map((line, index) => line.slice(0, Math.min(line.length, Math.max(0, shown - before[index])))));
  const active = $derived(lines.findIndex((line, index) => shown <= before[index] + line.length));

  $effect(() => {
    done = shown >= total;
  });

  $effect(() => {
    if (reduced) return;
    let timer = 0;
    const tick = () => {
      if (typed >= total) return;
      typed += 1;
      timer = window.setTimeout(tick, breaks.has(typed) ? LINE_PAUSE_MS : STEP_MS);
    };
    timer = window.setTimeout(tick, START_MS);
    return () => clearTimeout(timer);
  });

  const caretHostClass = css({
    display: 'inline-block',
    width: '0',
    transition: '[opacity 320ms ease-out]',
    '&[data-done="true"]': { opacity: '0' },
    _motionReduce: { transition: '[none]' },
  });
  const headlineClass = css({
    marginTop: '[-0.131em]',
    marginBottom: '[-0.1275em]',
    minHeight: { base: '[3.45em]', md: '[2.3em]' },
    textAlign: 'center',
    fontSize: { base: '[64px]', md: '[64px]', xl: '[88px]' },
    fontWeight: 'bold',
    lineHeight: '[1.15]',
    letterSpacing: '[-0.025em]',
  });
  const softBreakClass = css({ display: { base: 'inline', md: 'none' } });
</script>

<h1 class={headlineClass}>
  {parts[0]}{#if active === 0 && !done}<Caret centered={parts[0].length === 0} variant="block" />{/if}
  <br class={softBreakClass} />
  {parts[1]}{#if active === 1 && !done}<Caret variant="block" />{/if}
  <br />
  {parts[2]}{#if active === 2}<span class={caretHostClass} data-done={done}><Caret blinking={!done} variant="block" /></span>{/if}
</h1>
