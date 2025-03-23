<script lang="ts">
  import { TiptapNodeViewBubbleMenu } from '$lib/tiptap/components';
  import { NodeView, NodeViewContentEditable } from '$lib/tiptap/lib';
  import { values } from '$lib/tiptap/values';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { NodeViewProps } from '$lib/tiptap/lib';

  type Props = NodeViewProps;

  let { editor, node, getPos, updateAttributes }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const { component: Component } = $derived(values.blockquote.find(({ type }) => type === node.attrs.type)!);
</script>

<NodeView style={flex.raw({ gap: '16px' })}>
  <Component />

  <NodeViewContentEditable style={css.raw({ flexGrow: '1' })} />
</NodeView>

<TiptapNodeViewBubbleMenu {editor} {getPos} {node}>
  <div class={flex({ direction: 'column', gap: '4px' })}>
    {#each values.blockquote as { type, component: Component } (type)}
      <button class={center({ width: '400px', height: '50px' })} onclick={() => updateAttributes({ type })} type="button">
        <Component />
      </button>
    {/each}
  </div>
</TiptapNodeViewBubbleMenu>
