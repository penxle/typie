<script lang="ts">
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';

  type Props = {
    shortcut: string;
  };

  let { shortcut }: Props = $props();

  const modKey = navigator.platform.includes('Mac') ? '⌘' : 'Ctrl';
  const keys = $derived(shortcut.split('-').map((key) => (key === 'Mod' ? modKey : key)));
</script>

<span class={css({ display: 'inline-flex', alignItems: 'center', gap: '2px' })}>
  {#each keys as key, index (key + index)}
    <kbd
      class={center({
        paddingX: '6px',
        paddingY: '2px',
        borderWidth: '1px',
        borderRadius: '4px',
        borderColor: 'border.default',
        fontFamily: 'mono',
        fontSize: '12px',
        fontWeight: 'medium',
        color: 'text.subtle',
        backgroundColor: 'surface.muted',
      })}
    >
      {key}
    </kbd>

    {#if index < keys.length - 1}
      <span class={css({ color: 'text.faint', marginX: '1px' })}>+</span>
    {/if}
  {/each}
</span>
