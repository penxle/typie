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
    node: Ref<ProseMirrorNode>;
    getPos: () => number;
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

    if (from !== getPos() || to !== getPos() + node.current.nodeSize) {
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
      borderColor: 'gray.300',
      backgroundColor: 'white',
      boxShadow: '[2px 2px 8px 0 {colors.gray.900/15}]',
    })}
    use:floating
  >
    <div
      class={center({
        position: 'relative',
        gap: '8px',
        paddingX: '14px',
        paddingY: '8px',
        backgroundColor: 'white',
        zIndex: '2',
      })}
    >
      {@render children()}
    </div>
  </div>
{/if}
