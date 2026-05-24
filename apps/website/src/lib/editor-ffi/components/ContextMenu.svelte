<script lang="ts">
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import ContextMenuDesktop from './ContextMenuDesktop.svelte';
  import ContextMenuTouch from './ContextMenuTouch.svelte';

  const ctx = getEditorContext();
</script>

{#if ctx.editor}
  {#if ctx.editor.contextMenu.source === 'touch'}
    <ContextMenuTouch />
  {:else}
    <ContextMenuDesktop />
  {/if}
{/if}
