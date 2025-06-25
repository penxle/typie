<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import PanelLeftIcon from '~icons/lucide/panel-left';
  import PanelLeftDashedIcon from '~icons/lucide/panel-left-dashed';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { portal, tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import EntityTree from './@tree/EntityTree.svelte';
  import PlanUsageWidget from './PlanUsageWidget.svelte';
  import type { DashboardLayout_Posts_site, DashboardLayout_Posts_user } from '$graphql';

  type Props = {
    $site: DashboardLayout_Posts_site;
    $user: DashboardLayout_Posts_user;
  };

  let { $site: _site, $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_Posts_user on User {
        id

        ...DashboardLayout_PlanUsageWidget_user
      }
    `),
  );

  const site = fragment(
    _site,
    graphql(`
      fragment DashboardLayout_Posts_site on Site {
        id

        ...DashboardLayout_EntityTree_site
        ...DashboardLayout_PlanUsageWidget_site
      }
    `),
  );

  const createPost = graphql(`
    mutation DashboardLayout_Posts_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const createFolder = graphql(`
    mutation DashboardLayout_Posts_CreateFolder_Mutation($input: CreateFolderInput!) {
      createFolder(input: $input) {
        id
      }
    }
  `);

  const app = getAppContext();

  type Resizer = {
    deltaX: number;
    eligible: boolean;
    event: PointerEvent;
    element: HTMLElement;
  };

  let resizer = $state<Resizer | null>(null);
  let newWidth = $derived(Math.min(Math.max((app.preference.current.postsWidth ?? 240) + (resizer?.deltaX ?? 0), 240), 480));
</script>

{#if app.state.postsOpen && !app.preference.current.postsExpanded}
  <div
    class={css({ position: 'fixed', inset: '0', zIndex: '40' })}
    onclick={() => (app.state.postsOpen = false)}
    role="none"
    use:portal
  ></div>
{/if}

<div
  style:--min-width="240px"
  style:--width={`${newWidth}px`}
  style:--max-width="480px"
  style:--overflow="hidden"
  class={css(
    {
      flexShrink: '0',
      transitionDuration: '150ms',
      transitionTimingFunction: 'ease',
    },
    app.preference.current.postsExpanded
      ? {
          position: 'relative',
          marginY: '8px',
          marginRight: app.preference.current.postsExpanded === 'open' ? '4px' : '0',
          minWidth: app.preference.current.postsExpanded === 'open' ? 'var(--min-width)' : '0',
          maxWidth: app.preference.current.postsExpanded === 'open' ? 'var(--max-width)' : '0',
          opacity: app.preference.current.postsExpanded === 'open' ? '100' : '0',
          transitionProperty: 'min-width, max-width, opacity, position, margin-block',
        }
      : {
          position: 'fixed',
          left: app.state.postsOpen ? '64px' : '59px',
          insetY: '0',
          minWidth: app.state.postsOpen ? 'var(--min-width)' : '0',
          width: app.state.postsOpen ? 'var(--fixed-width, 0)' : '0',
          maxWidth: app.state.postsOpen ? 'var(--max-width)' : '0',
          opacity: app.state.postsOpen ? '100' : '0',
          zIndex: '50',
          transitionProperty: 'left, opacity, position, margin-block',
          overflow: 'var(--overflow)',
        },
  )}
  ontransitionendcapture={(e) => {
    if (!app.preference.current.postsExpanded && !app.state.postsOpen) {
      e.currentTarget.style.setProperty('--fixed-width', '0');
      e.currentTarget.style.setProperty('--overflow', 'hidden');
    }
  }}
  ontransitionstartcapture={(e) => {
    if (!app.preference.current.postsExpanded && app.state.postsOpen) {
      e.currentTarget.style.setProperty('--fixed-width', 'var(--width)');
      e.currentTarget.style.setProperty('--overflow', 'visible');
    }
  }}
>
  <div
    class={css(
      {
        display: 'flex',
        flexDirection: 'column',
        minWidth: 'var(--min-width)',
        width: 'var(--width)',
        maxWidth: 'var(--max-width)',
        height: 'full',
        backgroundColor: 'surface.default',
        transitionProperty: 'border, border-radius, box-shadow',
        transitionDuration: '150ms',
        transitionTimingFunction: 'ease',
        overflow: 'hidden',
      },
      app.preference.current.postsExpanded
        ? {
            borderWidth: '[0.5px]',
            borderRadius: '4px',
            boxShadow: '[0 3px 6px -2px {colors.shadow.default/3}, 0 1px 1px {colors.shadow.default/5}]',
          }
        : {
            borderColor: 'border.subtle',
            borderRightWidth: '1px',
            borderRightRadius: '4px',
            boxShadow: 'small',
          },
    )}
  >
    <div
      class={flex({
        justifyContent: 'space-between',
        alignItems: 'center',
        flexShrink: '0',
        gap: '4px',
        paddingX: '12px',
        paddingY: '8px',
        backgroundColor: 'surface.default',
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <span class={css({ fontSize: '14px', fontWeight: 'bold' })}>내 포스트</span>

        <button
          class={center({
            borderRadius: '4px',
            size: '20px',
            color: 'text.faint',
            transition: 'common',
            _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
          })}
          onclick={() => {
            if (app.preference.current.postsExpanded) {
              app.state.postsOpen = app.preference.current.postsExpanded === 'open';
              app.preference.current.postsExpanded = false;
              mixpanel.track('toggle_posts_expanded', { expanded: false });
            } else {
              app.preference.current.postsExpanded = app.state.postsOpen ? 'open' : 'closed';
              mixpanel.track('toggle_posts_expanded', { expanded: true });
            }
          }}
          type="button"
          use:tooltip={{ message: app.preference.current.postsExpanded ? '패널 고정 해제' : '패널 고정' }}
        >
          <Icon icon={app.preference.current.postsExpanded ? PanelLeftDashedIcon : PanelLeftIcon} size={14} />
        </button>
      </div>

      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <button
          class={center({
            borderRadius: '4px',
            size: '24px',
            color: 'text.faint',
            transition: 'common',
            _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
          })}
          onclick={async () => {
            await createFolder({
              siteId: $site.id,
              name: '새 폴더',
            });
            mixpanel.track('create_folder', { via: 'tree' });
          }}
          type="button"
          use:tooltip={{ message: '새 폴더 생성' }}
        >
          <Icon icon={FolderPlusIcon} />
        </button>

        <button
          class={center({
            borderRadius: '4px',
            size: '24px',
            color: 'text.faint',
            transition: 'common',
            _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
          })}
          onclick={async () => {
            const resp = await createPost({
              siteId: $site.id,
            });

            mixpanel.track('create_post', { via: 'tree' });

            await goto(`/${resp.entity.slug}`);
          }}
          type="button"
          use:tooltip={{ message: '새 포스트 생성' }}
        >
          <Icon icon={SquarePenIcon} />
        </button>
      </div>
    </div>

    <div
      class={css({
        flexGrow: '1',
        paddingX: '16px',
        paddingTop: '8px',
        paddingBottom: '32px',
        scrollPaddingY: '16px',
        overflowY: 'auto',
      })}
    >
      <EntityTree {$site} />
    </div>

    <PlanUsageWidget {$site} {$user} />
  </div>

  <div
    class={css({
      position: 'absolute',
      top: '0',
      right: '-6px',
      zIndex: '2',
      width: '12px',
      height: 'full',
      _hover: {
        cursor: 'col-resize',
        _after: {
          content: '""',
          display: 'block',
          borderRightRadius: '4px',
          marginLeft: '4px',
          height: 'full',
          width: '2px',
          backgroundColor: 'border.strong',
          opacity: '50',
        },
      },
    })}
    onpointerdowncapture={(e) => {
      resizer = {
        element: e.currentTarget,
        event: e,
        deltaX: 0,
        eligible: false,
      };
    }}
    onpointermovecapture={(e) => {
      if (!resizer) return;

      if (!resizer.eligible) {
        resizer.eligible = true;
        resizer.element.setPointerCapture(e.pointerId);
      }

      resizer.deltaX = Math.round(e.clientX - resizer.event.clientX);
    }}
    onpointerupcapture={() => {
      if (!resizer) return;

      if (resizer.eligible && resizer.element.hasPointerCapture(resizer.event.pointerId)) {
        resizer.element.releasePointerCapture(resizer.event.pointerId);
      }

      app.preference.current.postsWidth = newWidth;

      resizer = null;
    }}
  ></div>
</div>
