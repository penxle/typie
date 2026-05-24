<script lang="ts">
  import { flip, hide } from '@floating-ui/dom';
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Button, Icon, RingSpinner, TextInput } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import FileUpIcon from '~icons/lucide/file-up';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { graphql } from '$mearie';
  import { getEditorContext } from '../editor.svelte';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const ctx = getEditorContext();

  const embedData = $derived(element.data.type === 'embed' ? element.data : undefined);
  const embedId = $derived(embedData?.id || undefined);
  const asset = $derived(embedId ? ctx.editor?.embedAssets.get(embedId) : undefined);
  const canEdit = $derived(!ctx.editor?.readOnly);

  let inflightUrl = $state('');
  let inflight = $state(false);
  let error = $state(false);

  const showUrlInput = $derived(element.is_selected && !embedId && !inflight && canEdit);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    middleware: [flip(), hide()],
  });

  const [unfurlEmbed] = createMutation(
    graphql(`
      mutation ExternalEmbed_UnfurlEmbed($input: UnfurlEmbedInput!) {
        unfurlEmbed(input: $input) {
          id
          url
          title
          description
          thumbnailUrl
          html
        }
      }
    `),
  );

  const handleSubmit = async () => {
    if (!inflightUrl || !ctx.editor) return;

    error = false;
    inflight = true;

    try {
      const url = /^[^:]+:\/\//.test(inflightUrl) ? inflightUrl : `https://${inflightUrl}`;
      const result = await unfurlEmbed({ input: { url } });

      ctx.editor.embedAssets.set(result.unfurlEmbed.id, {
        id: result.unfurlEmbed.id,
        url: result.unfurlEmbed.url,
        title: result.unfurlEmbed.title ?? null,
        description: result.unfurlEmbed.description ?? null,
        thumbnailUrl: result.unfurlEmbed.thumbnailUrl ?? null,
        html: result.unfurlEmbed.html ?? null,
      });

      ctx.editor.enqueue({
        type: 'node',
        op: { type: 'set_attrs', id: element.node_id, attrs: { type: 'embed', id: result.unfurlEmbed.id } },
      });

      ctx.editor.focus();
    } catch {
      error = true;
      Toast.error('링크를 임베드할 수 없습니다.');
    } finally {
      inflight = false;
      inflightUrl = '';
    }
  };

  const deleteNode = () => {
    ctx.editor?.enqueue({ type: 'node', op: { type: 'delete', id: element.node_id } });
    ctx.editor?.focus();
  };
</script>

<ExternalElementWrapper {element} minHeight={asset ? undefined : '48px'}>
  <div class={css({ position: 'relative', width: 'full' })}>
    {#if asset}
      {#if asset.html}
        <div class={css({ display: 'contents' }, canEdit && { pointerEvents: 'none' })}>
          <!-- eslint-disable-next-line svelte/no-at-html-tags -->
          {@html asset.html}
        </div>
      {:else}
        <div class={flex({ borderWidth: '1px', borderColor: 'border.subtle', borderRadius: '6px' })}>
          <div class={flex({ direction: 'column', grow: '1', paddingX: '16px', paddingY: '15px', gap: '4px', minWidth: '0' })}>
            <p class={css({ fontSize: '14px', fontWeight: 'medium', overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' })}>
              {asset.title ?? '(제목 없음)'}
            </p>
            {#if asset.description}
              <p class={css({ fontSize: '12px', color: 'text.faint', overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' })}>
                {asset.description}
              </p>
            {/if}
            <p
              class={css({
                fontSize: '12px',
                color: 'text.muted',
                marginTop: 'auto',
                overflow: 'hidden',
                whiteSpace: 'nowrap',
                textOverflow: 'ellipsis',
              })}
            >
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
        </div>
      {/if}

      <div class={flex({ position: 'absolute', top: '8px', right: '8px', gap: '4px' })}>
        <button
          class={css({
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: '4px',
            color: 'text.bright',
            backgroundColor: '[#363839/70]',
            size: '28px',
            _hover: { backgroundColor: '[#363839/40]' },
          })}
          aria-label="링크 열기"
          onclick={() => window.open(asset.url, '_blank', 'noopener,noreferrer')}
          onpointerdown={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          type="button"
        >
          <Icon icon={ExternalLinkIcon} size={16} />
        </button>

        {#if canEdit}
          <button
            class={css({
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              borderRadius: '4px',
              color: 'text.bright',
              backgroundColor: '[#363839/70]',
              size: '28px',
              _hover: { backgroundColor: '[#363839/40]' },
            })}
            aria-label="임베드 삭제"
            onclick={deleteNode}
            onpointerdown={(e) => {
              e.preventDefault();
              e.stopPropagation();
            }}
            type="button"
          >
            <Icon icon={Trash2Icon} size={16} />
          </button>
        {/if}
      </div>
    {:else}
      <div
        class={flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          borderRadius: '4px',
          backgroundColor: 'surface.muted',
          width: 'full',
          height: '48px',
          paddingX: '14px',
          paddingY: '12px',
        })}
        use:anchor
      >
        <div
          class={flex({ align: 'center', gap: '12px', fontSize: '14px', color: error ? 'text.danger' : 'text.disabled', minWidth: '0' })}
        >
          {#if inflight}
            <RingSpinner style={css.raw({ size: '20px', flexShrink: '0' })} />
            <span class={css({ overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' })}>링크 임베드 중...</span>
          {:else}
            <Icon class={css({ flexShrink: '0' })} icon={FileUpIcon} size={20} />
            <span class={css({ overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' })}>
              {#if error}
                링크를 임베드할 수 없습니다
              {:else if canEdit}
                링크 임베드 (Youtube, Google Drive, 일반 링크 등)
              {:else}
                링크 임베드 없음
              {/if}
            </span>
          {/if}
        </div>

        {#if canEdit && !inflight}
          <button
            class={flex({
              align: 'center',
              borderRadius: '4px',
              padding: '4px',
              color: 'text.disabled',
              _hover: { backgroundColor: 'interactive.hover', color: 'text.danger' },
            })}
            aria-label="임베드 삭제"
            onclick={deleteNode}
            onpointerdown={(e) => e.stopPropagation()}
            type="button"
          >
            <Icon icon={Trash2Icon} size={16} />
          </button>
        {/if}
      </div>
    {/if}
  </div>
</ExternalElementWrapper>

{#if showUrlInput}
  <form
    class={flex({
      alignItems: 'center',
      gap: '6px',
      borderWidth: '1px',
      borderRadius: '8px',
      paddingX: '6px',
      paddingY: '4px',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
      zIndex: 'editor',
    })}
    onsubmit={(e) => {
      e.preventDefault();
      void handleSubmit();
    }}
    use:floating
  >
    <TextInput name="url" style={css.raw({ flex: '1', minWidth: '200px' })} placeholder="https://..." size="sm" bind:value={inflightUrl} />
    <Button size="sm" type="submit">확인</Button>
  </form>
{/if}
