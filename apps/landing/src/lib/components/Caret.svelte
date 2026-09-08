<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = { variant?: 'line' | 'block'; centered?: boolean; blinking?: boolean };

  let { variant = 'line', centered = false, blinking = true }: Props = $props();

  const caretClass = css({
    position: 'relative',
    display: 'inline-block',
    width: '0',
    '&[data-variant="line"]': { height: '[1em]', verticalAlign: '[-0.15em]' },
    '&[data-variant="block"]': { height: '[0.9em]', verticalAlign: '[-0.09em]' },
  });
  const barClass = css({
    position: 'absolute',
    top: '0',
    bottom: '0',
    left: '0',
    backgroundColor: 'text.default',
    animation: '[blink 1s step-end infinite]',
    _motionReduce: { animation: '[none]' },
    '[data-blinking="false"] &': { animation: '[none]' },
    '[data-variant="line"] &': { width: '1px' },
    '[data-variant="block"] &': { width: '[0.5em]' },
    '[data-centered="true"] &': { transform: '[translateX(-50%)]' },
  });
</script>

<span class={caretClass} aria-hidden="true" data-blinking={blinking} data-centered={centered} data-variant={variant}>
  <span class={barClass}></span>
</span>
