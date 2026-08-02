<script lang="ts">
  import Page from './Page.svelte';
  import type { Editor } from '../editor.svelte';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  const presentedPageCount = $derived(Math.max(editor.pageSizes.length, editor.preparingPage === undefined ? 0 : editor.preparingPage + 1));
</script>

{#each Array.from({ length: presentedPageCount }, (_value, index) => index) as page (page)}
  {@const publishedPageCount = editor.pageSizes.length}
  {@const size = editor.pageSizes[page] ?? editor.appliedSnapshot.pageSizes[page]}
  {#if size}
    <Page
      backingHeight={editor.pageBackingSizes[page]?.height ?? editor.appliedSnapshot.pageBackingSizes[page]?.height ?? size.height}
      height={size.height}
      {page}
      preparing={page >= publishedPageCount}
      width={size.width}
    />
  {/if}
{/each}
