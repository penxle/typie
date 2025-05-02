<script lang="ts">
  import dayjs from 'dayjs';
  import GiftIcon from '~icons/lucide/gift';
  import { fragment, graphql } from '$graphql';
  import { HorizontalDivider, Icon, Modal } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { TiptapRenderer } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import SidebarButton from './SidebarButton.svelte';
  import type { DashboardLayout_Announcements_postView, List } from '$graphql';

  type Props = {
    $posts: List<DashboardLayout_Announcements_postView>;
  };

  let { $posts: _posts }: Props = $props();

  const postViews = fragment(
    _posts,
    graphql(`
      fragment DashboardLayout_Announcements_postView on PostView {
        id
        title
        createdAt

        body {
          __typename

          ... on PostViewBodyAvailable {
            content
          }
        }
      }
    `),
  );

  const app = getAppContext();
  let open = $state(false);

  const hasUnread = $derived($postViews.some((postView) => !app.preference.current.announcementViewedIds?.includes(postView.id)));

  $effect(() => {
    if (open) {
      app.preference.current.announcementViewedIds = $postViews.map((postView) => postView.id);
    }
  });
</script>

{#if $postViews.length > 0}
  <div class={css({ position: 'relative' })}>
    <SidebarButton active={open} icon={GiftIcon} label="타이피 새소식" onclick={() => (open = true)} />

    {#if hasUnread}
      <div
        class={css({ position: 'absolute', top: '4px', right: '4px', borderRadius: 'full', size: '4px', backgroundColor: 'red.500' })}
      ></div>
    {/if}
  </div>
{/if}

<Modal style={css.raw({ maxWidth: '600px' })} bind:open>
  <div class={center({ gap: '4px', padding: '12px' })}>
    <Icon style={css.raw({ color: 'gray.500' })} icon={GiftIcon} size={14} />
    <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'gray.500' })}>타이피 새소식</span>
  </div>

  <HorizontalDivider />

  <div class={flex({ flexDirection: 'column', gap: '24px', paddingX: '24px', paddingY: '16px', overflowY: 'auto' })}>
    {#each $postViews as postView, idx (postView.id)}
      <div class={flex({ flexDirection: 'column', gap: '16px' })}>
        <div class={flex({ flexDirection: 'column', gap: '4px' })}>
          <div class={css({ fontSize: '14px', color: 'gray.500' })}>{dayjs(postView.createdAt).formatAsDate()}</div>
          <div class={css({ fontSize: '18px', fontWeight: 'bold' })}>
            {postView.title}
          </div>
        </div>
        {#if postView.body?.__typename === 'PostViewBodyAvailable'}
          <TiptapRenderer content={postView.body.content} />
        {/if}
      </div>

      {#if idx !== $postViews.length - 1}
        <HorizontalDivider />
      {/if}
    {/each}
  </div>
</Modal>
