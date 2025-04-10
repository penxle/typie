<script lang="ts">
  import CircleAlertIcon from '~icons/lucide/circle-alert';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import InfoIcon from '~icons/lucide/info';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import { values } from '../../values';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, updateAttributes }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  const callouts = values.callout.map(({ type }) => type);
  type Callout = (typeof callouts)[number];

  const calloutMap = {
    info: { icon: InfoIcon, color: '#3b82f6' },
    success: { icon: CircleCheckIcon, color: '#22c55e' },
    warning: { icon: CircleAlertIcon, color: '#f97316' },
    danger: { icon: TriangleAlertIcon, color: '#dc2626' },
  };

  const icon = $derived(calloutMap[attrs.type as Callout].icon);
  const color = $derived(calloutMap[attrs.type as Callout].color);
</script>

<NodeView>
  <div
    style:border-color={color}
    style:background-color={`color-mix(in srgb, ${color} 2%, transparent)`}
    class={flex({
      alignItems: 'flex-start',
      gap: '8px',
      borderWidth: '1px',
      borderRadius: '8px',
      paddingX: '12px',
      paddingY: '16px',
    })}
  >
    <svelte:element
      this={editor?.current.isEditable ? 'button' : 'div'}
      style:color
      class={css(
        {
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: '2px',
          size: '28px',
          _hover: {
            backgroundColor: 'gray.900/8',
          },
        },
        !editor?.current.isEditable && { pointerEvents: 'none' },
      )}
      contenteditable={false}
      onclick={() => {
        const type = callouts[(callouts.indexOf(attrs.type) + 1) % callouts.length];
        updateAttributes({ type });
      }}
      role={editor?.current.isEditable ? 'button' : 'img'}
      {...editor?.current.isEditable && {
        type: 'button',
      }}
    >
      <Icon {icon} size={20} />
    </svelte:element>

    <NodeViewContentEditable
      style={css.raw({
        flexGrow: '1',
        '& > p': {
          textAlign: '[left!]',
          textIndent: '0!',
          '&:first-child': {
            marginTop: '[calc((28px - 1lh) / 2)]',
          },
        },
      })}
    />
  </div>
</NodeView>
