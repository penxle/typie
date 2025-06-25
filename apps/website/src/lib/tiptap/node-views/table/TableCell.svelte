<script lang="ts">
  import { css } from '$styled-system/css';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { HTMLAttributes }: Props = $props();
</script>

<NodeView
  {...HTMLAttributes}
  style={css.raw({
    borderWidth: '1px',
    borderTopWidth: '0',
    borderColor: 'border.strong',
    paddingX: '14px',
    paddingY: '10px',
  })}
  as="td"
>
  <NodeViewContentEditable transparent />
</NodeView>
