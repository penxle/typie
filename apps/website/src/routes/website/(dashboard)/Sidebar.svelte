<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import BarChart3Icon from '~icons/lucide/bar-chart-3';
  import ChevronsLeftIcon from '~icons/lucide/chevrons-left';
  import ChevronsRightIcon from '~icons/lucide/chevrons-right';
  import CommandIcon from '~icons/lucide/command';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import GripVerticalIcon from '~icons/lucide/grip-vertical';
  import HomeIcon from '~icons/lucide/home';
  import SearchIcon from '~icons/lucide/search';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import { goto } from '$app/navigation';
  import { graphql } from '$mearie';
  import { getPaneGroup } from './[slug]/@pane/context.svelte';
  import EntityTree from './@tree/EntityTree.svelte';
  import PlanUsageWidget from './PlanUsageWidget.svelte';
  import Profile from './Profile.svelte';
  import SpaceMenu from './SpaceMenu.svelte';
  import type { DashboardLayout_Sidebar_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_Sidebar_user$key;
  };

  let { user$key }: Props = $props();

  const app = getAppContext();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_Sidebar_user on User {
        id
        name
        role

        avatar {
          id
          ...Img_image
        }

        sites {
          id
          name
          url

          logo {
            id
            ...Img_image
          }

          ...DashboardLayout_EntityTree_site
        }

        characterCountChanges {
          date
          additions
        }

        ...DashboardLayout_SpaceMenu_user
        ...DashboardLayout_Profile_user
        ...DashboardLayout_PlanUsageWidget_user
      }
    `),
    () => user$key,
  );

  const currentStreak = $derived.by(() => {
    const today = dayjs.kst().startOf('day');
    const activeDates = new Set(
      user.data.characterCountChanges.filter((c) => c.additions > 0).map((c) => dayjs(c.date as string).format('YYYY-MM-DD')),
    );

    let streak = 0;
    let checkDate = today;

    if (!activeDates.has(today.format('YYYY-MM-DD'))) {
      checkDate = today.subtract(1, 'day');
    }

    while (activeDates.has(checkDate.format('YYYY-MM-DD'))) {
      streak++;
      checkDate = checkDate.subtract(1, 'day');
    }

    return streak;
  });

  const site = $derived(user.data.sites.find((s) => s.id === app.preference.current.currentSiteId) ?? user.data.sites[0]);

  const [createDocument] = createMutation(
    graphql(`
      mutation DashboardLayout_Sidebar_CreateDocument_Mutation($input: CreateDocumentInput!) {
        createDocument(input: $input) {
          id

          entity {
            id
            slug

            container {
              ... on Site {
                id

                entities {
                  id

                  node {
                    __typename
                  }

                  ...DashboardLayout_EntityTree_Entity_entity
                }
              }

              ... on Entity {
                id

                children {
                  id

                  node {
                    __typename
                  }

                  ...DashboardLayout_EntityTree_Entity_entity
                }
              }
            }
          }
        }
      }
    `),
  );

  const [createFolder] = createMutation(
    graphql(`
      mutation DashboardLayout_Sidebar_CreateFolder_Mutation($input: CreateFolderInput!) {
        createFolder(input: $input) {
          id

          entity {
            id

            container {
              ... on Site {
                id

                entities {
                  id

                  node {
                    __typename
                  }

                  ...DashboardLayout_EntityTree_Entity_entity
                }
              }

              ... on Entity {
                id

                children {
                  id

                  node {
                    __typename
                  }

                  ...DashboardLayout_EntityTree_Entity_entity
                }
              }
            }
          }
        }
      }
    `),
  );

  const paneGroup = getPaneGroup();

  const getAdjacentOrders = () => {
    if (!app.state.current) return {};

    const currentEl = document.querySelector<HTMLElement>(`[data-slug="${app.state.current}"]`);
    if (!currentEl) return {};

    const lowerOrder = currentEl.dataset.order;

    let nextEl = currentEl.nextElementSibling as HTMLElement | null;
    while (nextEl && !Object.hasOwn(nextEl.dataset, 'id')) {
      nextEl = nextEl.nextElementSibling as HTMLElement | null;
    }
    const upperOrder = nextEl?.dataset.order;

    return { lowerOrder, upperOrder };
  };

  type Resizer = {
    deltaX: number;
    event: PointerEvent;
    element: HTMLElement;
  };

  let treeScrollEl = $state<HTMLDivElement>();
  let canScrollUp = $state(false);
  let canScrollDown = $state(false);

  const updateScrollState = () => {
    if (!treeScrollEl) return;
    canScrollUp = treeScrollEl.scrollTop > 0;
    canScrollDown = treeScrollEl.scrollTop + treeScrollEl.clientHeight < treeScrollEl.scrollHeight - 1;
  };

  $effect(() => {
    if (!treeScrollEl) return;
    updateScrollState();
    const observer = new ResizeObserver(updateScrollState);
    observer.observe(treeScrollEl);
    return () => observer.disconnect();
  });

  let resizer = $state<Resizer | null>(null);
  let newWidth = $derived(clamp((app.preference.current.sidebarWidth ?? 240) + (resizer?.deltaX ?? 0), 240, 480));

  const finishResizer = (commit: boolean) => {
    if (!resizer) return;

    if (commit && resizer.deltaX !== 0) {
      app.preference.current.sidebarWidth = newWidth;
    }

    resizer = null;
  };

  let spaceMenuOpen = $state(false);
  let profileOpen = $state(false);

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

  $effect(() => {
    if (sidebarState === 'hidden') {
      spaceMenuOpen = false;
      profileOpen = false;
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
      top: '0',
      bottom: '0',
      left: '0',
      width: '40px',
      height: '[clamp(min(480px,100vh), calc(100vh - 192px), 100vh)]',
      marginBlock: 'auto',
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
    top: app.preference.current.sidebarHidden ? '0' : undefined,
    bottom: app.preference.current.sidebarHidden ? '0' : undefined,
    left: app.preference.current.sidebarHidden ? '0' : undefined,
    marginBlock: app.preference.current.sidebarHidden ? 'auto' : undefined,
    height: app.preference.current.sidebarHidden ? '[clamp(min(480px,100vh), calc(100vh - 192px), 100vh)]' : undefined,
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
      overflowY: 'auto',
      overflowX: 'hidden',
    })}
  >
    <!-- 사이트 스위쳐 + 사이드바 토글 -->
    <div
      class={flex({
        alignItems: 'center',
        gap: '2px',
        paddingX: '12px',
        paddingTop: '12px',
        paddingBottom: '4px',
      })}
    >
      <SpaceMenu user$key={user.data} bind:open={spaceMenuOpen} />

      <button
        class={center({
          borderRadius: '6px',
          size: '28px',
          flexShrink: '0',
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
    </div>

    <!-- 스페이스 네비게이션 -->
    <div class={flex({ flexDirection: 'column', gap: '1px', paddingX: '12px', marginTop: '4px' })}>
      <a
        class={css({
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          paddingX: '8px',
          paddingY: '5px',
          borderRadius: '6px',
          transition: 'common',
          _supportHover: { backgroundColor: 'surface.muted' },
          '&[aria-current="page"]': {
            backgroundColor: 'surface.muted',
          },
        })}
        aria-current={paneGroup.panes.find((p) => p.id === paneGroup.state.current.focusedPaneId)?.kind === 'home' ? 'page' : undefined}
        href="/home"
      >
        <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={HomeIcon} size={16} />
        <span
          class={css({
            fontSize: '13px',
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
          gap: '8px',
          paddingX: '8px',
          paddingY: '5px',
          borderRadius: '6px',
          transition: 'common',
          _supportHover: { backgroundColor: 'surface.muted' },
        })}
        onclick={() => (app.state.commandPaletteOpen = true)}
        type="button"
      >
        <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={SearchIcon} size={16} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted' })}>검색</span>
        <div class={flex({ alignItems: 'center', marginLeft: 'auto', color: 'text.faint', fontSize: '11px' })}>
          {#if navigator.platform.includes('Mac')}
            <Icon style={css.raw({ marginRight: '2px' })} icon={CommandIcon} size={10} />
          {:else}
            <span>Ctrl+</span>
          {/if}
          <span>K</span>
        </div>
      </button>

      <button
        class={flex({
          alignItems: 'center',
          gap: '8px',
          paddingX: '8px',
          paddingY: '5px',
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
        <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={StickyNoteIcon} size={16} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted' })}>노트</span>
        <div class={flex({ alignItems: 'center', marginLeft: 'auto', color: 'text.faint', fontSize: '11px' })}>
          {#if navigator.platform.includes('Mac')}
            <Icon style={css.raw({ marginRight: '2px' })} icon={CommandIcon} size={10} />
          {:else}
            <span>Ctrl+</span>
          {/if}
          <span>J</span>
        </div>
      </button>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '1px', paddingX: '12px', marginTop: '8px' })}>
      <div class={css({ marginX: '8px', marginBottom: '6px', borderTopWidth: '1px', borderColor: 'border.subtle' })}></div>

      <button
        class={flex({
          alignItems: 'center',
          gap: '8px',
          paddingX: '8px',
          paddingY: '5px',
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
        <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={BarChart3Icon} size={16} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted' })}>통계</span>
        {#if currentStreak > 0}
          <span class={css({ marginLeft: 'auto', fontSize: '11px', fontWeight: 'medium', color: 'text.faint' })}>
            {currentStreak}일 연속
          </span>
        {/if}
      </button>
    </div>

    <div class={css({ marginX: '20px', marginTop: '8px', borderTopWidth: '1px', borderColor: 'border.subtle' })}></div>

    <!-- 문서 트리 -->
    <div
      class={flex({
        justifyContent: 'space-between',
        alignItems: 'center',
        paddingX: '12px',
        marginTop: '8px',
        marginBottom: '4px',
      })}
    >
      <span class={css({ paddingX: '8px', fontSize: '13px', fontWeight: 'semibold', color: 'text.faint' })}>글</span>

      <div class={flex({ alignItems: 'center', gap: '2px' })}>
        <button
          class={center({
            borderRadius: '4px',
            size: '24px',
            color: 'text.faint',
            transition: 'common',
            _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
          })}
          onclick={async () => {
            const { lowerOrder, upperOrder } = getAdjacentOrders();
            const resp = await createFolder({
              input: {
                siteId: site.id,
                name: '새 폴더',
                parentEntityId: app.state.ancestors.at(-1),
                lowerOrder,
                upperOrder,
              },
            });

            mixpanel.track('create_folder', { via: 'tree' });

            app.state.newFolderId = resp.createFolder.id;
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
            const { lowerOrder, upperOrder } = getAdjacentOrders();
            const resp = await createDocument({
              input: {
                siteId: site.id,
                parentEntityId: app.state.ancestors.at(-1),
                lowerOrder,
                upperOrder,
              },
            });

            mixpanel.track('create_document', { via: 'tree' });
            await goto(`/${resp.createDocument.entity.slug}`);
          }}
          type="button"
          use:tooltip={{ message: '새 문서 생성' }}
        >
          <Icon icon={SquarePenIcon} />
        </button>
      </div>
    </div>

    <div
      bind:this={treeScrollEl}
      class={css({
        flexGrow: '1',
        overflowY: 'auto',
        borderTopWidth: canScrollUp ? '1px' : '0',
        borderBottomWidth: canScrollDown ? '1px' : '0',
        borderColor: 'border.subtle',
        transition: '[border-width 150ms ease]',
      })}
      onscroll={updateScrollState}
    >
      <EntityTree site$key={site} />
    </div>

    <PlanUsageWidget user$key={user.data} />

    <!-- 프로필 -->
    <Profile user$key={user.data} bind:open={profileOpen} />
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
    onlostpointercapture={(e) => {
      if (!resizer || resizer.event.pointerId !== e.pointerId) return;
      finishResizer(true);
    }}
    onpointercancelcapture={(e) => {
      if (!resizer || resizer.event.pointerId !== e.pointerId) return;
      finishResizer(false);
    }}
    onpointerdowncapture={(e) => {
      if (e.button !== 0) return;

      const element = e.currentTarget;
      element.setPointerCapture(e.pointerId);

      resizer = {
        element,
        event: e,
        deltaX: 0,
      };
    }}
    onpointermovecapture={(e) => {
      if (!resizer || resizer.event.pointerId !== e.pointerId) return;

      resizer.deltaX = Math.round(e.clientX - resizer.event.clientX);
    }}
    onpointerupcapture={(e) => {
      if (!resizer || resizer.event.pointerId !== e.pointerId) return;
      finishResizer(true);
    }}
  ></div>
</div>
