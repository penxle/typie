<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import { defaultValues, values } from '../../values';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, HTMLAttributes }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  const { component: Component } = $derived(
    values.blockquote.find(({ type }) => type === attrs.type) ??
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      values.blockquote.find(({ type }) => type === defaultValues.blockquote)!,
  );
</script>

<NodeView style={flex.raw({ gap: '16px' })} {...HTMLAttributes}>
  <Component />

  <NodeViewContentEditable style={css.raw({ flexGrow: '1', '& > *': { textAlign: '[left!]', textIndent: '0!' } })} />
</NodeView>
