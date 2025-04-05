<script lang="ts">
  import { NodeView } from '$lib/tiptap/lib';
  import { values } from '$lib/tiptap/values';
  import { center } from '$styled-system/patterns';
  import type { NodeViewProps } from '$lib/tiptap/lib';

  type Props = NodeViewProps;

  let { node }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const { component: Component } = $derived(values.horizontalRule.find(({ type }) => type === node.attrs.type)!);
</script>

<NodeView style={center.raw({ minHeight: '[1lh]' })}>
  <Component />
</NodeView>
