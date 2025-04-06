<script lang="ts">
  import { NodeView, NodeViewContentEditable } from '$lib/tiptap/lib';
  import { values } from '$lib/tiptap/values';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { NodeViewProps } from '$lib/tiptap/lib';

  type Props = NodeViewProps;

  let { node }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const { component: Component } = $derived(values.blockquote.find(({ type }) => type === node.attrs.type)!);
</script>

<NodeView style={flex.raw({ gap: '16px' })}>
  <Component />

  <NodeViewContentEditable style={css.raw({ flexGrow: '1', '& > *': { textAlign: '[left!]', textIndent: '0!' } })} />
</NodeView>
