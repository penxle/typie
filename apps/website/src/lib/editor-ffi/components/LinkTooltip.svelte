<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { IS_MAC } from '../constants';
  import { getEditorContext } from '../editor.svelte';

  const { editor } = getEditorContext();

  const hover = $derived(editor?.linkHover);
  const followHint = $derived(editor?.readOnly ? '클릭하여 열기' : `${IS_MAC ? '⌘' : 'Ctrl'} + 클릭하여 열기`);
</script>

{#if hover}
  <div
    style:left={`${hover.clientX + 12}px`}
    style:top={`${hover.clientY + 18}px`}
    class={css({
      position: 'fixed',
      zIndex: '50',
      maxWidth: '320px',
      paddingX: '8px',
      paddingY: '4px',
      borderRadius: '6px',
      backgroundColor: 'surface.dark',
      color: 'text.bright',
      fontSize: '11px',
      lineHeight: '[1.3]',
      pointerEvents: 'none',
      boxShadow: '[0_2px_8px_rgba(0,0,0,0.15)]',
    })}
  >
    <div class={css({ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
      {hover.link.href}
    </div>
    <div class={css({ opacity: '70', fontSize: '10px', marginTop: '2px' })}>
      {followHint}
    </div>
  </div>
{/if}
