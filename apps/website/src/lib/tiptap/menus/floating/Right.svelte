<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { fade } from 'svelte/transition';
  import BookmarkIcon from '~icons/lucide/bookmark';
  import BookmarkFilledIcon from '~icons/typie/bookmark-filled';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';

  type Props = {
    editor: Editor;
    pos: number;
  };

  let { editor, pos }: Props = $props();

  const node = $derived(editor.state.doc.nodeAt(pos));

  const handleAnchor = () => {
    if (!node) {
      return;
    }

    if (editor.storage.anchors.current[node.attrs.nodeId] === undefined) {
      editor.storage.anchors.current = { ...editor.storage.anchors.current, [node.attrs.nodeId]: null };

      mixpanel.track('anchor_add');
    } else {
      editor.storage.anchors.current = Object.fromEntries(
        Object.entries(editor.storage.anchors.current).filter(([key]) => key !== node.attrs.nodeId),
      );

      mixpanel.track('anchor_remove');
    }
  };
</script>

{#if node}
  <div class={flex({ align: 'center' })} transition:fade|global={{ duration: 100 }}>
    <button
      class={css({
        borderRadius: '6px',
        padding: '2px',
        color: editor.storage.anchors.current[node.attrs.nodeId] === undefined ? 'text.faint' : { base: '[#FACC15]', _dark: '[#B8860B]' },
        _hover: { backgroundColor: 'interactive.hover' },
      })}
      onclick={handleAnchor}
      type="button"
    >
      <Icon icon={editor.storage.anchors.current[node.attrs.nodeId] === undefined ? BookmarkIcon : BookmarkFilledIcon} size={18} />
    </button>

    <div
      class={css({
        backgroundColor: 'white/80',
        borderRadius: '4px',
        fontSize: '12px',
        color: 'text.subtle',
      })}
    >
      {editor.storage.anchors.current[node.attrs.nodeId]}
    </div>
  </div>
{/if}
