<script lang="ts">
  import CircleAlertIcon from '~icons/lucide/circle-alert';
  import CircleCheckIcon from '~icons/lucide/circle-check';
  import InfoIcon from '~icons/lucide/info';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import { token } from '$styled-system/tokens';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import { values } from '../../values';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, updateAttributes, HTMLAttributes }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  const callouts = values.callout.map(({ type }) => type);
  type Callout = (typeof callouts)[number];

  const calloutMap = {
    info: { icon: InfoIcon, color: token('colors.callout.info') },
    success: { icon: CircleCheckIcon, color: token('colors.callout.success') },
    warning: { icon: CircleAlertIcon, color: token('colors.callout.warning') },
    danger: { icon: TriangleAlertIcon, color: token('colors.callout.danger') },
  };

  const icon = $derived(calloutMap[attrs.type as Callout].icon);
  const color = $derived(calloutMap[attrs.type as Callout].color);
</script>

<NodeView {...HTMLAttributes}>
  <div
    style:border-color={color}
    style:background-color={`color-mix(in srgb, ${color} 3%, transparent)`}
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
          _supportHover: {
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
