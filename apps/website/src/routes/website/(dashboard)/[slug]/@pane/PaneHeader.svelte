<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import PanelLeftIcon from '~icons/lucide/panel-left';
  import PrismIcon from '~icons/typie/prism';
  import HorizontalScrollLane from '$lib/editor-ffi/components/ui/HorizontalScrollLane.svelte';
  import PrismBadgeDot from '../../@prism/PrismBadgeDot.svelte';
  import { getPane, getPaneGroup } from './context.svelte';
  import { dragPane } from './dnd';
  import { getZenModePaneChrome, PANE_CHROME_EXPANSION_EASING, paneChromeExpansionTiming } from './zen-mode-pane-chrome.svelte';
  import ZenModePaneChromeEffects from './ZenModePaneChromeEffects.svelte';
  import ZenModePaneChromeSegment from './ZenModePaneChromeSegment.svelte';
  import type { Snippet } from 'svelte';
  import type { PaneHeaderPlacement } from './types';

  type Props = {
    children: Snippet;
    fixedActions?: Snippet;
    placement: PaneHeaderPlacement;
    scrollableActions?: Snippet;
  };

  let { children, fixedActions, placement, scrollableActions }: Props = $props();

  const app = getAppContext();
  const pane = getPane();
  const paneGroup = getPaneGroup();
  const paneChrome = getZenModePaneChrome();
  const dragPaneProps = $derived({ paneGroup, paneId: pane.id });
  const prismPanelOpen = $derived(app.preference.current.prismPanelOpen);
  const sidebarHidden = $derived(app.preference.current.sidebarHidden);
  const zenModeEnabled = $derived(app.preference.current.zenModeEnabled);
  const scrollableActionsViewportId = `pane-header-actions-${pane.id}`;

  const prismButton = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    paddingX: '8px',
    paddingY: '4px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '6px',
    transition: 'common',
    _supportHover: { backgroundColor: 'surface.muted' },
  });

  $effect(() => {
    if (!placement.topLeft) return;
    return () => {
      app.state.sidebarPeek = false;
    };
  });

  const expansionTiming = $derived(paneChromeExpansionTiming(paneChrome.phase === 'expanding' ? paneChrome.expansionPace : 'standard'));
  const registerHeaderLane = paneChrome.registerHeaderLane;
  const registerIdentity = (node: HTMLElement) => paneChrome.registerSegment('identity', node);
  const registerActions = (node: HTMLElement) => paneChrome.registerSegment('actions', node);
</script>

<div
  style:pointer-events={zenModeEnabled ? 'none' : 'auto'}
  class={flex({
    position: zenModeEnabled ? 'absolute' : 'relative',
    top: zenModeEnabled ? '0' : undefined,
    left: zenModeEnabled ? '0' : undefined,
    right: zenModeEnabled ? '0' : undefined,
    zIndex: zenModeEnabled ? 'overEditor' : undefined,
    flexDirection: 'column',
    minWidth: '0',
    flexShrink: '0',
    height: '37px',
    backgroundColor: zenModeEnabled ? 'transparent' : 'surface.default',
    userSelect: 'none',
  })}
  data-pane-header-adjacent-to-prism={placement.topRight}
  data-zen-mode-pane-chrome
  data-zen-pane-chrome-surface-visible={zenModeEnabled && paneChrome.isLaneSurfaceVisible('header')}
  role="region"
  use:dragPane={dragPaneProps}
  use:registerHeaderLane
