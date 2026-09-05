<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { hoverIntent, pointerCapture, scrollFog, tooltip } from '@typie/ui/actions';
  import { Icon, ProgressRing, Scrollbar } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import BarChart3Icon from '~icons/lucide/bar-chart-3';
  import CommandIcon from '~icons/lucide/command';
  import FolderPlusIcon from '~icons/lucide/folder-plus';
  import HomeIcon from '~icons/lucide/home';
  import SearchIcon from '~icons/lucide/search';
  import SquarePenIcon from '~icons/lucide/square-pen';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import TargetIcon from '~icons/lucide/target';
  import PrismIcon from '~icons/typie/prism';
  import { goto } from '$app/navigation';
  import { dailyGoalStatus, mergeTodayCharacterCountChanges, writingStreaks } from '$lib/user-stats';
  import { graphql } from '$mearie';
  import { getPaneGroup } from './[slug]/@pane/context.svelte';
  import ChangelogPopover from './@changelog/ChangelogPopover.svelte';
  import PrismBadgeDot from './@prism/PrismBadgeDot.svelte';
  import { SubscribeModal } from './@subscription/subscribe-modal.svelte';
  import TrialWidget from './@subscription/TrialWidget.svelte';
  import { createEntityTreeRevealRequest, entityTreeRevealState } from './@tree/entity-reveal.svelte';
  import EntityTree from './@tree/EntityTree.svelte';
  import QuickAccess from './@tree/QuickAccess.svelte';
  import SidebarSectionHeader from './@tree/SidebarSectionHeader.svelte';
  import { setupTreeContext } from './@tree/state.svelte';
  import { getEntityTreeElement } from './@tree/utils';
  import { getDayClock } from './day-clock.svelte';
  import Profile from './Profile.svelte';
  import { resolveSidebarNavigationDrag, resolveSidebarNavigationGeometry } from './sidebar-navigation-resize';
  import SpaceMenu from './SpaceMenu.svelte';
  import type { DashboardLayout_Sidebar_user$key } from '$mearie';
  import type { SidebarNavigationResizeSession } from './sidebar-navigation-resize';

  const IMMEDIATE_EDGE_WIDTH = 6;
  const INTENT_EDGE_WIDTH = 12;

  type Props = {
    user$key: DashboardLayout_Sidebar_user$key;
  };

  let { user$key }: Props = $props();

  const app = getAppContext();
  const dayClock = getDayClock();
  setupTreeContext();

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
          ...DashboardLayout_QuickAccess_site
        }

        characterCountChanges {
          date
          additions
        }

        todayCharacterCountChange {
          date
          additions
        }

        goal {
          targetCharacterCount
        }

        goalHistory {
          date
          additions
          achieved
        }

        ...DashboardLayout_SpaceMenu_user
        ...DashboardLayout_Profile_user
        ...DashboardLayout_TrialWidget_user
      }
    `),
    () => user$key,
  );

  const currentStreak = $derived.by(() => {
    const today = dayClock.now;
    const characterCountChanges = mergeTodayCharacterCountChanges(
      user.data.characterCountChanges,
      user.data.todayCharacterCountChange,
      today,
    );
    return writingStreaks(characterCountChanges, today).current;
  });

  const dailyGoal = $derived.by(() => {
    if (!user.data.goal) return null;
    return {
      ...dailyGoalStatus(user.data.goalHistory, user.data.goal.targetCharacterCount, user.data.todayCharacterCountChange, dayClock.now),
      target: user.data.goal.targetCharacterCount,
    };
  });

  const primaryNavigationItemClass = flex({
    alignItems: 'center',
    gap: '8px',
    paddingX: '8px',
    paddingY: '5px',
    borderRadius: '6px',
    transition: 'common',
    _supportHover: { backgroundColor: 'surface.hover' },
    '&[aria-current="page"]': { backgroundColor: 'surface.active' },
  });
  const primaryNavigationLabelClass = css({
    fontSize: '13px',
    fontWeight: 'medium',
    color: 'text.muted',
    '[aria-current="page"] > &': { fontWeight: 'bold', color: 'text.default' },
  });
  const primaryNavigationIconStyle = css.raw({ flexShrink: '0', color: 'text.muted' });
  const primaryNavigationShortcutClass = flex({ alignItems: 'center', marginLeft: 'auto', color: 'text.hint', fontSize: '11px' });

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

    const currentEl = getEntityTreeElement()?.querySelector<HTMLElement>(`[data-slug="${app.state.current}"]`);
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
    startWidth: number;
    startX: number;
  };

  let navigationScrollEl = $state<HTMLDivElement>();
  let navigationIntrinsicHeight = $state(0);
  let navigationMinimumContentHeight = $state(0);
  let navigationClipPreview = $state<number | null>(null);
  const navigationScrollId = 'sidebar-primary-navigation-scroll';
  const NAVIGATION_SCROLL_FOG_SIZE = 8;

  const navigationMinimumHeight = $derived(
    Math.min(navigationMinimumContentHeight + NAVIGATION_SCROLL_FOG_SIZE, navigationIntrinsicHeight),
  );
  const storedNavigationClip = $derived(app.preference.current.sidebarNavigationClip ?? 0);
  const navigationGeometry = $derived(
    resolveSidebarNavigationGeometry(navigationIntrinsicHeight, navigationMinimumHeight, storedNavigationClip),
  );
  const currentNavigationClip = $derived(clamp(navigationClipPreview ?? navigationGeometry.clip, 0, navigationGeometry.maxClip));
  const navigationViewportHeight = $derived(
    navigationIntrinsicHeight > 0 ? `${navigationIntrinsicHeight - currentNavigationClip}px` : undefined,
  );

  const startNavigationResizer = (event: PointerEvent): SidebarNavigationResizeSession | null => {
    if (navigationIntrinsicHeight === 0 || !event.isPrimary || event.button !== 0) return null;
    event.preventDefault();
    navigationClipPreview = navigationGeometry.clip;
    return { startClip: navigationGeometry.clip, startY: event.clientY };
  };

  const moveNavigationResizer = (session: SidebarNavigationResizeSession, event: PointerEvent) => {
    navigationClipPreview = resolveSidebarNavigationDrag(session, event.clientY, navigationGeometry.maxClip).clip;
  };

  const endNavigationResizer = (session: SidebarNavigationResizeSession, event: PointerEvent) => {
    const { clip, clipChanged } = resolveSidebarNavigationDrag(session, event.clientY, navigationGeometry.maxClip);
    if (clipChanged) {
      app.preference.current.sidebarNavigationClip = clip;
    }
    navigationClipPreview = null;
  };

  const cancelNavigationResizer = () => {
    navigationClipPreview = null;
  };

  let treeScrollEl = $state<HTMLDivElement>();
  const treeScrollId = 'sidebar-entity-tree-scroll';
  let treeSectionHeaderHeight = $state(0);
  let allDocumentsOpen = $derived(app.preference.current.sidebarAllDocumentsOpen);
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
  let newWidth = $derived(clamp((resizer?.startWidth ?? app.preference.current.sidebarWidth ?? 240) + (resizer?.deltaX ?? 0), 240, 480));

  const finishResizer = (commit: boolean) => {
    if (!resizer) return;

    if (commit && resizer.deltaX !== 0) {
      app.preference.current.sidebarWidth = newWidth;
    }

    resizer = null;
  };

  const startResizer = (event: PointerEvent): Resizer | null => {
    if (resizer || !event.isPrimary || event.button !== 0) return null;

    resizer = {
      deltaX: 0,
      startWidth: app.preference.current.sidebarWidth ?? 240,
      startX: event.clientX,
    };
    return resizer;
  };

  const moveResizer = (session: Resizer, event: PointerEvent) => {
    if (!resizer) return;
    resizer.deltaX = Math.round(event.clientX - session.startX);
  };

  const endResizer = (session: Resizer, event: PointerEvent) => {
    moveResizer(session, event);
    finishResizer(true);
  };

  let spaceMenuOpen = $state(false);
  let profileOpen = $state(false);

  let hideTimeout: ReturnType<typeof setTimeout> | null = null;
  let hovered = $state(false);
  let edgeRevealHeld = $state(false);
  let edgeIntentEnabled = $state(false);

  type SidebarState = 'hidden' | 'visible';
  let sidebarEl = $state<HTMLDivElement>();
  let sidebarState = $state<SidebarState>('hidden');
  let animateTransform = $state(false);

  const revealSidebar = () => {
    animateTransform = true;
    sidebarState = 'visible';
  };

  const transform = $derived.by(() => {
    if (!app.preference.current.sidebarHidden) return 'translateX(0)';
    return sidebarState === 'visible' ? 'translateX(0)' : 'translateX(-100%)';
  });

  $effect(() => {
    const sidebarHidden = app.preference.current.sidebarHidden;

    untrack(() => {
      animateTransform = false;
      if (sidebarHidden) sidebarState = app.state.sidebarPeek ? 'visible' : 'hidden';
    });
  });

  const cancelHide = () => {
    if (hideTimeout === null) return;
    clearTimeout(hideTimeout);
    hideTimeout = null;
  };

  const shouldHide = () =>
    app.preference.current.sidebarHidden &&
    !hovered &&
    !edgeRevealHeld &&
    !app.state.sidebarPeek &&
    app.state.openMenuCount === 0 &&
    sidebarState !== 'hidden';

  $effect(() => {
    const hide = shouldHide();

    untrack(() => {
      cancelHide();
      if (!hide) return;

      hideTimeout = setTimeout(() => {
        hideTimeout = null;
        if (!shouldHide()) return;

        animateTransform = true;
        sidebarState = 'hidden';
      }, 300);
    });

    return cancelHide;
  });

  $effect(() => {
    if (sidebarState !== 'hidden') return;
    spaceMenuOpen = false;
    profileOpen = false;
  });

  $effect(() => {
    if (!app.state.sidebarPeek || !app.preference.current.sidebarHidden) return;

    untrack(revealSidebar);
  });

  const handleMouseEnter = () => {
    hovered = true;

    if (app.preference.current.sidebarHidden) revealSidebar();
  };

  const handleMouseLeave = () => {
    hovered = false;
  };

  type EdgeZone = 'immediate' | 'intent' | null;

  const sidebarFootprint = (event: PointerEvent) => {
    const rect = sidebarEl?.getBoundingClientRect();
    if (!rect || event.clientX < 0 || event.clientY < rect.top || event.clientY >= rect.bottom) return null;
    return rect;
  };

  const isEdgeRevealSurface = (target: EventTarget | null) => {
    if (!(target instanceof Element)) return false;

    const surface = target.closest('.main-container, [role="tabpanel"], [role="textbox"]');
    if (!surface || !surface.closest('.main-container')) return false;

    for (let element: Element | null = target; element && element !== surface; element = element.parentElement) {
      if (!(element instanceof HTMLElement)) continue;
      if (element.tabIndex >= 0 || element.hasAttribute('tabindex') || element.hasAttribute('role') || element.isContentEditable) {
        return false;
      }
    }
    return true;
  };

  const edgeZone = (event: PointerEvent): EdgeZone => {
    if (
      sidebarState !== 'hidden' ||
      event.pointerType === 'touch' ||
      !app.preference.current.sidebarHidden ||
      event.clientX < 0 ||
      event.clientX >= INTENT_EDGE_WIDTH
    ) {
      return null;
    }

    const rect = sidebarFootprint(event);
    if (!rect || !isEdgeRevealSurface(event.target)) return null;

    return event.clientX < IMMEDIATE_EDGE_WIDTH ? 'immediate' : 'intent';
  };

  const revealFromEdge = () => {
    edgeRevealHeld = true;
    revealSidebar();
  };

  const handleEdgeIntent = (event: PointerEvent) => {
    if (edgeZone(event) === 'intent') revealFromEdge();
  };

  const handleEdgePointerMove = (event: PointerEvent) => {
    if (edgeRevealHeld) {
      const rect = sidebarFootprint(event);
      if (!rect || event.clientX >= rect.width) edgeRevealHeld = false;
    }

    const zone = edgeZone(event);
    edgeIntentEnabled = zone === 'intent';
    if (zone === 'immediate') revealFromEdge();
  };

  const handleEdgePointerDown = (event: PointerEvent) => {
    if (edgeZone(event) !== 'immediate') return;
    event.preventDefault();
    event.stopPropagation();
    revealFromEdge();
  };

  const releaseEdgeReveal = () => {
    edgeIntentEnabled = false;
    edgeRevealHeld = false;
  };
</script>

<svelte:body use:hoverIntent={{ delay: 400, intentEnabled: edgeIntentEnabled, samples: 1, onIntent: handleEdgeIntent }} />

<svelte:window
  onblur={releaseEdgeReveal}
  onpointercancel={releaseEdgeReveal}
  onpointerdowncapture={handleEdgePointerDown}
  onpointermove={handleEdgePointerMove}
  onpointerout={(event) => {
    if (event.relatedTarget === null) releaseEdgeReveal();
  }}
/>

<div
  bind:this={sidebarEl}
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
    transitionDuration: animateTransform && app.preference.current.sidebarHidden ? '300ms' : '0ms',
    transitionTimingFunction: 'cubic-bezier(0.4, 0, 0.2, 1)',
    transitionProperty: '[transform]',
  })}
  aria-label="사이드바"
  data-zen-mode-closing-surface
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
      backgroundColor: 'surface.canvas',
      borderTopWidth: app.preference.current.sidebarHidden ? '1px' : undefined,
      borderBottomWidth: app.preference.current.sidebarHidden ? '1px' : undefined,
      borderRightWidth: '1px',
      borderColor: 'border.hairline',
      borderTopRightRadius: app.preference.current.sidebarHidden ? '12px' : undefined,
      borderBottomRightRadius: app.preference.current.sidebarHidden ? '12px' : undefined,
      boxShadow: 'sm',
      transitionProperty: '[border, border-radius, box-shadow]',
      transitionDuration: '150ms',
      transitionTimingFunction: 'ease',
      overflow: 'hidden',
    })}
  >
    <!-- 사이트 스위쳐 -->
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
    </div>

    <div class={flex({ flexDirection: 'column', flexGrow: '1', minHeight: '0', overflow: 'hidden' })}>
      <div
        style:height={navigationViewportHeight}
        class={css({ position: 'relative', flexShrink: '0', minHeight: '0', overflow: 'hidden' })}
      >
        <div
          bind:this={navigationScrollEl}
          id={navigationScrollId}
          class={css({ height: 'full', overflowY: 'auto', overflowX: 'hidden', scrollbar: 'hidden' })}
          use:scrollFog={{ orientation: 'vertical', size: NAVIGATION_SCROLL_FOG_SIZE }}
        >
          <!-- 주요 내비게이션 -->
          <div
            class={flex({ flexDirection: 'column', gap: '1px', paddingX: '12px', paddingBottom: '8px' })}
            bind:clientHeight={navigationIntrinsicHeight}
          >
            <div
              class={flex({ flexDirection: 'column', gap: '1px', paddingTop: '4px' })}
              bind:clientHeight={navigationMinimumContentHeight}
            >
              <a
                class={primaryNavigationItemClass}
                aria-current={paneGroup.panes.find((p) => p.id === paneGroup.state.current.focusedPaneId)?.kind === 'home'
                  ? 'page'
                  : undefined}
                href="/home"
              >
                <Icon style={primaryNavigationIconStyle} icon={HomeIcon} size={16} />
                <span class={primaryNavigationLabelClass}>홈</span>
              </a>

              <button class={primaryNavigationItemClass} onclick={() => (app.state.commandPaletteOpen = true)} type="button">
                <Icon style={primaryNavigationIconStyle} icon={SearchIcon} size={16} />
                <span class={primaryNavigationLabelClass}>검색</span>
                <div class={primaryNavigationShortcutClass}>
                  {#if navigator.platform.includes('Mac')}
                    <Icon style={css.raw({ marginRight: '2px' })} icon={CommandIcon} size={10} />
                  {:else}
                    <span>Ctrl+</span>
                  {/if}
                  <span>K</span>
                </div>
              </button>

              <button
                class={primaryNavigationItemClass}
                onclick={() => {
                  app.state.notesOpen = true;
                  mixpanel.track('open_notes_modal');
                }}
                type="button"
              >
                <Icon style={primaryNavigationIconStyle} icon={StickyNoteIcon} size={16} />
                <span class={primaryNavigationLabelClass}>노트</span>
                <div class={primaryNavigationShortcutClass}>
                  {#if navigator.platform.includes('Mac')}
                    <Icon style={css.raw({ marginRight: '2px' })} icon={CommandIcon} size={10} />
                  {:else}
                    <span>Ctrl+</span>
                  {/if}
                  <span>J</span>
                </div>
              </button>
            </div>

            <button
              class={primaryNavigationItemClass}
              onclick={() => {
                const next = !app.preference.current.prismPanelOpen;
                app.preference.current.prismPanelOpen = next;
                mixpanel.track(next ? 'open_prism_panel' : 'close_prism_panel', { via: 'sidebar' });
              }}
              type="button"
            >
              <span class={css({ position: 'relative', display: 'flex', flexShrink: '0' })}>
                <Icon style={primaryNavigationIconStyle} icon={PrismIcon} size={16} />
                {#if app.state.prismBadge}
                  <PrismBadgeDot />
                {/if}
              </span>
              <span class={primaryNavigationLabelClass}>PRISM</span>
              <div class={primaryNavigationShortcutClass}>
                {#if navigator.platform.includes('Mac')}
                  <Icon style={css.raw({ marginRight: '2px' })} icon={CommandIcon} size={10} />
                {:else}
                  <span>Ctrl+</span>
                {/if}
                <span>E</span>
              </div>
            </button>

            <button
              class={primaryNavigationItemClass}
              onclick={() => {
                app.state.userGoalOpen = true;
                mixpanel.track('open_user_goal_modal', { via: 'sidebar' });
              }}
              type="button"
            >
              <Icon style={primaryNavigationIconStyle} icon={TargetIcon} size={16} />
              <span class={primaryNavigationLabelClass}>일일 목표</span>
              {#if dailyGoal}
                <div class={flex({ alignItems: 'center', gap: '8px', marginLeft: 'auto' })}>
                  <ProgressRing
                    progress={dailyGoal.additions / dailyGoal.target}
                    size={16}
                    state={dailyGoal.achieved ? 'achieved' : 'under'}
                  />
                  {#if dailyGoal.streak > 0}
                    <span class={css({ fontSize: '11px', fontWeight: 'medium', color: 'text.hint' })}>{dailyGoal.streak}일 연속</span>
                  {/if}
                </div>
              {/if}
            </button>

            <button
              class={primaryNavigationItemClass}
              onclick={() => {
                app.state.statsOpen = true;
                mixpanel.track('open_stats_modal');
              }}
              type="button"
            >
              <Icon style={primaryNavigationIconStyle} icon={BarChart3Icon} size={16} />
              <span class={primaryNavigationLabelClass}>통계</span>
              {#if currentStreak > 0}
                <span class={css({ marginLeft: 'auto', fontSize: '11px', fontWeight: 'medium', color: 'text.hint' })}>
                  {currentStreak}일 연속
                </span>
              {/if}
            </button>
          </div>
        </div>

        <Scrollbar
          controls={navigationScrollId}
          label="주요 내비게이션 세로 스크롤"
          orientation="vertical"
          scrollContainer={navigationScrollEl}
          size="md"
        />
      </div>

      <div
        class={css({
          position: 'relative',
          flexShrink: '0',
          height: '9px',
          marginX: '12px',
          cursor: 'row-resize',
          touchAction: 'none',
          userSelect: 'none',
          '&:hover > div': { height: '2px', backgroundColor: 'border.emphasis' },
        })}
        use:pointerCapture={{
          start: startNavigationResizer,
          move: moveNavigationResizer,
          end: endNavigationResizer,
          cancel: cancelNavigationResizer,
        }}
      >
        <div
          class={css({
            position: 'absolute',
            top: '1/2',
            left: '8px',
            right: '8px',
            height: navigationClipPreview === null ? '1px' : '2px',
            borderRadius: 'full',
            backgroundColor: navigationClipPreview === null ? 'border.default' : 'border.emphasis',
            pointerEvents: 'none',
            transform: 'translateY(-50%)',
            transition: '[background-color 150ms ease, height 150ms ease]',
          })}
        ></div>
      </div>

      <div class={css({ position: 'relative', flexGrow: '1', minHeight: '0' })}>
        <div
          bind:this={treeScrollEl}
          id={treeScrollId}
          style:scroll-padding-block-start={`${treeSectionHeaderHeight}px`}
          class={flex({
            flexDirection: 'column',
            height: 'full',
            overflowY: 'auto',
            overflowX: 'hidden',
            scrollbar: 'hidden',
            borderBottomWidth: canScrollDown ? '1px' : '0',
            borderColor: 'border.hairline',
            transition: '[border-width 150ms ease]',
          })}
          data-entity-row-drag-scroll-surface
          onscroll={updateScrollState}
        >
          <QuickAccess {canScrollUp} site$key={site} bind:headerHeight={treeSectionHeaderHeight} />

          <!-- 문서 트리 -->
          <SidebarSectionHeader
            dividerVisible={canScrollUp}
            label="모두"
            onToggle={() => (app.preference.current.sidebarAllDocumentsOpen = !allDocumentsOpen)}
            open={allDocumentsOpen}
            bind:height={treeSectionHeaderHeight}
          >
            {#snippet actions()}
              <div class={flex({ alignItems: 'center', gap: '2px' })}>
                <button
                  class={center({
                    borderRadius: '4px',
                    size: '24px',
                    color: 'text.muted',
                    opacity: '50',
                    transition: 'common',
                    _hover: { color: 'text.default', opacity: '100' },
                    _focusVisible: { opacity: '100' },
                  })}
                  onclick={async () => {
                    if (!SubscribeModal.gate('sidebar_create_folder')) {
                      return;
                    }

                    const ancestorFolderIds = [...app.state.ancestors];
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

                    entityTreeRevealState.set(createEntityTreeRevealRequest(resp.createFolder.entity.id, ancestorFolderIds, true));
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
                    color: 'text.muted',
                    opacity: '50',
                    transition: 'common',
                    _hover: { color: 'text.default', opacity: '100' },
                    _focusVisible: { opacity: '100' },
                  })}
                  onclick={async () => {
                    if (!SubscribeModal.gate('sidebar_create_document')) {
                      return;
                    }

                    const ancestorFolderIds = [...app.state.ancestors];
                    const { lowerOrder, upperOrder } = getAdjacentOrders();

                    const resp = await createDocument({
                      input: {
                        siteId: site.id,
                        parentEntityId: app.state.ancestors.at(-1),
                        lowerOrder,
                        upperOrder,
                        v2: true,
                      },
                    });

                    mixpanel.track('create_document', { via: 'tree' });
                    const revealRequest = createEntityTreeRevealRequest(resp.createDocument.entity.id, ancestorFolderIds, false);
                    entityTreeRevealState.set(revealRequest);
                    await goto(`/${resp.createDocument.entity.slug}`);
                  }}
                  type="button"
                  use:tooltip={{ message: '새 문서 생성' }}
                >
                  <Icon icon={SquarePenIcon} />
                </button>
              </div>
            {/snippet}
          </SidebarSectionHeader>

          <EntityTree open={allDocumentsOpen} scrollContainer={treeScrollEl} site$key={site} />
        </div>

        <Scrollbar
          controls={treeScrollId}
          label="트리 세로 스크롤"
          orientation="vertical"
          scrollContainer={treeScrollEl}
          size="md"
          trackInsetStart={treeSectionHeaderHeight}
        />
      </div>
    </div>

    <TrialWidget user$key={user.data} />

    <!-- 프로필 -->
    <Profile user$key={user.data} bind:open={profileOpen} />
  </div>

  <ChangelogPopover />

  <div
    class={css({
      position: 'absolute',
      top: '0',
      right: '-4px',
      zIndex: 'sidebar',
      width: '8px',
      height: 'full',
      pointerEvents: sidebarState === 'hidden' && app.preference.current.sidebarHidden ? 'none' : undefined,
      cursor: 'col-resize',
      _hoverAfter: {
        content: '""',
        display: 'block',
        borderRightRadius: '4px',
        marginLeft: '4px',
        height: 'full',
        width: '2px',
        backgroundColor: 'border.emphasis',
        opacity: '50',
      },
    })}
    use:pointerCapture={{ start: startResizer, move: moveResizer, end: endResizer, cancel: () => finishResizer(false) }}
  ></div>
</div>
