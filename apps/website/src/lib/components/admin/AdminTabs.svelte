<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    tabs: { key: string; label: string }[];
    current: string;
  };

  let { tabs, current = $bindable() }: Props = $props();
</script>

<div
  class={css({
    display: 'inline-flex',
    gap: '2px',
    borderRadius: '10px',
    backgroundColor: { base: 'surface.muted', _dark: 'admin.canvas' },
    padding: '3px',
    marginBottom: '20px',
  })}
>
  {#each tabs as tab (tab.key)}
    <button
      class={css({
        borderWidth: '1px',
        borderColor: current === tab.key ? 'border.subtle' : 'transparent',
        borderRadius: '8px',
        paddingX: '12px',
        paddingY: '6px',
        fontSize: '13px',
        fontWeight: current === tab.key ? 'semibold' : 'medium',
        color: current === tab.key ? 'text.default' : 'text.faint',
        backgroundColor: current === tab.key ? 'admin.card.default' : 'transparent',
        boxShadow: current === tab.key ? 'adminCard' : undefined,
        _hover: { color: 'text.default' },
      })}
      onclick={() => (current = tab.key)}
      type="button"
    >
      {tab.label}
    </button>
  {/each}
</div>
