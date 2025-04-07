<script lang="ts">
  import { css } from '$styled-system/css';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import Menu from './Menu.svelte';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, updateAttributes }: Props = $props();
</script>

<NodeView style={css.raw({ position: 'relative' })}>
  {#if editor?.current.isEditable}
    <div class={css({ position: 'absolute', top: '4px', right: '4px' })} contentEditable={false}>
      <Menu {node} {updateAttributes} />
    </div>
  {/if}

  <NodeViewContentEditable
    style={css.raw({
      paddingY: '18px',
      paddingX: '16px',
      fontSize: '14px',
      fontFamily: 'mono',
      backgroundColor: 'gray.100',
      borderRadius: '4px',
      overflowX: 'auto',
      whiteSpace: 'pre-wrap',
      '&:not(:has(.ProseMirror-trailingBreak))': {
        _after: {
          content: '""',
          display: 'inline-block',
        },
      },
    })}
    as="pre"
  />
</NodeView>
