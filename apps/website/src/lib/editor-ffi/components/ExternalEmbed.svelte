<script lang="ts">
  import { flip, hide, shift } from '@floating-ui/dom';
  import { createMutation } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import LinkIcon from '~icons/lucide/link';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { graphql } from '$mearie';
  import { getEditorContext } from '../editor.svelte';
  import { createDeleteEmbedNodeMessage, processEmbedUpload } from '../handlers/embed-flow';
  import EmbedHtml from './EmbedHtml.svelte';
  import ExternalCard from './ExternalCard.svelte';
  import ExternalCardAction from './ExternalCardAction.svelte';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import ExternalMediaAction from './ExternalMediaAction.svelte';
  import ExternalMediaControls from './ExternalMediaControls.svelte';
  import ExternalPlaceholder from './ExternalPlaceholder.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  const LINK_CARD_WIDTH = 400;

  let { element }: Props = $props();

  const ctx = getEditorContext();

  const embedData = $derived(element.data.type === 'embed' ? element.data : undefined);
  const embedId = $derived(embedData?.id || undefined);
  const asset = $derived(embedId ? ctx.editor?.embedAssets.get(embedId) : undefined);
  const html = $derived(asset?.html);
  const inflight = $derived(ctx.editor?.inflightEmbeds.get(element.node));
  const canEdit = $derived(!ctx.editor?.readOnly);
  const displayZoom = $derived(ctx.editor?.safeDisplayZoom() ?? 1);

  let inflightUrl = $state('');
  let editing = $state(false);
  let urlInputEl = $state<HTMLInputElement>();
  let componentWidth = $state(0);
  let componentHeight = $state(0);

  const layoutReady = $derived(componentHeight > 0 && Math.abs(componentHeight - element.bounds.height) < 1);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 8,
    middleware: [flip(), shift({ padding: 8 }), hide()],
    onClickOutside: () => {
      editing = false;
    },
  });

  $effect(() => {
    if (!editing) return;
    return pushEscapeHandler(() => {
      editing = false;
      ctx.editor?.focus();
      return true;
    });
  });

  const embedOrigin = $derived.by(() => {
    if (!asset) return '';
    try {
      return new URL(asset.url).host;
    } catch {
      return asset.url;
    }
  });

  // The floating bar mounts into the body, so focus has to wait for it to land.
  $effect(() => {
    if (!editing || !urlInputEl) return;
    const timer = setTimeout(() => urlInputEl?.focus({ preventScroll: true }), 0);
    return () => clearTimeout(timer);
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
          faviconUrl
          html
        }
      }
    `),
  );

  const handleSubmit = async () => {
    const editor = ctx.editor;
    if (!inflightUrl || !editor) return;

    const url = inflightUrl;
    const uploadId = crypto.randomUUID();
    const isCurrent = () =>
      ctx.editor === editor &&
      !editor.destroyed &&
      !editor.readOnly &&
      editor.inflightEmbeds.get(element.node)?.uploadId === uploadId &&
      editor.appliedSnapshot.externalElements.some(
        (external) => external.node === element.node && external.data.type === 'embed' && !external.data.id,
      );

    const result = await processEmbedUpload({
      url,
      nodeId: element.node,
      setPending: () => editor.inflightEmbeds.set(element.node, { uploadId, url }),
      clearPending: () => {
        if (editor.inflightEmbeds.get(element.node)?.uploadId === uploadId) editor.inflightEmbeds.delete(element.node);
      },
      isCurrent,
      unfurl: async (normalizedUrl) => {
        const result = await unfurlEmbed({ input: { url: normalizedUrl } });
        return {
          id: result.unfurlEmbed.id,
          url: result.unfurlEmbed.url,
          title: result.unfurlEmbed.title ?? null,
          description: result.unfurlEmbed.description ?? null,
          thumbnailUrl: result.unfurlEmbed.thumbnailUrl ?? null,
          faviconUrl: result.unfurlEmbed.faviconUrl ?? null,
          html: result.unfurlEmbed.html ?? null,
        };
      },
      setEmbedAsset: (value) => editor.embedAssets.set(value.id, value),
      commit: (message) => {
        if (!isCurrent()) throw new Error('Embed upload is no longer current');
        const update = editor.updateNow(() => editor.enqueue(message));
        return update?.commandOutcomes.every((outcome) => outcome.type === 'applied') ?? false;
      },
    });

    if (result === 'uploaded') {
      editing = false;
      inflightUrl = '';
      editor.focus();
    } else if (result === 'failed') {
      Toast.error('링크를 임베드할 수 없습니다.');
    }
  };

  const deleteNode = () => {
    const editor = ctx.editor;
    if (!editor) return;

    editor.inflightEmbeds.delete(element.node);
    editor.enqueue(createDeleteEmbedNodeMessage(element.node));
    editor.focus();
  };
</script>

<ExternalElementWrapper {element}>
  <div
    class={cx('group', css({ position: 'relative', width: 'full' }))}
    bind:clientWidth={componentWidth}
    bind:clientHeight={componentHeight}
  >
    {#if asset && html}
      {#key html}
        <EmbedHtml
          class={css({ display: 'contents' }, canEdit && { pointerEvents: 'none' })}
          {html}
          {layoutReady}
          scrollRoot={ctx.editor?.scrollRootEl ?? null}
        >
          {#snippet fallback()}
            {@render linkCard()}
          {/snippet}
        </EmbedHtml>
      {/key}

      {#if canEdit}
        <ExternalMediaControls
          height={componentHeight}
          meta={`${asset.title ?? '(제목 없음)'} · ${embedOrigin}`}
          selected={element.is_selected}
          width={componentWidth}
          zoom={displayZoom}
        >
          <ExternalMediaAction
            icon={ExternalLinkIcon}
            label="링크 열기"
            onclick={() => window.open(asset.url, '_blank', 'noopener,noreferrer')}
          />
          <ExternalMediaAction icon={Trash2Icon} label="임베드 삭제" onclick={deleteNode} />
        </ExternalMediaControls>
      {/if}
    {:else if asset}
      {@render linkCard()}
    {:else if inflight}
      <ExternalCard icon={LinkIcon} meta="불러오는 중" spinner title={inflight.url} />
    {:else if embedId}
      <ExternalCard icon={LinkIcon} loading />
    {:else}
      <div use:anchor>
        <ExternalPlaceholder
          {canEdit}
          hint={canEdit ? 'Youtube, Google Drive, 일반 링크를 넣을 수 있어요' : undefined}
          icon={LinkIcon}
          onclick={() => (editing = true)}
          title={canEdit ? '링크 임베드 추가' : '비어있는 임베드'}
        >
          {#if canEdit}
            <ExternalCardAction danger icon={Trash2Icon} label="임베드 삭제" onclick={deleteNode} />
          {/if}
        </ExternalPlaceholder>
      </div>
    {/if}
  </div>
</ExternalElementWrapper>

{#if editing && canEdit}
  {@render urlForm()}
{/if}

{#snippet linkCard()}
  <div
    style:max-width={`${LINK_CARD_WIDTH}px`}
    class={css({
      marginX: 'auto',
      borderRadius: '8px',
      borderWidth: '1px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
      overflow: 'hidden',
    })}
  >
    {#if asset?.thumbnailUrl}
      <img
        class={css({
          display: 'block',
          width: 'full',
          aspectRatio: '[1200 / 630]',
          objectFit: 'cover',
          backgroundColor: 'surface.inset',
        })}
        alt=""
        src={asset.thumbnailUrl}
      />
    {/if}

    <div class={flex({ alignItems: 'center', gap: '12px', paddingX: '14px', paddingY: '12px' })}>
      <div class={flex({ direction: 'column', flexGrow: '1', minWidth: '0' })}>
        <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default', truncate: true })} data-selection-label>
          {asset?.title ?? '(제목 없음)'}
        </span>
        {#if asset?.description}
          <span class={css({ marginTop: '2px', fontSize: '12px', color: 'text.muted', truncate: true })} data-selection-label>
            {asset.description}
          </span>
        {/if}
        <span class={flex({ alignItems: 'center', gap: '6px', marginTop: '6px', minWidth: '0', fontSize: '12px', color: 'text.hint' })}>
          {#if asset?.faviconUrl}
            <img
              class={css({ display: 'block', flexShrink: '0', size: '14px', borderRadius: '2px', objectFit: 'cover' })}
              alt=""
              src={asset.faviconUrl}
            />
          {:else}
            <Icon icon={LinkIcon} size={14} />
          {/if}
          <span class={css({ truncate: true })} data-selection-label>{embedOrigin}</span>
        </span>
      </div>

      <div
        class={flex({
          alignItems: 'center',
          gap: '2px',
          flexShrink: '0',
          opacity: '0',
          transition: 'common',
          _groupActive: { opacity: '100' },
          _groupHover: { opacity: '100' },
          '&:focus-within': { opacity: '100' },
        })}
      >
        <ExternalCardAction
          icon={ExternalLinkIcon}
          label="링크 열기"
          onclick={() => asset && window.open(asset.url, '_blank', 'noopener,noreferrer')}
        />
        {#if canEdit}
          <ExternalCardAction danger icon={Trash2Icon} label="임베드 삭제" onclick={deleteNode} />
        {/if}
      </div>
    </div>
  </div>
{/snippet}

{#snippet urlForm()}
  <div
    class={`embed-url-bar ${css({
      zIndex: 'overEditor',
      borderRadius: '8px',
      backgroundColor: 'surface.default',
      boxShadow: 'lg',
      overflow: 'hidden',
      _dark: { borderWidth: '1px', borderColor: 'border.hairline' },
    })}`}
    use:floating
  >
    <div class={flex({ alignItems: 'center', gap: '8px', height: '36px', paddingLeft: '12px', paddingRight: '4px' })}>
      <Icon style={css.raw({ flexShrink: '0', color: 'text.hint' })} icon={LinkIcon} size={14} />

      <input
        bind:this={urlInputEl}
        class={css({
          flexGrow: '1',
          width: '208px',
          minWidth: '0',
          height: 'full',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.default',
          backgroundColor: 'transparent',
          border: 'none',
          outline: 'none',
          _placeholder: { color: 'text.hint', fontWeight: 'normal' },
        })}
        aria-label="링크"
        onkeydown={(e) => {
          if (e.isComposing) return;
          if (e.key === 'Enter') {
            e.preventDefault();
            void handleSubmit();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            editing = false;
          }
        }}
        placeholder="https://..."
        type="url"
        bind:value={inflightUrl}
      />

      <button
        class={css({
          flexShrink: '0',
          height: '28px',
          paddingX: '10px',
          borderRadius: '6px',
          fontSize: '12px',
          fontWeight: 'medium',
          color: 'text.default',
          backgroundColor: 'surface.inset',
          transition: 'common',
          _enabled: { _hover: { backgroundColor: 'surface.active' } },
          _disabled: { opacity: '40' },
        })}
        disabled={inflightUrl.trim().length === 0}
        onclick={() => void handleSubmit()}
        onpointerdown={(e) => e.preventDefault()}
        type="button"
      >
        삽입
      </button>
    </div>
  </div>
{/snippet}

<style>
  .embed-url-bar {
    transform-origin: top center;
    animation: embed-url-bar-in 150ms cubic-bezier(0.23, 1, 0.32, 1) both;
  }

  @keyframes embed-url-bar-in {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .embed-url-bar {
      animation: none;
    }
  }
</style>
