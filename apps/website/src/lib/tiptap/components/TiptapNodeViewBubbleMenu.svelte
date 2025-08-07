<script lang="ts">
  import { hide } from '@floating-ui/dom';
  import { Editor, posToDOMRect } from '@tiptap/core';
  import { createFloatingActions } from '$lib/actions';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import type { VirtualElement } from '@floating-ui/dom';
  import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
  import type { Snippet } from 'svelte';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
    node: ProseMirrorNode;
    getPos: () => number | undefined;
    children: Snippet;
  };

  let { editor, node, getPos, children }: Props = $props();

  let open = $state(false);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 10,
    middleware: [hide({ strategy: 'escaped' })],
  });

  const update = (editor: Editor) => {
    const { from, to } = editor.state.selection;
    const pos = getPos();
    if (pos === undefined) {
      return;
    }

    if (from !== pos || to !== pos + node.nodeSize) {
      open = false;
      return;
    }

    open = true;

    const targetEl: VirtualElement = {
      getBoundingClientRect: () => posToDOMRect(editor.view, from, to),
      contextElement: editor.view.dom,
    };

    anchor(targetEl);
  };

  $effect(() => {
    if (editor) {
      update(editor.current);
    }
  });
</script>

{#if open}
  <div
    class={css({
      borderWidth: '1px',
      borderRadius: '4px',
      borderColor: 'border.strong',
      backgroundColor: 'surface.default',
      cursor: 'pointer',
      boxShadow: 'small',
    })}
    use:floating
  >
    <div
      class={center({
        position: 'relative',
        gap: '8px',
        borderRadius: '4px',
        paddingX: '14px',
        paddingY: '8px',
        backgroundColor: 'surface.default',
        zIndex: 'editor',
      })}
    >
      {@render children()}
    </div>
  </div>
{/if}
