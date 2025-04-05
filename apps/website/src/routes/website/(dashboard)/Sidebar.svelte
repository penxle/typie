<script lang="ts">
  import ChevronsLeftIcon from '~icons/lucide/chevrons-left';
  import PenLineIcon from '~icons/lucide/pen-line';
  import { goto } from '$app/navigation';
  import Logo from '$assets/logos/logo.svg?component';
  import { fragment, graphql } from '$graphql';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import PageList from './PageList.svelte';
  import type { DashboardLayout_Sidebar_user } from '$graphql';
  import type { Item } from './types';

  type Props = {
    $user: DashboardLayout_Sidebar_user;
    items: Item[];
  };

  let { $user: _user, items }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_Sidebar_user on User {
        id

        sites {
          id
        }
      }
    `),
  );

  const createPost = graphql(`
    mutation DashboardLayout_Sidebar_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const app = getAppContext();
</script>

<nav
  class={css({
    zIndex: '[1000]',
    flexGrow: '0',
    flexShrink: '0',
    width: app.preference.current.sidebarExpanded ? '240px' : '0',
    transitionDuration: '200ms',
    transitionProperty: 'width',
    transitionTimingFunction: 'ease',
    pointerEvents: 'none',
    height: 'full',
  })}
  aria-hidden={!app.preference.current.sidebarExpanded && !app.state.sidebarPopoverVisible}
>
  <div
    class={css({
      position: 'absolute',
      top: '0',
      left: '0',
      bottom: '0',
      zIndex: '50',
      display: 'flex',
      flexDirection: 'column',
      width: '0',
      overflow: 'visible',
      pointerEvents: 'none',
    })}
  >
    <div
      class={css(
        {
          display: 'flex',
          flexDirection: 'column',
          position: 'relative',
          pointerEvents: 'auto',
          visibility: 'visible',
          paddingTop: '0',
          width: '240px',
          opacity: '100',
          transitionDuration: '200ms',
          transitionTimingFunction: 'ease',
          transitionProperty: 'width, opacity, transform',
        },
        app.preference.current.sidebarExpanded
          ? {
              height: 'full',
              transform: 'translateX(0) translateY(0)',
            }
          : {
              height: 'auto',
              transform: 'translateX(0) translateY(59px)',
            },

        !app.preference.current.sidebarExpanded &&
          !app.state.sidebarPopoverVisible && {
            opacity: '0',
            transform: 'translateX(-220px) translateY(59px)',
          },
      )}
      onpointerenter={() => (app.state.sidebarPopoverVisible = true)}
      onpointerleave={() => (app.state.sidebarPopoverVisible = false)}
    >
      <div
        class={css({
          position: 'relative',
          top: '-15px',
          marginBottom: '-15px',
          height: '15px',
          width: '240px',
          backgroundColor: 'transparent',
        })}
        onpointerenter={() => (app.state.sidebarPopoverVisible = true)}
      ></div>
      <div
        class={css(
          { position: 'absolute', inset: '0', zIndex: '[-1]', display: app.preference.current.sidebarExpanded ? 'none' : 'block' },
          !app.preference.current.sidebarExpanded && app.state.sidebarPopoverVisible && { backgroundColor: 'white', boxShadow: 'medium' },
        )}
      ></div>

      <div
        class={css({
          position: 'relative',
          display: 'flex',
          flexDirection: 'column',
          height: 'full',
          maxHeight: app.preference.current.sidebarExpanded ? 'full' : '[calc(-118px + 100vh)]',
          overflowY: 'auto',
        })}
        onpointerenter={() => (app.state.sidebarPopoverVisible = true)}
        onpointerleave={() => (app.state.sidebarPopoverVisible = false)}
      >
        <div class={css({ position: 'sticky', top: '0', backgroundColor: 'white' })}>
          <div class={flex({ align: 'center', justify: 'space-between' })}>
            <Logo class={css({ height: '32px', flex: 'none' })} />

            <div class={flex({ align: 'center', gap: '4px' })}>
              {#if app.preference.current.sidebarExpanded}
                <button
                  onclick={() => {
                    app.preference.current.sidebarExpanded = false;
                    app.state.sidebarPopoverVisible = false;
                  }}
                  type="button"
                >
                  <Icon icon={ChevronsLeftIcon} />
                </button>
              {/if}

              <button
                onclick={async () => {
                  const resp = await createPost({
                    siteId: $user.sites[0].id,
                  });

                  await goto(`/${resp.entity.slug}`);
                }}
                type="button"
              >
                <Icon icon={PenLineIcon} />
              </button>
            </div>
          </div>

          <nav>
            <p>홈</p>
            <p>검색</p>
            <p>알림</p>
            <p>설정</p>
          </nav>
        </div>

        <hr class={css({ marginY: '20px', border: 'none', height: '1px', width: 'full', backgroundColor: 'gray.900' })} />

        <div class={css({ minHeight: '720px' })}>
          <p>보관함</p>

          <PageList {items} />
        </div>
      </div>
    </div>
  </div>
</nav>
