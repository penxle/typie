<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';
  import type { Device } from './device';

  type Props = { device: Device; anchor?: Device; target?: Device; decorations?: Snippet; aside?: Snippet; children: Snippet };

  let { device, anchor, target, decorations, aside, children }: Props = $props();

  const LEAD_GAP = 56;
  // DeviceChrome의 앱 라벨이 프레임 위로 나가는 양 — DeviceChrome.svelte의 top: -34px
  const HEADROOM = 34;

  let available = $state(0);
  let availableHeight = $state(0);

  const usable = $derived(Math.max(0, availableHeight - HEADROOM * 2));
  const scaleOf = (item: Device) => (available > 0 && usable > 0 ? Math.min(1, available / item.width, usable / item.height) : 1);

  const scale = $derived(scaleOf(device));
  const inset = $derived(((anchor ?? device).width * scaleOf(anchor ?? device)) / 2);
  const reserve = $derived((target ?? device).width * scaleOf(target ?? device));
  const leadTop = $derived(Math.max(0, (availableHeight - device.height * scale) / 2 - HEADROOM));
  const frameRight = $derived(`calc(50% - ${inset}px)`);
  const leadLeft = $derived(`calc(50% - ${inset}px)`);
  const leadRight = $derived(`calc(50% - ${inset - reserve - LEAD_GAP}px)`);

  const hostClass = css({
    position: 'relative',
    display: 'grid',
    gridTemplateRows: '[minmax(0, 1fr)]',
    width: 'full',
    maxWidth: '[1400px]',
    height: 'full',
    marginX: 'auto',
  });
  const leadClass = css({
    display: { base: 'none', lg: 'block' },
    position: 'absolute',
    top: '[var(--lead-top)]',
    left: '[var(--lead-left)]',
    right: '[var(--lead-right)]',
    minWidth: '0',
  });
  const stageClass = css({ position: 'relative', display: 'grid', alignContent: 'center', minHeight: '0' });
  const boxClass = css({ position: 'relative', width: 'full' });
  const frameClass = css({
    position: 'absolute',
    top: '0',
    left: { base: '[50%]', md: '[auto]' },
    right: { base: '[auto]', md: '[var(--frame-right)]' },
    color: 'text.default',
    transformOrigin: { base: '[top center]', md: '[top right]' },
    transform: '[translateX(var(--frame-shift)) scale(var(--frame-scale))]',
    willChange: 'width, height, border-radius, padding, transform',
    '--frame-shift': { base: '-50%', md: '0px' },
    borderWidth: '[1.5px]',
    borderColor: 'border.emphasis',
  });
  const textClass = css({
    position: 'relative',
    fontFamily: 'prose',
    letterSpacing: '[-0.025em]',
    color: 'text.default',
    wordBreak: 'break-all',
  });
</script>

<div class={hostClass}>
  {#if aside}
    <div style:--lead-left={leadLeft} style:--lead-right={leadRight} style:--lead-top="{leadTop}px" class={leadClass}>
      {@render aside()}
    </div>
  {/if}

  <div class={stageClass} bind:clientWidth={available} bind:clientHeight={availableHeight}>
    <div style:height="{device.height * scale}px" class={boxClass}>
      <div
        style:width="{device.width}px"
        style:height="{device.height}px"
        style:border-radius="{device.radius}px"
        style:padding="{device.padding}px"
        style:padding-top="{device.paddingTop}px"
        style:--frame-scale={scale}
        style:--frame-right={frameRight}
        class={frameClass}
      >
        {@render decorations?.()}
        <p
          style:font-size="{device.fontSize}px"
          style:font-weight={String(Math.round(device.fontWeight))}
          style:line-height={String(device.lineHeight)}
          class={textClass}
        >
          {@render children()}
        </p>
      </div>
    </div>
  </div>
</div>
