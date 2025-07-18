<script lang="ts">
  import { onMount, tick } from 'svelte';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileUpIcon from '~icons/lucide/file-up';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Button, Icon, Menu, MenuItem, RingSpinner, TextInput } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { NodeView } from '../../lib';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, selected, updateAttributes, deleteNode, HTMLAttributes }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  const unfurlEmbed = graphql(`
    mutation EmbedNodeView_UnfurlEmbed_Mutation($input: UnfurlEmbedInput!) {
      unfurlEmbed(input: $input) {
        id
        url
        title
        description
        thumbnailUrl
        html
      }
    }
  `);

  let inflightUrl = $state<string>();
  let inflight = $state(false);
  let pickerOpened = $state(false);
  let inputEl = $state<HTMLInputElement>();
  let embedContainerEl = $state<HTMLDivElement>();

  $effect(() => {
    pickerOpened = selected;
  });

  $effect(() => {
    if (pickerOpened) {
      tick().then(() => {
        inputEl?.focus();
      });
    }
  });

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    onClickOutside: () => {
      pickerOpened = false;
    },
  });

  const handleInsert = async () => {
    if (!inflightUrl) {
      return;
    }

    inflight = true;
    try {
      const attrs = await unfurlEmbed({
        url: /^[^:]+:\/\//.test(inflightUrl) ? inflightUrl : `https://${inflightUrl}`,
      });

      updateAttributes(attrs);
    } finally {
      inflight = false;
    }
  };

  export const handle = (event: CustomEvent) => {
    if (event.type === 'inflight') {
      inflight = true;
      inflightUrl = event.detail.url;
    } else if (event.type === 'success') {
      inflight = false;
      inflightUrl = undefined;
      updateAttributes(event.detail.attrs);
    } else if (event.type === 'error') {
      inflight = false;
      inflightUrl = undefined;
    }
  };

  onMount(() => {
    if (!document.querySelector('script#iframely-embed')) {
      const script = document.createElement('script');
      script.id = 'iframely-embed';
      script.async = true;
      script.src = 'https://cdn.iframe.ly/embed.js';
      document.head.append(script);
    }

    if (attrs.html && attrs.title) {
      const iframe = embedContainerEl?.querySelector('iframe');
      if (iframe) {
        iframe.setAttribute('title', attrs.title);
      }
    }
  });
</script>

<NodeView data-drag-handle draggable {...HTMLAttributes}>
  <div class={cx('group', css({ position: 'relative' }))}>
    {#if attrs.id}
      {#if attrs.html}
        <div bind:this={embedContainerEl} class={css({ display: 'contents' }, editor?.current.isEditable && { pointerEvents: 'none' })}>
          <!-- eslint-disable-next-line svelte/no-at-html-tags -->
          {@html attrs.html}
        </div>
      {:else}
        <svelte:element
          this={editor?.current.isEditable ? 'div' : 'a'}
          class={flex({ borderWidth: '1px', borderColor: 'border.subtle', borderRadius: '6px' })}
          {...!editor?.current.isEditable && { href: attrs.url, target: '_blank', rel: 'noopener noreferrer' }}
        >
          <div class={flex({ direction: 'column', grow: '1', paddingX: '16px', paddingY: '15px' })}>
            <p class={css({ marginBottom: '3px', fontSize: '14px', fontWeight: 'medium', lineClamp: 1 })}>
              {attrs.title ?? '(제목 없음)'}
            </p>

            <p class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint', lineClamp: 2, whiteSpace: 'pre-line' })}>
              {attrs.description ?? ''}
            </p>

            <p class={css({ marginTop: 'auto', fontSize: '12px', fontWeight: 'medium', lineClamp: 1 })}>{new URL(attrs.url).origin}</p>
          </div>

          {#if attrs.thumbnailUrl}
            <img
              class={css({
                flexShrink: '0',
                borderTopRightRadius: '5px',
                borderBottomRightRadius: '5px',
                size: '118px',
                objectFit: 'cover',
              })}
              alt={attrs.title ?? '(제목 없음)'}
              src={attrs.thumbnailUrl}
            />
          {/if}
        </svelte:element>
      {/if}

      {#if editor?.current.isEditable && !window.__webview__}
        <button
          class={css({
            position: 'absolute',
            top: '20px',
            right: '20px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: '4px',
            color: 'text.bright',
            backgroundColor: '[#363839/70]',
            size: '28px',
            opacity: '0',
            transition: 'opacity',
            _hover: { backgroundColor: '[#363839/40]' },
            _groupHover: { opacity: '100' },
          })}
          onclick={() => deleteNode()}
          type="button"
        >
          <Icon icon={Trash2Icon} size={16} />
        </button>
      {/if}
    {:else}
      <div
        class={flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          borderRadius: '4px',
          backgroundColor: 'surface.muted',
        })}
        use:anchor
      >
        <div
          class={flex({
            align: 'center',
            gap: '12px',
            paddingX: '14px',
            paddingY: '12px',
            fontSize: '14px',
            color: 'text.disabled',
          })}
        >
          {#if inflight}
            <RingSpinner style={css.raw({ size: '20px' })} />
            링크 임베드 중...
          {:else}
            <Icon icon={FileUpIcon} size={20} />
            {#if editor?.current.isEditable}
              링크 임베드(Youtube, Google Drive, 일반 링크 등)
            {:else}
              링크 임베드 없음
            {/if}
          {/if}
        </div>

        {#if editor?.current.isEditable && !window.__webview__}
          <Menu>
            {#snippet button({ open })}
              <div
                class={css(
                  {
                    marginRight: '12px',
                    borderRadius: '4px',
                    padding: '2px',
                    color: 'text.disabled',
                    opacity: '0',
                    transition: 'common',
                    _hover: { backgroundColor: 'interactive.hover' },
                    _groupHover: { opacity: '100' },
                  },
                  open && { opacity: '100' },
                )}
              >
                <Icon icon={EllipsisIcon} size={20} />
              </div>
            {/snippet}

            <MenuItem onclick={() => deleteNode()} variant="danger">
              <Icon icon={Trash2Icon} size={12} />
              <span>삭제</span>
            </MenuItem>
          </Menu>
        {/if}
      </div>
    {/if}
  </div>
</NodeView>

{#if pickerOpened && !attrs.id && !inflight && editor?.current.isEditable && !window.__webview__}
  <form
    class={center({
      flexDirection: 'column',
      gap: '12px',
      borderWidth: '1px',
      borderRadius: '12px',
      padding: '12px',
      width: '380px',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
      zIndex: '1',
    })}
    onsubmit={(e) => {
      e.preventDefault();
      handleInsert();
    }}
    use:floating
  >
    <span class={css({ fontSize: '13px', color: 'text.muted', textAlign: 'center' })}>
      Youtube, Google Drive, 일반 링크 등
      <br />
      다양한 콘텐츠를 임베드할 수 있어요
    </span>

    <TextInput
      name="url"
      style={css.raw({ width: 'full' })}
      placeholder="https://..."
      size="sm"
      bind:element={inputEl}
      bind:value={inflightUrl}
    />

    <Button style={css.raw({ width: 'full' })} size="sm" type="submit">확인</Button>
  </form>
{/if}
