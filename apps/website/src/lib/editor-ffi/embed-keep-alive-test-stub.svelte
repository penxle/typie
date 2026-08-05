<script lang="ts">
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();
</script>

{#if element.data.type === 'embed'}
  <iframe data-keep-alive-embed={element.node} title="keep-alive embed"></iframe>
{/if}