>
  {#if zenModeEnabled}
    <ZenModePaneChromeEffects lane="header" />

    <div
      style:--zen-pane-chrome-foreground-radius={`${paneChrome.foregroundRevealRadius('header')}px`}
      style:clip-path={paneChrome.headerLaneInteractionClip()}
      style:pointer-events={paneChrome.isHeaderLaneInteractive() ? 'auto' : 'none'}
      style:transition={`clip-path ${expansionTiming.backgroundExpandMs}ms ${PANE_CHROME_EXPANSION_EASING}, --zen-pane-chrome-foreground-radius ${expansionTiming.backgroundExpandMs}ms ${PANE_CHROME_EXPANSION_EASING}`}
      class={css({
        position: 'absolute',
        top: '0',
        right: '0',
        left: '0',
        height: '36px',
        cursor: 'grab',
        _motionReduce: { transitionDuration: '0ms' },
      })}
      aria-hidden="true"
      data-zen-pane-chrome-header-foreground-hit
    ></div>
  {/if}

  <div class={flex({ alignItems: 'center', width: 'full', height: '36px', flexShrink: '0' })}>
    <div
      class={zenModeEnabled
        ? flex({
            alignItems: 'center',
            flexBasis: '0',
            flexGrow: '1',
            flexShrink: '1',
            minWidth: placement.topLeft ? '[40px]' : '8px',
            height: 'full',
          })
        : css({ display: 'contents' })}
      data-pane-header-leading-lane
    >
      <ZenModePaneChromeSegment
        class={flex({
          position: 'relative',
          alignItems: 'center',
          gap: '4px',
          flexBasis: zenModeEnabled ? undefined : '0',
          flexGrow: zenModeEnabled ? undefined : '1',
          flexShrink: '1',
          minWidth: placement.topLeft ? '[32px]' : '0',
          height: 'full',
          paddingLeft: placement.topLeft ? '8px' : '4px',
        })}
        active={zenModeEnabled}
        aria-label="문서 위치"
        contentCursor="grab"
        register={registerIdentity}
        segment="identity"
      >
        <div class={css({ position: 'relative', display: 'contents' })}>
          {#if placement.topLeft}
            <button
              class={center({
                size: '24px',
                flexShrink: '0',
                borderRadius: '6px',
                color: 'text.faint',
                transition: 'common',
                _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
              })}
              onclick={() => {
                app.state.sidebarPeek = false;
                app.preference.current.sidebarHidden = !sidebarHidden;
                mixpanel.track('toggle_sidebar_auto_hide', { enabled: app.preference.current.sidebarHidden });
              }}
              onmouseenter={() => (app.state.sidebarPeek = true)}
              onmouseleave={() => (app.state.sidebarPeek = false)}
              type="button"
              use:tooltip={{ message: sidebarHidden ? '사이드바 고정' : '사이드바 자동 숨김' }}
            >
              <Icon icon={PanelLeftIcon} size={14} />
            </button>
          {/if}

          <div class={flex({ alignItems: 'center', minWidth: '0', height: 'full', overflowX: 'hidden' })}>
            {@render children()}
          </div>
        </div>
      </ZenModePaneChromeSegment>

      <div
        style:cursor={zenModeEnabled && paneChrome.isHeaderGapInteractive() ? 'grab' : undefined}
        style:pointer-events={zenModeEnabled ? (paneChrome.isHeaderGapInteractive() ? 'auto' : 'none') : undefined}
        class={css({ flexGrow: zenModeEnabled ? '1' : undefined, minWidth: zenModeEnabled ? '8px' : '0' })}
      ></div>
    </div>

    <ZenModePaneChromeSegment
      class={flex({
        position: 'relative',
        alignItems: 'center',
        gap: '4px',
        flexShrink: '1',
        minWidth: '0',
        height: 'full',
        overflowX: 'hidden',
        paddingRight: '8px',
      })}
      active={zenModeEnabled}
      aria-label="pane 도구"
      contentCursor="grab"
      register={registerActions}
      segment="actions"
    >
      <div class={flex({ position: 'relative', alignItems: 'center', gap: '4px', minWidth: '0', height: 'full' })}>
        {#if scrollableActions}
          <HorizontalScrollLane
            alignment="start"
            contentIdentity={pane.id}
            label="문서 헤더 도구 가로 스크롤"
            viewportId={scrollableActionsViewportId}
            viewportName="pane-header-actions"
          >
            <div class={flex({ alignItems: 'center', gap: '4px', minWidth: '[max-content]' })}>
              {@render scrollableActions()}
            </div>
          </HorizontalScrollLane>
        {/if}

        {#if fixedActions}
          <div class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}>
            {@render fixedActions()}
          </div>
        {/if}

        {#if placement.topRight}
          <button
            class={css(prismButton, prismPanelOpen ? { backgroundColor: 'surface.muted' } : {})}
            aria-label={app.state.prismBadge ? 'PRISM 열기/닫기, 확인할 항목 있음' : 'PRISM 열기/닫기'}
            aria-pressed={prismPanelOpen}
            onclick={() => {
              const next = !prismPanelOpen;
              app.preference.current.prismPanelOpen = next;
              mixpanel.track(next ? 'open_prism_panel' : 'close_prism_panel', { via: 'header' });
            }}
            type="button"
            use:tooltip={{ message: prismPanelOpen ? 'PRISM 닫기' : 'PRISM 열기', keys: ['Mod', 'E'] }}
          >
            <span class={css({ position: 'relative', display: 'flex', flexShrink: '0' })}>
              <Icon style={css.raw({ color: prismPanelOpen ? 'text.default' : 'text.faint' })} icon={PrismIcon} size={14} />
              {#if app.state.prismBadge}
                <PrismBadgeDot />
              {/if}
            </span>
            <span
              class={css({
                fontSize: '12px',
                fontWeight: 'semibold',
                letterSpacing: '[0.04em]',
                lineHeight: '[1]',
                color: prismPanelOpen ? 'text.default' : 'text.muted',
              })}
            >
              PRISM
            </span>
          </button>
        {/if}
      </div>
    </ZenModePaneChromeSegment>
  </div>

  <div
    class={css({ width: 'full', height: '1px', flexShrink: '0', backgroundColor: zenModeEnabled ? 'transparent' : 'surface.muted' })}
    aria-hidden="true"
    data-pane-header-boundary
  ></div>
</div>
