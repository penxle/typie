<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import dayjs from 'dayjs';
  import { graphql } from '$graphql';
  import { Img } from '$lib/components';
  import type { RemarkOverlay } from '$lib/editor/slate';

  type Props = {
    remark: RemarkOverlay;
    onclick: () => void;
  };

  let { remark, onclick }: Props = $props();

  const userQuery = graphql(`
    query DocumentPanelRemarkItem_Query($userId: ID!) @client {
      userView(id: $userId) {
        id
        name
        avatar {
          id
          ...Img_image
        }
      }
    }
  `);

  $effect(() => {
    userQuery.load({ userId: remark.userId });
  });
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
  {#if $userQuery?.userView.avatar}
    <Img
      style={css.raw({ width: '20px', height: '20px', borderRadius: 'full', flexShrink: '0', marginTop: '1px' })}
      $image={$userQuery.userView.avatar}
      alt={$userQuery.userView.name}
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
      {#if $userQuery}
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
          {$userQuery.userView.name}
        </span>
        <span class={css({ fontSize: '11px', color: 'text.faint', flexShrink: '0' })}>
          {dayjs(remark.createdAt).fromNow()}
        </span>
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
