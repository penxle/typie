<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, updateAttributes }: Props = $props();

  let open = $state(node.attrs.open);
</script>

<NodeView>
  <details
    class={flex({ flexDirection: 'column', borderWidth: '1px', borderRadius: '8px' })}
    ontoggle={(e) => {
      if (editor?.current.isEditable) {
        updateAttributes({ open: e.currentTarget.open });
      }
    }}
    bind:open
  >
    <summary
      class={flex({
        alignItems: 'center',
        gap: '8px',
        paddingX: '12px',
        paddingY: '8px',
        color: 'gray.500',
        backgroundColor: 'gray.100',
        userSelect: 'none',
        cursor: 'pointer',
      })}
      contenteditable={false}
    >
      <Icon style={css.raw({ '& *': { strokeWidth: '[1.5px]' } })} icon={open ? ChevronUpIcon : ChevronDownIcon} size={20} />
      <div class={css({ fontSize: '14px', fontWeight: 'medium' })}>
        {open ? '접기' : '펼치기'}
      </div>
    </summary>

    <NodeViewContentEditable style={css.raw({ paddingX: '24px', paddingY: '16px' })} />
  </details>
</NodeView>
