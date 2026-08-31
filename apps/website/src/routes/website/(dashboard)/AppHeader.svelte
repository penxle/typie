<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import PanelLeftIcon from '~icons/lucide/panel-left';
  import PrismIcon from '~icons/typie/prism';
  import PrismBadgeDot from './@prism/PrismBadgeDot.svelte';

  const app = getAppContext();

  const open = $derived(app.preference.current.prismPanelOpen);
  const hidden = $derived(app.preference.current.sidebarHidden);

  const button = css.raw({
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
</script>

<header
  class={css({
    position: 'relative',
    zIndex: app.preference.current.zenModeEnabled ? 'underEditor' : 'sidebar',
    display: 'flex',
    flexShrink: '0',
    alignItems: 'center',
    justifyContent: 'space-between',
    height: '36px',
    paddingX: '8px',
    backgroundColor: 'surface.default',
    borderBottomWidth: '1px',
    borderColor: 'border.subtle',
  })}
>
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
      app.preference.current.sidebarHidden = !hidden;
      mixpanel.track('toggle_sidebar_auto_hide', { enabled: app.preference.current.sidebarHidden });
    }}
    onmouseenter={() => (app.state.sidebarPeek = true)}
    onmouseleave={() => (app.state.sidebarPeek = false)}
    type="button"
    use:tooltip={{ message: hidden ? '사이드바 고정' : '사이드바 자동 숨김' }}
  >
    <Icon icon={PanelLeftIcon} size={14} />
  </button>

  <button
    class={css(button, open ? { backgroundColor: 'surface.muted' } : {})}
    aria-label={app.state.prismBadge ? 'PRISM 열기/닫기, 확인할 항목 있음' : 'PRISM 열기/닫기'}
    aria-pressed={open}
    onclick={() => {
      const next = !open;
      app.preference.current.prismPanelOpen = next;
      mixpanel.track(next ? 'open_prism_panel' : 'close_prism_panel', { via: 'header' });
    }}
    type="button"
    use:tooltip={{ message: open ? 'PRISM 닫기' : 'PRISM 열기', keys: ['Mod', 'E'] }}
  >
    <span class={css({ position: 'relative', display: 'flex', flexShrink: '0' })}>
      <Icon style={css.raw({ color: open ? 'text.default' : 'text.faint' })} icon={PrismIcon} size={14} />
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
        color: open ? 'text.default' : 'text.muted',
      })}
    >
      PRISM
    </span>
  </button>
</header>
