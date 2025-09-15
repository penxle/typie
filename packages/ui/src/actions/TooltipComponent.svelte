<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { scale } from 'svelte/transition';
  import type { ArrowAction, FloatingAction } from './floating.svelte';

  type Props = {
    message?: string | null;
    trailing?: string;
    keys?: [...ModifierKey[], string];
    floating: FloatingAction;
    arrow?: ArrowAction;
  };

  type ModifierKey = 'Mod' | 'Ctrl' | 'Alt' | 'Shift';

  const isMac = typeof window !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.userAgent);
  const modifierKeys: Record<ModifierKey, string> = {
    Mod: isMac ? '⌘' : 'Ctrl',
    Ctrl: isMac ? '⌃' : 'Ctrl',
    Alt: isMac ? '⌥' : 'Alt',
    Shift: isMac ? '⇧' : 'Shift',
  };

  let { message, trailing, keys, floating, arrow }: Props = $props();
</script>

{#if message}
  <div
    class={flex({
      alignItems: 'center',
      gap: '4px',
      borderRadius: '4px',
      paddingX: '8px',
      paddingY: '4px',
      fontSize: '12px',
      fontWeight: 'semibold',
      color: 'text.bright',
      backgroundColor: 'surface.dark',
      boxShadow: 'medium',
      zIndex: 'tooltip',
      pointerEvents: 'none',
    })}
    role="tooltip"
    use:floating={{ appendTo: document.querySelector('.tooltip-container') as Element | null }}
    transition:scale|global={{ start: 0.9, duration: 200 }}
  >
    <span>{message}</span>

    {#if trailing}
      <span class={css({ color: 'text.bright', opacity: '50' })}>{trailing}</span>
    {/if}

    {#if keys}
      <div
        class={flex({
          gap: isMac ? '0' : '2px',
          alignItems: 'center',
          fontFamily: '[Pretendard]',
          fontWeight: 'medium',
          color: 'text.bright',
          opacity: '50',
          lineHeight: '[1em]',
        })}
      >
        {#each keys as key, index (index)}
          <kbd class={center({ minWidth: '12px' })}>
            {modifierKeys[key as ModifierKey] ?? key}
          </kbd>

          {#if !isMac && index < keys.length - 1}
            <span>+</span>
          {/if}
        {/each}
      </div>
    {/if}

    {#if arrow}
      <div
        class={css({
          borderTopLeftRadius: '2px',
          size: '8px',
          backgroundColor: 'surface.dark',
        })}
        use:arrow
      ></div>
    {/if}
  </div>
{/if}
