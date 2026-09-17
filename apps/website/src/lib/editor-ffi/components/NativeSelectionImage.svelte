<script lang="ts">
  import ExternalImage from './ExternalImage.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';
  import type { Editor } from '../editor.svelte';

  let { editor, element }: { editor: Editor; element: ExternalElement } = $props();
  let keepMounted = $state(false);
  const size = $derived(editor.images.displaySize(element));
  const visible = $derived(
    editor.published?.frames.has(element.page_idx) === true &&
      editor.pageExternalElements(element.page_idx).some((candidate) => candidate.node === element.node),
  );
</script>

<!-- This atom never leaves the reading-order DOM. Its pixels are transparent;
     copy obtains the actual URL from the engine's asset metadata. -->
<img
  style:left={`${element.bounds.x + (element.bounds.width - (size?.width ?? element.bounds.width)) / 2}px`}
  style:top={`${element.bounds.y}px`}
  style:width={`${size?.width ?? element.bounds.width}px`}
  style:height={`${size?.height ?? element.bounds.height}px`}
  class="selection-image"
  alt="본문 이미지"
  aria-hidden={visible || keepMounted}
  data-selection-atom
  draggable="false"
  src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
/>

{#if visible || keepMounted}
  <ExternalImage {element} onKeepMountedChange={(value) => (keepMounted = value)} />
{/if}

<style>
  .selection-image {
    position: absolute;
    z-index: 1;
    pointer-events: none;
    user-select: text;
    -webkit-user-select: text;
  }
</style>
