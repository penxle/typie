<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import BarChart3Icon from '~icons/lucide/bar-chart-3';
  import ChevronsLeftIcon from '~icons/lucide/chevrons-left';
  import ChevronsRightIcon from '~icons/lucide/chevrons-right';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import GripVerticalIcon from '~icons/lucide/grip-vertical';
  import HelpCircleIcon from '~icons/lucide/help-circle';
  import HomeIcon from '~icons/lucide/home';
  import NewspaperIcon from '~icons/lucide/newspaper';
  import SearchIcon from '~icons/lucide/search';
  import SettingsIcon from '~icons/lucide/settings';
  import ShieldUserIcon from '~icons/lucide/shield-user';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { goto, pushState } from '$app/navigation';
  import { page } from '$app/state';
  import { fragment, graphql } from '$graphql';
  import EntityTree from './@tree/EntityTree.svelte';
  import PlanUsageWidget from './PlanUsageWidget.svelte';
  import Profile from './Profile.svelte';
  import ThemeSwitch from './ThemeSwitch.svelte';
  import type { DashboardLayout_Sidebar_site, DashboardLayout_Sidebar_user } from '$graphql';

  type Props = {
    $site: DashboardLayout_Sidebar_site;
    $user: DashboardLayout_Sidebar_user;
  };

  let { $site: _site, $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_Sidebar_user on User {
        id
        role

        ...DashboardLayout_Profile_user
        ...DashboardLayout_PlanUsageWidget_user
      }
    `),
  );

  const site = fragment(
    _site,
    graphql(`
      fragment DashboardLayout_Sidebar_site on Site {
        id

        ...DashboardLayout_EntityTree_site
        ...DashboardLayout_PlanUsageWidget_site
        ...DashboardLayout_TrashModal_site
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

  const createFolder = graphql(`
    mutation DashboardLayout_Sidebar_CreateFolder_Mutation($input: CreateFolderInput!) {
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
  let newWidth = $derived(clamp((app.preference.current.sidebarWidth ?? 240) + (resizer?.deltaX ?? 0), 240, 480));

  let hideTimeout = $state<NodeJS.Timeout | null>(null);
  let hovered = $state(false);

  type SidebarState = 'hidden' | 'peeking' | 'visible';
  let sidebarState = $state<SidebarState>('hidden');

  const transform = $derived.by(() => {
    if (!app.preference.current.sidebarHidden) {
      return 'translateX(0)';
    }

    switch (sidebarState) {
      case 'hidden': {
        return 'translateX(-100%)';
      }
      case 'peeking': {
        return 'translateX(calc(-100% + 20px))';
      }
      case 'visible': {
        return 'translateX(0)';
      }
      default: {
        return 'translateX(-100%)';
      }
    }
  });

  $effect(() => {
    if (!app.preference.current.sidebarHidden) return;

    if (app.state.openMenuCount === 0 && !hovered) {
      untrack(() => {
        if (hideTimeout) {
          clearTimeout(hideTimeout);
        }

        hideTimeout = setTimeout(() => {
          sidebarState = 'hidden';
          hideTimeout = null;
        }, 300);
      });
    }
  });

  const handleMouseEnter = () => {
    hovered = true;

    if (hideTimeout) {
      clearTimeout(hideTimeout);
      hideTimeout = null;
    }

    if (app.preference.current.sidebarHidden && app.preference.current.sidebarTrigger === 'hover') {
      sidebarState = 'visible';
    }
  };

  const handleMouseLeave = () => {
    hovered = false;

    if (!app.preference.current.sidebarHidden) return;

    if (app.state.openMenuCount > 0) return;

    if (hideTimeout) {
      clearTimeout(hideTimeout);
    }

    hideTimeout = setTimeout(() => {
      if (app.state.openMenuCount === 0) {
        sidebarState = 'hidden';
      }
      hideTimeout = null;
    }, 300);
  };
</script>

{#if app.preference.current.sidebarTrigger === 'hover' && app.preference.current.sidebarHidden && sidebarState !== 'visible'}
  <div
    class={css({
      position: 'fixed',
      top: '128px',
      left: '0',
      width: '40px',
      height: '[calc(100vh - 256px)]',
      zIndex: 'sidebar',
    })}
    onmouseenter={() => {
      if (sidebarState === 'hidden') {
        sidebarState = 'peeking';
      }
    }}
    onmouseleave={() => {
      if (sidebarState !== 'visible') {
        sidebarState = 'hidden';
      }
    }}
    role="button"
    tabindex="-1"
  ></div>
{/if}

<div
  style:--min-width="240px"
  style:--width={`${newWidth}px`}
  style:--max-width="480px"
  style:transform
  class={css({
    position: app.preference.current.sidebarHidden ? 'fixed' : 'relative',
    top: app.preference.current.sidebarHidden ? '128px' : undefined,
    left: app.preference.current.sidebarHidden ? '0' : undefined,
    height: app.preference.current.sidebarHidden ? '[calc(100vh - 256px)]' : undefined,
    flexShrink: app.preference.current.sidebarHidden ? undefined : '0',
    minWidth: app.preference.current.sidebarHidden ? undefined : 'var(--min-width)',
    maxWidth: app.preference.current.sidebarHidden ? undefined : 'var(--max-width)',
    width: app.preference.current.sidebarHidden ? 'var(--width)' : undefined,
    zIndex: app.preference.current.sidebarHidden ? 'sidebar' : undefined,
    opacity: '100',
    transitionDuration: '300ms',
    transitionTimingFunction: 'cubic-bezier(0.4, 0, 0.2, 1)',
    transitionProperty: '[transform]',
  })}
  aria-label="사이드바"
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
  role="navigation"
>
  <div
    class={css({
      display: 'flex',
      flexDirection: 'column',
      minWidth: 'var(--min-width)',
      width: 'var(--width)',
      maxWidth: 'var(--max-width)',
      height: 'full',
      backgroundColor: 'surface.subtle',
      borderTopWidth: app.preference.current.sidebarHidden ? '1px' : undefined,
      borderBottomWidth: app.preference.current.sidebarHidden ? '1px' : undefined,
      borderRightWidth: '1px',
      borderColor: 'border.subtle',
      borderTopRightRadius: app.preference.current.sidebarHidden ? '12px' : undefined,
      borderBottomRightRadius: app.preference.current.sidebarHidden ? '12px' : undefined,
      boxShadow: 'card',
      transitionProperty: '[border, border-radius, box-shadow]',
      transitionDuration: '150ms',
      transitionTimingFunction: 'ease',
      overflow: 'hidden',
    })}
  >
    <div
      class={flex({
        justifyContent: 'space-between',
        alignItems: 'center',
        marginX: '8px',
        marginTop: '8px',
        gap: '8px',
      })}
    >
      <div class={css({ minWidth: '0', flexShrink: '1', overflow: 'hidden' })}>
        <Profile {$user} />
      </div>

      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <button
          class={center({
            borderRadius: '4px',
            size: '24px',
            color: 'text.faint',
            transition: 'common',
            _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
          })}
          onclick={() => {
            app.preference.current.sidebarHidden = !app.preference.current.sidebarHidden;
            if (app.preference.current.sidebarHidden) {
              sidebarState = 'visible';
              hovered = false;
              setTimeout(() => {
                if (!hovered) {
                  sidebarState = 'hidden';
                }
              }, 300);
            }
            mixpanel.track('toggle_sidebar_auto_hide', { enabled: app.preference.current.sidebarHidden });
          }}
          type="button"
          use:tooltip={{ message: app.preference.current.sidebarHidden ? '사이드바 고정' : '사이드바 자동 숨김' }}
        >
          <Icon icon={app.preference.current.sidebarHidden ? ChevronsRightIcon : ChevronsLeftIcon} size={14} />
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
            const resp = await createFolder({
              siteId: $site.id,
              name: '새 폴더',
            });

            mixpanel.track('create_folder', { via: 'tree' });

            app.state.newFolderId = resp.id;
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

    <div class={flex({ flexDirection: 'column', marginX: '8px', marginTop: '8px' })}>
      <a
        class={css({
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          paddingX: '8px',
          paddingY: '6px',
          borderRadius: '6px',
          transition: 'common',
          _supportHover: { backgroundColor: 'surface.muted' },
          '&[aria-current="page"]': {
            backgroundColor: 'surface.muted',
          },
        })}
        aria-current={page.route.id === '/website/(dashboard)/home' ? 'page' : undefined}
        href="/home"
      >
        <Icon style={css.raw({ color: 'text.faint' })} icon={HomeIcon} size={14} />
        <span
          class={css({
            fontSize: '14px',
            fontWeight: 'medium',
            color: 'text.muted',
            '[aria-current="page"] > &': {
              fontWeight: 'bold',
              color: 'text.default',
            },
          })}
        >
          홈
        </span>
      </a>

      <button
        class={flex({
          alignItems: 'center',
          gap: '6px',
          paddingX: '8px',
          paddingY: '6px',
          borderRadius: '6px',
          transition: 'common',
          _supportHover: { backgroundColor: 'surface.muted' },
        })}
        onclick={() => {
          app.state.notesOpen = true;
          mixpanel.track('open_notes_modal');
        }}
        type="button"
      >
        <Icon style={css.raw({ color: 'text.faint' })} icon={StickyNoteIcon} size={14} />
        <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.muted' })}>노트</span>
      </button>

      <button
        class={flex({
          alignItems: 'center',
          gap: '6px',
          paddingX: '8px',
          paddingY: '6px',
          borderRadius: '6px',
          transition: 'common',
          _supportHover: { backgroundColor: 'surface.muted' },
        })}
        onclick={() => (app.state.commandPaletteOpen = true)}
        type="button"
      >
        <Icon style={css.raw({ color: 'text.faint' })} icon={SearchIcon} size={14} />
        <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.muted' })}>검색</span>
      </button>

      <button
        class={flex({
          alignItems: 'center',
          gap: '6px',
          paddingX: '8px',
          paddingY: '6px',
          borderRadius: '6px',
          transition: 'common',
          _supportHover: { backgroundColor: 'surface.muted' },
        })}
        onclick={() => {
          app.state.statsOpen = true;
          mixpanel.track('open_stats_modal');
        }}
        type="button"
      >
        <Icon style={css.raw({ color: 'text.faint' })} icon={BarChart3Icon} size={14} />
        <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.muted' })}>통계</span>
      </button>
    </div>

    <div class={css({ marginX: '8px', paddingX: '8px', marginTop: '8px', paddingTop: '6px' })}>
      <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>포스트</span>
    </div>

    <div
      class={css({
        position: 'relative',
        flexGrow: '1',
        overflow: 'hidden',
        _before: {
          content: '""',
          position: 'absolute',
          top: '0',
          left: '0',
          right: '0',
          height: '12px',
          background: '[linear-gradient(to bottom, token(colors.surface.subtle), transparent)]',
          pointerEvents: 'none',
          zIndex: '1',
        },
        _after: {
          content: '""',
          position: 'absolute',
          bottom: '0',
          left: '0',
          right: '0',
          height: '12px',
          background: '[linear-gradient(to top, token(colors.surface.subtle), transparent)]',
          pointerEvents: 'none',
          zIndex: '1',
        },
      })}
    >
      <div
        class={css({
          position: 'absolute',
          inset: '0',
          paddingX: '8px',
          paddingY: '12px',
          overflowY: 'auto',
        })}
      >
        <EntityTree {$site} />
      </div>
    </div>

    <PlanUsageWidget {$site} {$user} />

    <div
      class={flex({
        justifyContent: 'space-between',
        paddingY: '4px',
        paddingX: '8px',
        borderTopWidth: '1px',
        borderColor: 'border.default',
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <a
          class={center({
            borderRadius: '8px',
            size: '32px',
            color: 'text.faint',
            transition: 'common',
            _hover: {
              color: 'text.subtle',
              backgroundColor: 'surface.muted',
            },
          })}
          href="https://typie.link/help"
          rel="noopener noreferrer"
          target="_blank"
          use:tooltip={{ message: '고객센터', placement: 'top', offset: 8, trailing: '(새 탭)' }}
        >
          <Icon icon={HelpCircleIcon} size={16} />
        </a>

        <a
          class={center({
            borderRadius: '8px',
            size: '32px',
            color: 'text.faint',
            transition: 'common',
            _hover: {
              color: 'text.subtle',
              backgroundColor: 'surface.muted',
            },
          })}
          href="/changelog"
          rel="noopener noreferrer"
          target="_blank"
          use:tooltip={{ message: '업데이트 노트', placement: 'top', offset: 8, trailing: '(새 탭)' }}
        >
          <Icon icon={NewspaperIcon} size={16} />
        </a>

        {#if $user.role === 'ADMIN'}
          <a
            class={center({
              borderRadius: '8px',
              size: '32px',
              color: 'text.faint',
              transition: 'common',
              _hover: {
                color: 'text.subtle',
                backgroundColor: 'surface.muted',
              },
            })}
            href="/admin"
            use:tooltip={{ message: '어드민', placement: 'top', offset: 8 }}
          >
            <Icon icon={ShieldUserIcon} size={16} />
          </a>
        {/if}
      </div>

      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <button
          class={center({
            borderRadius: '8px',
            size: '32px',
            color: 'text.faint',
            transition: 'common',
            _hover: {
              color: 'text.subtle',
              backgroundColor: 'surface.muted',
            },
            '&[aria-pressed="true"]': {
              color: 'text.subtle',
              backgroundColor: 'surface.muted',
            },
          })}
          aria-pressed={app.state.trashOpen}
          data-type="trash"
          onclick={() => {
            app.state.trashOpen = true;
            mixpanel.track('open_trash_modal', { via: 'sidebar' });
          }}
          type="button"
          use:tooltip={{ message: '휴지통', placement: 'top', offset: 8 }}
        >
          <Icon icon={Trash2Icon} size={16} />
        </button>

        <button
          class={center({
            borderRadius: '8px',
            size: '32px',
            color: 'text.faint',
            transition: 'common',
            _hover: {
              color: 'text.subtle',
              backgroundColor: 'surface.muted',
            },
          })}
          onclick={() => {
            pushState('', { shallowRoute: '/preference/profile' });
            mixpanel.track('open_preference_modal', { via: 'sidebar' });
          }}
          type="button"
          use:tooltip={{ message: '설정', placement: 'top', offset: 8 }}
        >
          <Icon icon={SettingsIcon} size={16} />
        </button>

        <ThemeSwitch />
      </div>
    </div>
  </div>

  {#if app.preference.current.sidebarHidden}
    {#if app.preference.current.sidebarTrigger === 'click'}
      <button
        class={center({
          position: 'absolute',
          top: '8px',
          right: '-24px',
          width: '24px',
          height: '60px',
          backgroundColor: 'surface.subtle',
          borderWidth: '1px',
          borderLeftWidth: '0',
          borderColor: 'border.subtle',
          borderTopRightRadius: '12px',
          borderBottomRightRadius: '12px',
          boxShadow: 'card',
          color: 'text.faint',
          cursor: 'pointer',
          opacity: sidebarState === 'visible' ? '0' : '100',
          transform: sidebarState === 'visible' ? 'translateX(-100%)' : 'translateX(0)',
          transitionProperty: '[opacity, transform]',
          transitionDuration: '300ms',
          transitionDelay: '150ms',
          transitionTimingFunction: 'cubic-bezier(0.4, 0, 0.2, 1)',
          zIndex: '[-1]',
        })}
        aria-label="사이드바 열기"
        onclick={() => {
          if (sidebarState === 'hidden') {
            sidebarState = 'visible';
          }
        }}
        type="button"
      >
        <Icon icon={GripVerticalIcon} size={14} />
      </button>
    {:else if app.preference.current.sidebarTrigger === 'hover'}
      <div
        class={center({
          position: 'absolute',
          top: '8px',
          right: '-24px',
          width: '24px',
          height: '60px',
          backgroundColor: 'surface.subtle',
          borderWidth: '1px',
          borderLeftWidth: '0',
          borderColor: 'border.subtle',
          borderTopRightRadius: '12px',
          borderBottomRightRadius: '12px',
          boxShadow: 'card',
          color: 'text.faint',
          pointerEvents: 'none',
          opacity: sidebarState === 'visible' ? '0' : '100',
          transform: sidebarState === 'visible' ? 'translateX(-100%)' : 'translateX(0)',
          transitionProperty: '[opacity, transform]',
          transitionDuration: '300ms',
          transitionDelay: '150ms',
          transitionTimingFunction: 'cubic-bezier(0.4, 0, 0.2, 1)',
          zIndex: '[-1]',
        })}
        aria-label="사이드바"
      >
        <Icon icon={GripVerticalIcon} size={14} />
      </div>
    {/if}
  {/if}

  <div
    class={css({
      position: 'absolute',
      top: '0',
      right: '-6px',
      zIndex: app.preference.current.zenModeEnabled ? 'underEditor' : 'sidebar',
      width: '12px',
      height: 'full',
      cursor: 'col-resize',
      _hoverAfter: {
        content: '""',
        display: 'block',
        borderRightRadius: '4px',
        marginLeft: '4px',
        height: 'full',
        width: '2px',
        backgroundColor: 'border.strong',
        opacity: '50',
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

      app.preference.current.sidebarWidth = newWidth;

      resizer = null;
    }}
  ></div>
</div>
