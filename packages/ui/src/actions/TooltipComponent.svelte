<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '../components';
  import type { Action } from 'svelte/action';
  import type { TooltipPresentation } from './tooltip-coordinator.svelte';

  type Props = {
    presentation: TooltipPresentation;
    outgoingPresentation?: TooltipPresentation;
    contentHidden: boolean;
    floating: Action<HTMLElement>;
    presenceAction: Action<HTMLElement>;
    surfaceAction: Action<HTMLElement>;
    arrowAction: Action<HTMLElement>;
    contentAction: Action<HTMLElement>;
    outgoingContentAction: Action<HTMLElement>;
    showArrow: boolean;
    motion: 'idle' | 'travel' | 'crossfade';
  };

  type ModifierKey = 'Mod' | 'Ctrl' | 'Alt' | 'Shift';

  const isMac = typeof window !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.userAgent);
  const modifierKeys: Record<ModifierKey, string> = {
    Mod: isMac ? '⌘' : 'Ctrl',
    Ctrl: isMac ? '⌃' : 'Ctrl',
    Alt: isMac ? '⌥' : 'Alt',
    Shift: isMac ? '⇧' : 'Shift',
  };

  const contentClass = (value: TooltipPresentation) =>
    value.kind === 'action' ? flex({ alignItems: 'center', gap: '4px', fontWeight: 'semibold' }) : css({ fontWeight: 'medium' });

  let {
    presentation,
    outgoingPresentation,
    contentHidden,
    floating,
    presenceAction,
    surfaceAction,
    arrowAction,
    contentAction,
    outgoingContentAction,
    showArrow,
    motion,
  }: Props = $props();
</script>

{#snippet renderPresentation(value: TooltipPresentation)}
  {#if value.kind === 'action'}
    <span class={css({ whiteSpace: 'pre-line' })}>{value.message}</span>

    {#if value.trailingIcon}
      <Icon style={css.raw({ color: 'text.on.inverse', opacity: '50' })} icon={value.trailingIcon} size={12} />
    {/if}
    {#if value.trailing}
      <span class={css({ color: 'text.on.inverse', opacity: '50' })}>{value.trailing}</span>
    {/if}

    {#if value.keys}
      <div
        class={flex({
          gap: isMac ? '0' : '2px',
          alignItems: 'center',
          fontFamily: '[Pretendard]',
          fontWeight: 'medium',
          color: 'text.on.inverse',
          opacity: '50',
          lineHeight: '[1em]',
        })}
      >
        {#each value.keys as key, index (index)}
          <kbd class={center({ minWidth: '12px' })}>
            {modifierKeys[key as ModifierKey] ?? key}
          </kbd>

          {#if !isMac && index < value.keys.length - 1}
            <span>+</span>
          {/if}
        {/each}
      </div>
    {/if}
  {:else if typeof value.message === 'string'}
    {value.message}
  {:else}
    {@render value.message?.()}
  {/if}
{/snippet}

<div
  class={css({ width: '[max-content]', maxWidth: '[calc(100vw - 16px)]', zIndex: 'tooltip', pointerEvents: 'none' })}
  data-tooltip-motion={motion}
  role="tooltip"
  use:floating
>
  <div class={css({ position: 'relative' })} data-tooltip-presence use:presenceAction>
    <div
      class={css(
        {
          position: 'relative',
          overflow: 'hidden',
          borderRadius: '4px',
          boxSizing: 'border-box',
          paddingX: '8px',
          paddingY: '4px',
          fontSize: '12px',
          color: 'text.on.inverse',
          backgroundColor: 'surface.inverse',
          boxShadow: 'md',
        },
        presentation.kind === 'wrapper' ? presentation.tooltipStyle : undefined,
      )}
      data-tooltip-surface
      use:surfaceAction
    >
      <div class={css({ position: 'relative' })}>
        <div
          style:opacity={contentHidden ? 0 : undefined}
          class={contentClass(presentation)}
          data-tooltip-content="current"
          use:contentAction
        >
          {@render renderPresentation(presentation)}
        </div>

        {#if outgoingPresentation}
          <div
            class={[css({ position: 'absolute', top: '0', left: '0' }), contentClass(outgoingPresentation)]}
            aria-hidden="true"
            data-tooltip-content="outgoing"
            use:outgoingContentAction
          >
            {@render renderPresentation(outgoingPresentation)}
          </div>
        {/if}
      </div>
    </div>

    {#if showArrow}
      <div
        class={css({
          borderTopLeftRadius: '2px',
          size: '8px',
          backgroundColor: 'surface.inverse',
        })}
        use:arrowAction
      ></div>
    {/if}
  </div>
</div>
