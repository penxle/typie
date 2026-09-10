<script lang="ts">
  import ExternalArchived from './ExternalArchived.svelte';
  import ExternalEmbed from './ExternalEmbed.svelte';
  import ExternalFile from './ExternalFile.svelte';
  import ExternalImage from './ExternalImage.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
    onKeepMountedChange?: (keepMounted: boolean) => void;
  };

  let { element, onKeepMountedChange }: Props = $props();
</script>

{#if element.data.type === 'image'}
  <ExternalImage {element} {onKeepMountedChange} />
{:else if element.data.type === 'file'}
  <ExternalFile {element} />
{:else if element.data.type === 'embed'}
  <ExternalEmbed {element} />
{:else if element.data.type === 'archived'}
  <ExternalArchived {element} />
{/if}
