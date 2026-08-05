<script lang="ts">
  import DocumentEmbeds from './DocumentEmbeds.svelte';
  import Page from './Page.svelte';
  import type { Editor } from '../editor.svelte';
  import type { EditorSurfaceHost } from '../editor-surface-host.svelte';

  type Props = {
    editor: Editor;
    surfaceHost: EditorSurfaceHost | undefined;
  };

  let { editor, surfaceHost }: Props = $props();
</script>

{#key editor}
  {#if surfaceHost}
    {#key surfaceHost}
      {#each editor.pageSizes as size, page (page)}
        {#if size}
          <Page {editor} height={size.height} {page} {surfaceHost} width={size.width} />
        {/if}
      {/each}
    {/key}
  {/if}

  <DocumentEmbeds {editor} />
{/key}
