<script lang="ts">
  import { TiptapNodeViewBubbleMenu } from '$lib/tiptap/components';
  import { NodeView } from '$lib/tiptap/lib';
  import { values } from '$lib/tiptap/values';
  import { center, flex } from '$styled-system/patterns';
  import type { NodeViewProps } from '$lib/tiptap/lib';

  type Props = NodeViewProps;

  let { editor, node, getPos, updateAttributes }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const { component: Component } = $derived(values.horizontalRule.find(({ type }) => type === node.attrs.type)!);
</script>

<NodeView style={center.raw({ minHeight: '[1lh]' })}>
  <Component />
</NodeView>

<TiptapNodeViewBubbleMenu {editor} {getPos} {node}>
  <div class={flex({ direction: 'column', gap: '4px' })}>
    {#each values.horizontalRule as { type, component: Component } (type)}
      <button class={center({ width: '400px', height: '50px' })} onclick={() => updateAttributes({ type })} type="button">
        <Component />
      </button>
    {/each}
  </div>
</TiptapNodeViewBubbleMenu>
