<script lang="ts">
  import IconCodeXml from '~icons/lucide/code-xml';
  import IconGlobe from '~icons/lucide/globe';
  import IconPanelTop from '~icons/lucide/panel-top';
  import IconRotateCw from '~icons/lucide/rotate-cw';
  import { browser } from '$app/environment';
  import { Icon, RingSpinner } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import { transform } from './utils';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { editor, node, HTMLAttributes }: Props = $props();

  let iframeEl = $state<HTMLIFrameElement>();
  let height = $state(0);

  let mode = $state<'preview' | 'code'>(editor?.current.isEditable ? 'code' : 'preview');

  const handleModeSwitch = () => {
    if (mode === 'preview') {
      mode = 'code';
    } else if (mode === 'code') {
      height = 0;
      mode = 'preview';
    }
  };

  const handleRefresh = () => {
    if (iframeEl) {
      height = 0;
      iframeEl.srcdoc = transform(node.textContent);
    }
  };
</script>

<svelte:window
  onmessage={(event) => {
    if (event.source === iframeEl?.contentWindow && event.data.type === 'resize') {
      height = event.data.height;
    }
  }}
/>

<NodeView
  style={css.raw({
    borderWidth: '1px',
    borderRadius: '8px',
    backgroundColor: 'gray.50',
    overflow: 'hidden',
  })}
  {...HTMLAttributes}
>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      gap: '60px',
      borderBottomWidth: '1px',
      paddingX: '12px',
      paddingY: '8px',
      backgroundColor: 'gray.100',
    })}
    contentEditable={false}
  >
    <div class={flex({ alignItems: 'center', gap: '6px', flexShrink: '0' })}>
      <div class={css({ borderRadius: 'full', size: '12px', backgroundColor: '[#FF5F57]' })}></div>
      <div class={css({ borderRadius: 'full', size: '12px', backgroundColor: '[#FFBD2E]' })}></div>
      <div class={css({ borderRadius: 'full', size: '12px', backgroundColor: '[#28C840]' })}></div>
    </div>

    <div
      class={center({
        gap: '6px',
        flexGrow: '1',
        borderRadius: '8px',
        paddingX: '10px',
        paddingY: '4px',
        maxWidth: '550px',
        backgroundColor: 'gray.200',
      })}
    >
      <Icon style={css.raw({ color: 'gray.500' })} icon={IconGlobe} size={14} />
      <div class={css({ fontSize: '13px', color: 'gray.500', userSelect: 'none' })}>HTML</div>
    </div>

    <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
      <button
        class={center({
          size: '28px',
          borderRadius: '4px',
          color: 'gray.500',
          _enabled: {
            _hover: { backgroundColor: 'gray.200' },
          },
          _disabled: { opacity: '50' },
        })}
        disabled={mode === 'code'}
        onclick={handleRefresh}
        type="button"
      >
        <Icon icon={IconRotateCw} size={14} />
      </button>

      <button
        class={center({
          size: '28px',
          borderRadius: '4px',
          color: 'gray.500',
          _hover: { backgroundColor: 'gray.200' },
        })}
        onclick={handleModeSwitch}
        type="button"
      >
        <Icon icon={mode === 'code' ? IconPanelTop : IconCodeXml} size={14} />
      </button>
    </div>
  </div>

  <NodeViewContentEditable
    style={css.raw(
      {
        paddingX: '16px',
        paddingY: '16px',
        minHeight: '80px',
        fontFamily: 'mono',
        fontSize: '14px',
        backgroundColor: 'white',
        overflowX: 'auto',
        whiteSpace: 'pre',
        tabSize: '4',
        touchAction: 'none',
        '&:not(:has(.ProseMirror-trailingBreak))': {
          _after: {
            content: '""',
            display: 'inline-block',
          },
        },
      },
      mode === 'preview' && { display: 'none' },
    )}
  />

  {#if mode === 'preview'}
    <div class={css({ position: 'relative', backgroundColor: 'white', minHeight: '200px' })} contentEditable={false}>
      {#if height === 0}
        <div class={center({ position: 'absolute', inset: '0', backgroundColor: 'white' })}>
          <RingSpinner style={css.raw({ size: '24px', color: 'gray.500' })} />
        </div>
      {/if}

      {#if browser}
        <iframe
          bind:this={iframeEl}
          style:height={`${height}px`}
          class={css({ display: 'block', width: 'full' })}
          loading="lazy"
          referrerpolicy="no-referrer"
          sandbox="allow-scripts"
          srcdoc={transform(node.textContent)}
          title="HTML 블록"
        ></iframe>
      {/if}
    </div>
  {/if}
</NodeView>
