<script lang="ts">
  // cspell:ignore onrestart

  import { css } from '@typie/styled-system/css';
  import { prefersReducedMotion } from '@typie/ui/state/reduced-motion';
  import { cubicOut } from 'svelte/easing';
  import { fade, scale } from 'svelte/transition';
  import CircleArrowUpIcon from '~icons/lucide/circle-arrow-up';

  type Props = { onrestart: () => void };
  let { onrestart }: Props = $props();

  const enter = (node: Element) =>
    prefersReducedMotion.current
      ? fade(node, { duration: 150 })
      : scale(node, { duration: 180, start: 0.95, opacity: 0, easing: cubicOut });
</script>

<button
  style:-webkit-app-region="no-drag"
  class={css({
    display: 'flex',
    alignItems: 'center',
    gap: '5px',
    flexShrink: '0',
    height: '24px',
    marginY: '8px',
    paddingLeft: '8px',
    paddingRight: '10px',
    borderRadius: 'full',
    fontSize: '12px',
    fontWeight: 'semibold',
    color: 'text.default',
    boxShadow: '[inset 0 0 0 1px {colors.border.default}]',
    whiteSpace: 'nowrap',
    transitionProperty: '[background-color, color, box-shadow, transform]',
    transitionDuration: '[120ms]',
    _hover: { color: 'surface.default', backgroundColor: 'accent.default' },
    _active: { transform: 'scale(0.97)' },
    _focusVisible: { boxShadow: '[0 0 0 2px {colors.accent.default}]' },
  })}
  onclick={onrestart}
  title="새 버전이 준비됐어요. 지금 재시작하거나, 다음에 앱을 종료할 때 자동으로 적용돼요."
  type="button"
  in:enter
>
  <CircleArrowUpIcon class={css({ size: '13px' })} />
  업데이트
</button>
