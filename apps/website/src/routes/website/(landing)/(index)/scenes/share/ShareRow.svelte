<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { rise } from '../../scrub';

  type Props = { show: number; on: number; enter: number };

  let { show, on, enter }: Props = $props();

  const TRACK = 64;
  const THUMB = 26;
  const LABEL = '공유';

  const rowClass = flex({
    align: 'center',
    gap: { base: '10px', md: '16px' },
    height: '[40px]',
    fontFamily: 'landing',
    pointerEvents: 'none',
  });
  const labelClass = css({
    fontSize: { base: '22px', md: '28px', lg: '[32px]' },
    fontWeight: 'bold',
    letterSpacing: '[-0.02em]',
    color: 'text.default',
  });
  const trackClass = css({ position: 'relative', borderRadius: 'full', backgroundColor: 'surface.inset', overflow: 'hidden' });
  const trackOnClass = css({ position: 'absolute', inset: '0', backgroundColor: 'accent.default' });
  const thumbClass = css({
    position: 'absolute',
    top: '[3px]',
    left: '0',
    borderRadius: 'full',
    backgroundColor: 'surface.default',
    boxShadow: 'sm',
  });
</script>

<div style:opacity={show} style:transform={rise(enter, 8)} class={rowClass}>
  <span class={labelClass}>{LABEL}</span>
  <span style:width="{TRACK}px" style:height="{THUMB + 6}px" class={trackClass}>
    <span style:opacity={on} class={trackOnClass}></span>
    <span
      style:width="{THUMB}px"
      style:height="{THUMB}px"
      style:transform="translateX({3 + on * (TRACK - THUMB - 6)}px)"
      class={thumbClass}
    ></span>
  </span>
</div>
