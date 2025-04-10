<script lang="ts">
  import { center } from '$styled-system/patterns';
  import { NodeView } from '../../lib';
  import { values } from '../../values';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const { component: Component } = $derived(values.horizontalRule.find(({ type }) => type === attrs.type)!);
</script>

<NodeView style={center.raw({ minHeight: '[1lh]' })}>
  <Component />
</NodeView>
