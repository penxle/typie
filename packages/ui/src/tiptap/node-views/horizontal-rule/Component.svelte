<script lang="ts">
  import { center } from '@typie/styled-system/patterns';
  import { NodeView } from '../../lib';
  import { defaultValues, values } from '../../values';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, HTMLAttributes }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  const { component: Component } = $derived(
    values.horizontalRule.find(({ type }) => type === attrs.type) ??
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      values.horizontalRule.find(({ type }) => type === defaultValues.horizontalRule)!,
  );
</script>

<NodeView style={center.raw({ width: 'full', height: '16px' })} {...HTMLAttributes}>
  <Component />
</NodeView>
