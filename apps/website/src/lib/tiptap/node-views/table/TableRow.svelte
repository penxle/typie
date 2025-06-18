<script lang="ts">
  import { css } from '$styled-system/css';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { HTMLAttributes }: Props = $props();
</script>

<NodeView {...HTMLAttributes} style={css.raw({ '&:last-child :is(td, th)': { borderBottom: 'none' } })} as="tr">
  <NodeViewContentEditable transparent />
</NodeView>
