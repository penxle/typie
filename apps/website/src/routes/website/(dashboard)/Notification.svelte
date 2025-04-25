<script lang="ts">
  import dayjs from 'dayjs';
  import { fly } from 'svelte/transition';
  import BellIcon from '~icons/lucide/bell';
  import { fragment, graphql } from '$graphql';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import SidebarButton from './SidebarButton.svelte';
  import type { DashboardLayout_Notification_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_Notification_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_Notification_user on User {
        id

        notifications {
          id
          state

          data {
            __typename

            ... on NotificationAnnouncementData {
              link
              message
            }

            ... on NotificationCommentData {
              comment {
                id
                content
                createdAt
              }

              post {
                id
                title

                view {
                  id

                  entity {
                    id
                    url
                  }
                }
              }
            }
          }
        }
      }
    `),
  );

  const markAllNotificationsAsRead = graphql(`
    mutation DashboardLayout_Notification_MarkAllNotificationsAsRead_Mutation {
      markAllNotificationsAsRead {
        id
        state
      }
    }
  `);

  const markNotificationAsRead = graphql(`
    mutation DashboardLayout_Notification_MarkNotificationAsRead_Mutation($input: MarkNotificationAsReadInput!) {
      markNotificationAsRead(input: $input) {
        id
        state
      }
    }
  `);

  let open = $state(false);
</script>

<SidebarButton active={open} icon={BellIcon} label="알림" onclick={() => (open = true)} />

{#if open}
  <div class={css({ position: 'fixed', inset: '0', zIndex: '50' })}>
    <div class={css({ position: 'absolute', inset: '0' })} onclick={() => (open = false)} role="none"></div>
    <div
      class={flex({
        position: 'absolute',
        left: '64px',
        insetY: '0',
        flexDirection: 'column',
        borderRightWidth: '1px',
        borderColor: 'gray.100',
        borderRightRadius: '4px',
        paddingX: '6px',
        width: '350px',
        backgroundColor: 'white',
        boxShadow: 'small',
        overflowY: 'auto',
        zIndex: '50',
      })}
      transition:fly={{ x: -5, duration: 100 }}
    >
      <div
        class={css({
          position: 'sticky',
          top: '0',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexShrink: '0',
          paddingX: '12px',
          backgroundColor: 'white',
          height: '46px',
          zIndex: '1',
        })}
      >
        <p class={css({ fontSize: '14px', fontWeight: 'bold' })}>알림</p>

        <button
          class={css({
            borderRadius: '4px',
            fontSize: '14px',
            fontWeight: 'medium',
            color: 'brand.500',
            paddingX: '6px',
            paddingY: '2px',
            _hover: { backgroundColor: 'gray.100' },
          })}
          onclick={async () => {
            await markAllNotificationsAsRead();
          }}
          type="button"
        >
          모두 읽기
        </button>
      </div>

      <ul class={flex({ direction: 'column', gap: '4px', flexGrow: '1', marginTop: '4px', paddingBottom: '6px' })}>
        {#each $user.notifications as notification (notification.id)}
          <li
            class={css({
              borderRadius: '6px',
              fontSize: '14px',
              fontWeight: 'medium',
              color: 'gray.700',
              _hover: { backgroundColor: 'gray.100' },
            })}
          >
            <button
              class={flex({
                justify: 'space-between',
                gap: '8px',
                position: 'relative',
                paddingX: '12px',
                paddingY: '10px',
                width: 'full',
              })}
              onclick={async () => {
                if (notification.state === 'UNREAD') {
                  await markNotificationAsRead({ notificationId: notification.id });
                }

                const url =
                  notification.data.__typename === 'NotificationCommentData'
                    ? notification.data.post.view.entity.url
                    : notification.data.link;

                if (url) {
                  window.open(url, '_blank');
                }
              }}
              type="button"
            >
              {#if notification.state === 'UNREAD'}
                <div
                  class={css({
                    position: 'absolute',
                    top: '10px',
                    left: '6px',
                    borderRadius: 'full',
                    size: '4px',
                    backgroundColor: 'brand.500',
                  })}
                ></div>
              {/if}

              {#if notification.data.__typename === 'NotificationAnnouncementData'}
                <p>{notification.data.message}</p>
              {:else if notification.data.__typename === 'NotificationCommentData'}
                <p class={css({ textAlign: 'left', width: 'full' })}>
                  <strong
                    class={css({ display: 'inline-block', fontWeight: 'bold', verticalAlign: 'bottom', maxWidth: '200px', truncate: true })}
                  >
                    {notification.data.post.title}
                  </strong>
                  에 댓글이 달렸어요
                </p>

                <date
                  class={css({ display: 'block', flex: 'none', marginTop: '2px', fontSize: '12px', color: 'gray.400' })}
                  datetime={notification.data.comment.createdAt}
                >
                  {dayjs(notification.data.comment.createdAt).formatAsDateTime()}
                </date>
              {/if}
            </button>
          </li>
        {:else}
          <li
            class={css({
              marginY: 'auto',
              paddingBottom: '32px',
              fontSize: '14px',
              textAlign: 'center',
              fontWeight: 'medium',
              color: 'gray.400',
            })}
          >
            알림이 없어요
          </li>
        {/each}
      </ul>
    </div>
  </div>
{/if}
