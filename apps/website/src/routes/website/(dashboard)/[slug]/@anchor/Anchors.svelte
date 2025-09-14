<script lang="ts">
  import { calculateAnchorPositions, getAnchorElements } from '@typie/ui/anchor';
  import { onMount } from 'svelte';
  import { YState } from '../state.svelte';
  import Anchor from './Anchor.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';

  type Props = {
    anchors: YState<Record<string, string | null>>;
    editor: Ref<Editor> | undefined;
    showOutline?: boolean;
  };

  let { anchors, editor, showOutline = false }: Props = $props();

  const anchorElements = $derived.by(() => {
    if (!editor?.current) {
      return {};
    }

    return getAnchorElements(editor.current, Object.keys(anchors.current));
  });

  const anchorPositions = $derived.by(() => {
    if (!editor?.current || Object.keys(anchorElements).length === 0) return [];

    return calculateAnchorPositions(editor.current, anchorElements, anchors.current);
  });

  const updateAnchorName = (nodeId: string, name: string | null) => {
    const newAnchors = { ...anchors.current };
    newAnchors[nodeId] = name;
    anchors.current = newAnchors;
  };

  onMount(() => {
    if (editor) {
      editor.current.storage.anchors = anchors;
    }
  });
</script>

{#each anchorPositions as anchor (anchor.nodeId)}
  <Anchor
    name={anchor.name ?? anchor.excerpt}
    {editor}
    element={anchor.element}
    nodeId={anchor.nodeId}
    outline={showOutline}
    position={anchor.position}
    {updateAnchorName}
  />
{/each}
