<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, updateAttributes, HTMLAttributes }: Props = $props();

  let open = $state(editor?.current.isEditable ? node.attrs.open : false);
</script>

<NodeView {...HTMLAttributes}>
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
        borderTopRadius: '8px',
        borderBottomRadius: open ? '0' : '8px',
        paddingX: '12px',
        paddingY: '8px',
        color: 'gray.500',
        backgroundColor: 'gray.100',
        userSelect: 'none',
        cursor: 'pointer',
      })}
      contenteditable={false}
      onkeyup={(e) => {
        if (e.code === 'Space') {
          e.preventDefault();
        }
      }}
    >
      <Icon style={css.raw({ '& *': { strokeWidth: '[1.5px]' } })} icon={open ? ChevronUpIcon : ChevronDownIcon} size={20} />
      {#if editor?.current.isEditable}
        <input
          class={css({ flexGrow: '1', fontSize: '14px', fontWeight: 'medium' })}
          oninput={(e) => updateAttributes({ title: e.currentTarget.value })}
          placeholder="제목을 입력하세요"
          type="text"
          value={node.attrs.title}
        />
      {:else}
        <span class={css({ flexGrow: '1', fontSize: '14px', fontWeight: 'medium' })}>{node.attrs.title}</span>
      {/if}
    </summary>

    <NodeViewContentEditable style={css.raw({ paddingX: '24px', paddingY: '16px' })} />
  </details>
</NodeView>
