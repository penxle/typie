<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import AppIcon from '$assets/logos/app-icon-white.svg?component';
  import Mark from '$assets/logos/mark.svg?component';
  import { ramp } from '../../scrub';
  import { mix } from './device';
  import type { Device } from './device';

  type Props = { device: Device; progress: number; t: number };

  let { device, progress, t }: Props = $props();

  const laptop = $derived(1 - t);
  const phone = $derived(t);

  const DOCK_ICONS = 8;
  const APP_SLOT = 1;
  const DOCK_WIDTH = DOCK_ICONS * 26 + (DOCK_ICONS - 1) * 8 + 2 * 10;

  const iconsFade = $derived(1 - Math.min(1, t / 0.35));
  const identity = $derived(ramp(progress, 0.63, 0.66));

  const baseClass = css({
    position: 'absolute',
    insetX: '[-1.5px]',
    bottom: '[-19px]',
    height: '[14px]',
    borderWidth: '[1.5px]',
    borderColor: 'border.emphasis',
    borderRadius: '[0 0 14px 14px]',
    pointerEvents: 'none',
    _after: {
      content: '""',
      position: 'absolute',
      top: '[-1.5px]',
      left: '[50%]',
      width: '[64px]',
      height: '[5px]',
      marginLeft: '[-32px]',
      borderWidth: '[1.5px]',
      borderTopWidth: '0',
      borderColor: 'border.emphasis',
      borderRadius: '[0 0 5px 5px]',
    },
  });
  const windowTitleClass = flex({
    position: 'absolute',
    top: '20px',
    insetX: '0',
    align: 'center',
    justify: 'center',
    height: '12px',
    fontFamily: 'ui',
    fontSize: '13px',
    fontWeight: 'medium',
    letterSpacing: '[-0.01em]',
    color: 'text.muted',
    pointerEvents: 'none',
  });
  const lightsClass = flex({ position: 'absolute', align: 'center', pointerEvents: 'none' });
  const lightClass = css({ borderRadius: 'full' });
  const islandClass = css({
    position: 'absolute',
    top: '[12.5px]',
    left: '[50%]',
    width: '[92px]',
    height: '[27px]',
    marginLeft: '[-46px]',
    borderRadius: 'full',
    backgroundColor: 'border.emphasis',
    pointerEvents: 'none',
  });
  const dockClass = css({
    position: 'absolute',
    left: '[50%]',
    transform: '[translateX(-50%)]',
    borderWidth: '1px',
    borderColor: 'border.default',
    backgroundColor: 'text.default/6',
    overflow: 'hidden',
    pointerEvents: 'none',
  });
  const pillClass = css({ position: 'absolute', inset: '0', backgroundColor: 'text.default/40' });
  const iconsClass = flex({ position: 'absolute', top: '[50%]', left: '[50%]', gap: '8px', transformOrigin: '[center]' });
  const dockIconClass = css({ position: 'relative', size: '[26px]', borderRadius: '7px', flexShrink: '0' });
  const dockPlaceholderClass = css({ backgroundColor: 'text.default/18' });
  const dockAppClass = css({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    color: 'text.default',
    '& svg': { display: 'block', width: '[14px]', height: '[auto]' },
  });
  const runningDotClass = css({
    position: 'absolute',
    bottom: '[-6px]',
    left: '[50%]',
    size: '[3px]',
    marginLeft: '[-1.5px]',
    borderRadius: 'full',
    backgroundColor: 'text.default/60',
  });
  const identityClass = flex({
    position: 'absolute',
    top: '[-34px]',
    align: 'center',
    justify: 'flex-start',
    gap: '7px',
    color: 'text.muted',
    pointerEvents: 'none',
    whiteSpace: 'nowrap',
    willChange: 'opacity',
    '& svg': { display: 'block', size: '[16px]', borderRadius: '[3.5px]' },
  });
  const identityNameClass = css({ fontFamily: 'ui', fontSize: '13px', fontWeight: 'medium', letterSpacing: '[-0.01em]' });
  const statusClass = flex({ position: 'absolute', top: '24px', right: '28px', align: 'center', gap: '4px', pointerEvents: 'none' });
  const barsClass = flex({ align: 'flex-end', gap: '[1.5px]' });
  const barClass = css({ width: '[2px]', borderRadius: '[1px]', backgroundColor: 'text.default/70' });
  const wifiClass = css({ display: 'flex', color: 'text.default/70' });
  const batteryClass = css({
    position: 'relative',
    width: '[17px]',
    height: '[8px]',
    borderWidth: '[1px]',
    borderColor: 'text.default/70',
    borderRadius: '[2.5px]',
    _after: {
      content: '""',
      position: 'absolute',
      top: '[1.5px]',
      bottom: '[1.5px]',
      left: '[1.5px]',
      width: '[10px]',
      borderRadius: '[1px]',
      backgroundColor: 'text.default/70',
    },
  });
</script>

<span style:opacity={device.foot} class={baseClass}></span>

<span style:top="20px" style:left="20px" style:gap="8px" style:opacity={laptop} class={lightsClass}>
  {#each ['#ff5f57', '#febc2e', '#28c840'] as color, index (index)}
    <span style:width="12px" style:height="12px" style:background={color} class={lightClass}></span>
  {/each}
</span>
<span style:opacity={laptop} class={windowTitleClass}>타이피</span>
<span style:opacity={phone * (0.5 + phone * 0.5)} class={islandClass}></span>

<span
  style:bottom="{mix(14, 10, t)}px"
  style:width="{mix(DOCK_WIDTH, 100, t)}px"
  style:height="{mix(42, 4, t)}px"
  style:border-radius="{mix(14, 2, t)}px"
  style:border-color={t > 0.7 ? 'transparent' : undefined}
  class={dockClass}
>
  <span style:opacity={phone} class={pillClass}></span>
  <span style:opacity={iconsFade} style:transform="translate(-50%, -50%) scale({0.6 + iconsFade * 0.4})" class={iconsClass}>
    {#each { length: DOCK_ICONS }, index (index)}
      {#if index === APP_SLOT}
        <span class="{dockIconClass} {dockPlaceholderClass} {dockAppClass}">
          <Mark />
          <span class={runningDotClass}></span>
        </span>
      {:else}
        <span class="{dockIconClass} {dockPlaceholderClass}"></span>
      {/if}
    {/each}
  </span>
</span>

<span style:left="16px" style:opacity={identity} class={identityClass}>
  <AppIcon />
  <span class={identityNameClass}>타이피</span>
</span>

<span style:opacity={phone} class={statusClass}>
  <span class={barsClass}>
    <span style:height="3px" class={barClass}></span>
    <span style:height="4.5px" class={barClass}></span>
    <span style:height="6px" class={barClass}></span>
    <span style:height="7.5px" class={barClass}></span>
  </span>
  <span class={wifiClass}>
    <svg aria-hidden="true" fill="none" height="9" viewBox="0 0 16 12" width="12">
      <path d="M1.5 4.6a9.2 9.2 0 0 1 13 0" stroke="currentColor" stroke-linecap="round" stroke-width="1.9" />
      <path d="M4.3 7.4a5.3 5.3 0 0 1 7.4 0" stroke="currentColor" stroke-linecap="round" stroke-width="1.9" />
      <circle cx="8" cy="10.3" fill="currentColor" r="1.4" />
    </svg>
  </span>
  <span class={batteryClass}></span>
</span>
