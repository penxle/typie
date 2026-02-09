<script lang="ts">
  import { hide } from '@floating-ui/dom';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Button, Icon, Menu, MenuItem, RingSpinner, TextInput } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { onMount, tick } from 'svelte';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileUpIcon from '~icons/lucide/file-up';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { graphql } from '$graphql';
  import { getEditor } from '$lib/editor/context';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import type { ExternalElement, ExternalElementData } from '$lib/editor/types';

  type EmbedData = Extract<ExternalElementData, { type: 'embed' }>;

  type Props = {
    el: ExternalElement;
  };

  let { el }: Props = $props();

  const editor = getEditor();

  let pickerOpened = $state(false);
  let inputEl = $state<HTMLInputElement>();
  let inflightUrl = $state<string>();
  let inflight = $state(false);

  const embedData = $derived(el.data as EmbedData);
  const isEditable = $derived(!editor.isReadOnly());
  const asset = $derived(embedData.id ? editor.embedAssets.get(embedData.id) : undefined);
  const hasEmbed = $derived(!!asset || inflight);

  const unfurlEmbed = graphql(`
    mutation ExternalEmbed_UnfurlEmbed_Mutation($input: UnfurlEmbedInput!) {
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

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    middleware: [hide()],
  });

  $effect(() => {
    pickerOpened = el.isSelected;
  });

  $effect(() => {
    if (pickerOpened && !hasEmbed) {
      tick().then(() => {
        inputEl?.focus();
      });
    }
  });

  const handleInsert = async () => {
    if (!inflightUrl) return;

    inflight = true;
    try {
      const url = /^[^:]+:\/\//.test(inflightUrl) ? inflightUrl : `https://${inflightUrl}`;
      const result = await unfurlEmbed({ url });

      editor.embedAssets.set(result.id, {
        id: result.id,
        url: result.url,
        title: result.title ?? null,
        description: result.description ?? null,
        thumbnailUrl: result.thumbnailUrl ?? null,
        html: result.html ?? null,
      });

      editor.dispatch({
        type: 'setEmbedId',
        nodeId: el.nodeId,
        embedId: result.id,
      });

      editor.focus();
    } catch (err) {
      console.error('Embed unfurl failed:', err);
      Toast.error('링크를 임베드할 수 없습니다.');
    } finally {
      inflight = false;
      inflightUrl = undefined;
    }
  };

  const handleDelete = () => {
    editor.dispatch({ type: 'deleteNode', nodeId: el.nodeId });
    editor.focus();
  };

  onMount(() => {
    if (!document.querySelector('script#iframely-embed')) {
      const script = document.createElement('script');
      script.id = 'iframely-embed';
      script.async = true;
      script.src = 'https://cdn.iframe.ly/embed.js';
      document.head.append(script);
    }
  });
</script>

<ExternalElementWrapper {el} minHeight={hasEmbed ? undefined : '48px'}>
  <div class={cx('group', css({ position: 'relative', width: 'full' }))} use:anchor>
    {#if asset}
      {#if asset.html}
        <div class={css({ display: 'contents' }, isEditable && { pointerEvents: 'none' })}>
          <!-- eslint-disable-next-line svelte/no-at-html-tags -->
          {@html asset.html}
        </div>
      {:else}
        <svelte:element
          this={isEditable ? 'div' : 'a'}
          class={flex({ borderWidth: '1px', borderColor: 'border.subtle', borderRadius: '6px' })}
          {...!isEditable && { href: asset.url, target: '_blank', rel: 'noopener noreferrer' }}
        >
          <div class={flex({ direction: 'column', grow: '1', paddingX: '16px', paddingY: '15px' })}>
            <p class={css({ marginBottom: '3px', fontSize: '14px', fontWeight: 'medium', lineClamp: 1 })}>
              {asset.title ?? '(제목 없음)'}
            </p>

            <p class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint', lineClamp: 2, whiteSpace: 'pre-line' })}>
              {asset.description ?? ''}
            </p>

            <p class={css({ marginTop: 'auto', fontSize: '12px', fontWeight: 'medium', lineClamp: 1 })}>
              {new URL(asset.url).origin}
            </p>
          </div>

          {#if asset.thumbnailUrl}
            <img
              class={css({
                flexShrink: '0',
                borderTopRightRadius: '5px',
                borderBottomRightRadius: '5px',
                size: '118px',
                objectFit: 'cover',
              })}
              alt={asset.title ?? '(제목 없음)'}
              src={asset.thumbnailUrl}
            />
          {/if}
        </svelte:element>
      {/if}

      {#if isEditable}
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
            zIndex: '10',
            _hover: { backgroundColor: '[#363839/40]' },
            _groupHover: { opacity: '100' },
          })}
          aria-label="임베드 삭제"
          onclick={handleDelete}
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
          width: 'full',
          height: '48px',
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
            {#if isEditable}
              링크 임베드(Youtube, Google Drive, 일반 링크 등)
            {:else}
              링크 임베드 없음
            {/if}
          {/if}
        </div>

        {#if isEditable}
          <Menu>
            {#snippet button({ open }: { open: boolean })}
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

            <MenuItem onclick={handleDelete} variant="danger">
              <Icon icon={Trash2Icon} size={12} />
              <span>삭제</span>
            </MenuItem>
          </Menu>
        {/if}
      </div>
    {/if}
  </div>
</ExternalElementWrapper>

{#if pickerOpened && !hasEmbed && isEditable}
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
      zIndex: 'editor',
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
