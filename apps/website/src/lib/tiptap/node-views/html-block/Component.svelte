<script lang="ts">
  import { on } from 'svelte/events';
  import { css } from '$styled-system/css';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import { transform } from './utils';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { editor, node }: Props = $props();

  let iframeEl = $state<HTMLIFrameElement>();

  $effect(() => {
    // eslint-disable-next-line unicorn/prefer-global-this
    const off = on(window, 'message', (event) => {
      if (event.source === iframeEl?.contentWindow && event.data.type === 'resize') {
        iframeEl.height = `${event.data.height}px`;
      }
    });

    return () => {
      off();
    };
  });
</script>

<NodeView>
  {#if editor?.current.isEditable}
    <NodeViewContentEditable
      style={css.raw({
        paddingY: '18px',
        paddingX: '16px',
        fontSize: '14px',
        fontWeight: 'medium',
        fontFamily: 'mono',
        backgroundColor: 'gray.200',
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
  {:else}
    <NodeViewContentEditable style={css.raw({ display: 'none' })} />

    <iframe
      bind:this={iframeEl}
      class={css({ display: 'block', width: 'full' })}
      height="0"
      loading="lazy"
      referrerpolicy="no-referrer"
      sandbox="allow-scripts"
      srcdoc={transform(node.textContent)}
      title="HTML"
    ></iframe>
  {/if}
</NodeView>
