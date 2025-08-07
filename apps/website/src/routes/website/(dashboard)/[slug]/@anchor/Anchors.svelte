<script lang="ts">
  import { calculateAnchorPositions, getAnchorElements } from '$lib/anchor';
  import Anchor from './Anchor.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    anchors: Record<string, string | null>;
    editor: Ref<Editor> | undefined;
    showOutline?: boolean;
    updateAnchorName: (nodeId: string, name: string | null) => void;
  };

  let { anchors, editor, showOutline = false, updateAnchorName }: Props = $props();

  const anchorElements = $derived.by(() => {
    if (!editor) {
      return {};
    }

    return getAnchorElements(Object.keys(anchors));
  });

  const anchorPositions = $derived.by(() => {
    if (!editor || Object.keys(anchorElements).length === 0) return [];

    return calculateAnchorPositions(anchorElements, anchors);
  });
</script>

{#each anchorPositions as anchor (anchor.nodeId)}
  <Anchor
    name={anchor.name}
    {editor}
    element={anchor.element}
    nodeId={anchor.nodeId}
    outline={showOutline}
    position={anchor.position}
    {updateAnchorName}
  />
{/each}
