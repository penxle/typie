<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import PanelLeftIcon from '~icons/lucide/panel-left';
  import PrismIcon from '~icons/typie/prism';
  import HorizontalScrollLane from '$lib/editor-ffi/components/ui/HorizontalScrollLane.svelte';
  import PrismBadgeDot from '../../@prism/PrismBadgeDot.svelte';
  import { getPane, getPaneGroup } from './context.svelte';
  import { dragPane } from './dnd';
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
  const dragPaneProps = $derived({ paneGroup, paneId: pane.id });
  const prismPanelOpen = $derived(app.preference.current.prismPanelOpen);
  const sidebarHidden = $derived(app.preference.current.sidebarHidden);
  const globalControlsVisible = $derived(!app.preference.current.zenModeEnabled);
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
    if (!globalControlsVisible) {
      app.state.sidebarPeek = false;
      return;
    }

    return () => {
      app.state.sidebarPeek = false;
    };
  });
</script>

<div
  class={flex({
    alignItems: 'center',
    gap: '4px',
    minWidth: '0',
    flexShrink: '0',
    height: '36px',
    paddingLeft: globalControlsVisible && placement.topLeft ? '8px' : '4px',
    paddingRight: '8px',
    backgroundColor: 'surface.default',
    borderRadius: '4px',
    userSelect: 'none',
  })}
  role="region"
  use:dragPane={dragPaneProps}
>
  {#if globalControlsVisible && placement.topLeft}
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

  <div class={flex({ alignItems: 'center', flexBasis: '0', flexGrow: '1', minWidth: '0', height: 'full', overflowX: 'hidden' })}>
    {@render children()}
  </div>

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

  {#if globalControlsVisible && placement.topRight}
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

<HorizontalDivider color="secondary" />
