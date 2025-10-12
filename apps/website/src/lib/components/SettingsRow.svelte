<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    label: import('svelte').Snippet;
    description?: import('svelte').Snippet;
    value: import('svelte').Snippet;
    error?: import('svelte').Snippet;
    vertical?: boolean;
  };

  let { label, description, value, error, vertical = false }: Props = $props();
</script>

{#if vertical}
  <div class={css({ paddingX: '20px', paddingY: '16px' })}>
    <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', marginBottom: '4px' })}>
      {@render label()}
    </div>
    {#if description}
      <div class={css({ fontSize: '12px', fontWeight: 'normal', color: 'text.subtle', marginBottom: '12px' })}>
        {@render description()}
      </div>
    {/if}
    <div>
      {@render value()}
    </div>
    {#if error}
      <div class={css({ marginTop: '4px', _empty: { display: 'none' } })}>
        {@render error()}
      </div>
    {/if}
  </div>
{:else}
  <div
    class={css({
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      minHeight: '56px',
      paddingX: '20px',
      paddingY: '12px',
      gap: '16px',
    })}
  >
    <div class={css({ flex: '1' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>
        {@render label()}
      </div>
      {#if description}
        <div class={css({ fontSize: '12px', fontWeight: 'normal', color: 'text.subtle', marginTop: '2px' })}>
          {@render description()}
        </div>
      {/if}
    </div>

    <div class={css({ flex: 'none', display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: '4px' })}>
      <div class={css({ fontSize: '13px', color: 'text.subtle' })}>
        {@render value()}
      </div>
      {#if error}
        <div class={css({ _empty: { display: 'none' } })}>
          {@render error()}
        </div>
      {/if}
    </div>
  </div>
{/if}
