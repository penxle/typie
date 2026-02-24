<script lang="ts">
  import { getEditorContext } from '$lib/editor/context.svelte';
  import ContextMenuDesktop from './ContextMenuDesktop.svelte';
  import ContextMenuTouch from './ContextMenuTouch.svelte';

  const { editor } = getEditorContext();
</script>

{#if editor.contextMenu.source === 'touch'}
  <ContextMenuTouch />
{:else}
  <ContextMenuDesktop />
{/if}
