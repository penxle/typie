<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { TimeAgo } from '@typie/ui/components';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import type { RemarkOverlay } from '$lib/editor/slate';

  type Props = {
    remark: RemarkOverlay;
    onclick: () => void;
  };

  let { remark, onclick }: Props = $props();

  const userQuery = createQuery(
    graphql(`
      query DocumentPanelRemarkItem_Query($userId: ID!) {
        userView(id: $userId) {
          id
          name
          avatar {
            id
            ...Img_image
          }
        }
      }
    `),
    () => ({ userId: remark.userId }),
  );
</script>

<button
  class={css({
    display: 'flex',
    gap: '8px',
    paddingX: '16px',
    paddingY: '10px',
    borderBottomWidth: '1px',
    borderColor: 'border.subtle',
    cursor: 'pointer',
    textAlign: 'left',
    backgroundColor: 'transparent',
    borderWidth: '0',
    borderTopWidth: '0',
    borderLeftWidth: '0',
    borderRightWidth: '0',
    transition: 'common',
    _hover: { backgroundColor: 'surface.subtle' },
  })}
  data-remark-panel-item
  {onclick}
  type="button"
>
  {#if userQuery.data?.userView.avatar}
    <Img
      style={css.raw({ width: '20px', height: '20px', borderRadius: 'full', flexShrink: '0', marginTop: '1px' })}
      alt={userQuery.data.userView.name}
      image$key={userQuery.data.userView.avatar}
      size={24}
    />
  {:else}
    <div
      class={css({
        width: '20px',
        height: '20px',
        borderRadius: 'full',
        flexShrink: '0',
        marginTop: '1px',
        backgroundColor: 'surface.muted',
      })}
    ></div>
  {/if}

  <div class={css({ flexGrow: '1', minWidth: '0' })}>
    <div class={css({ display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '2px' })}>
      {#if userQuery.data}
        <span
          class={css({
            fontSize: '13px',
            fontWeight: 'semibold',
            color: 'text.default',
            truncate: true,
            flexShrink: '1',
            minWidth: '0',
          })}
        >
          {userQuery.data.userView.name}
        </span>
        <TimeAgo style={{ fontSize: '11px', color: 'text.faint', flexShrink: '0' }} timestamp={remark.createdAt} />
      {:else}
        <div
          class={css({
            width: '60px',
            height: '[1lh]',
            fontSize: '13px',
            borderRadius: '4px',
            backgroundColor: 'surface.muted',
            animation: 'pulse 1.5s ease-in-out infinite',
          })}
        ></div>
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
        minHeight: '[2lh]',
      })}
    >
      {remark.text}
    </p>
  </div>
</button>
