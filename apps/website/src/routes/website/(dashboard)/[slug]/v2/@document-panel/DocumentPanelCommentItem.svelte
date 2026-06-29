<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, TimeAgo } from '@typie/ui/components';
  import MapPinOffIcon from '~icons/lucide/map-pin-off';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import type { DocumentPanelV2CommentItem_thread$key } from '$mearie';

  type Props = {
    thread$key: DocumentPanelV2CommentItem_thread$key;
    active: boolean;
    locatable?: boolean;
    resolved?: boolean;
    onUnresolve?: () => void;
    onclick?: () => void;
  };
  let { thread$key, active, locatable = true, resolved = false, onUnresolve, onclick }: Props = $props();

  const thread = createFragment(
    graphql(`
      fragment DocumentPanelV2CommentItem_thread on DocumentCommentThread {
        id

        comments {
          id
          content
          createdAt

          user {
            id
            name
            avatar {
              id
              ...Img_image
            }
          }
        }
      }
    `),
    () => thread$key,
  );

  const root = $derived(thread.data.comments[0]);
  const rootClass = $derived(
    flex({
      width: 'full',
      gap: '8px',
      paddingX: '16px',
      paddingY: '10px',
      borderBottomWidth: '1px',
      borderColor: 'border.subtle',
      cursor: onclick ? 'pointer' : 'default',
      textAlign: 'left',
      backgroundColor: active ? 'surface.subtle' : 'transparent',
      transition: 'common',
      _hover: onclick ? { backgroundColor: 'surface.subtle' } : undefined,
    }),
  );
</script>

{#snippet content()}
  {#if onclick}
    <Img
      style={css.raw({ size: '20px', borderRadius: 'full', flexShrink: '0', marginTop: '1px' })}
      alt={root.user.name}
      image$key={root.user.avatar}
      size={24}
    />
  {:else}
    <div class={css({ size: '20px', borderRadius: 'full', flexShrink: '0', marginTop: '1px', backgroundColor: 'surface.muted' })}></div>
  {/if}

  <div class={css({ flexGrow: '1', minWidth: '0' })}>
    <div class={flex({ alignItems: 'center', gap: '4px', marginBottom: '2px' })}>
      <span class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default', truncate: true, minWidth: '0' })}>
        {root?.user.name ?? ''}
      </span>
      {#if root}
        <TimeAgo
          style={css.raw({ fontSize: '11px', color: 'text.faint', flexShrink: '0' })}
          timestamp={new Date(root.createdAt).getTime()}
        />
      {/if}
      {#if !locatable}
        <span
          class={flex({ alignItems: 'center', gap: '2px', marginLeft: 'auto', fontSize: '11px', color: 'text.faint', flexShrink: '0' })}
        >
          <Icon icon={MapPinOffIcon} size={12} />위치 없음
        </span>
      {:else if resolved || thread.data.comments.length > 1}
        <span class={flex({ alignItems: 'center', gap: '8px', marginLeft: 'auto', flexShrink: '0' })}>
          {#if thread.data.comments.length > 1}
            <span class={css({ fontSize: '11px', color: 'text.faint' })}>
              {thread.data.comments.length}
            </span>
          {/if}
          {#if resolved}
            <button
              class={css({
                fontSize: '11px',
                lineHeight: '[1.2]',
                color: 'accent.brand.default',
                _hover: { textDecoration: 'underline' },
              })}
              onclick={(e) => {
                e.stopPropagation();
                onUnresolve?.();
              }}
              type="button"
            >
              다시 열기
            </button>
          {/if}
        </span>
      {/if}
    </div>
    <p
      class={css({
        margin: '0',
        fontSize: '13px',
        lineHeight: '[1.4]',
        color: 'text.subtle',
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-word',
        lineClamp: 2,
      })}
    >
      {root?.content ?? ''}
    </p>
  </div>
{/snippet}

{#if onclick}
  <button class={rootClass} data-comment-panel-item {onclick} type="button">
    {@render content()}
  </button>
{:else}
  <div class={rootClass} data-comment-panel-item>
    {@render content()}
  </div>
{/if}
