<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { untrack } from 'svelte';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import PlusIcon from '~icons/lucide/plus';
  import XIcon from '~icons/lucide/x';
  import type { Snippet } from 'svelte';

  type Props = { tabs: TabState[]; activeId: string | null; children?: Snippet };
  let { tabs, activeId, children }: Props = $props();

  const isMac = window.shell.platform === 'darwin';
  const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
  const CLOSE_MS = 140;
  const SETTLE_MS = 140;
  const DRAG_THRESHOLD = 8;

  type Drag = { id: string; index: number; startX: number; dx: number; width: number; active: boolean; target: number; settling: boolean };

  let scroller = $state<HTMLDivElement | null>(null);
  let overflowLeft = $state(false);
  let overflowRight = $state(false);
  let closing = $state<string[]>([]);
  let entering = $state<string | null>(null);
  let drag = $state<Drag | null>(null);
  let pendingPointerAdd = false;
  let knownIds = new Set<string>();

  const updateOverflow = () => {
    if (!scroller) return;
    overflowLeft = scroller.scrollLeft > 1;
    overflowRight = scroller.scrollLeft + scroller.clientWidth < scroller.scrollWidth - 1;
  };

  $effect(() => {
    if (!scroller) return;
    const observer = new ResizeObserver(updateOverflow);
    observer.observe(scroller);
    const onWheel = (event: WheelEvent) => {
      if (!scroller || Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
      scroller.scrollLeft += event.deltaY;
      event.preventDefault();
    };
    scroller.addEventListener('wheel', onWheel, { passive: false });
    return () => {
      observer.disconnect();
      scroller?.removeEventListener('wheel', onWheel);
    };
  });

  $effect(() => {
    const ids = new Set(tabs.map((tab) => tab.id));
    if (pendingPointerAdd && !reduceMotion.matches) {
      const added = tabs.find((tab) => !knownIds.has(tab.id));
      if (added) {
        entering = added.id;
        requestAnimationFrame(() => requestAnimationFrame(() => (entering = null)));
      }
    }
    pendingPointerAdd = false;
    knownIds = ids;
    const stale = untrack(() => closing).filter((id) => !ids.has(id));
    if (stale.length > 0) closing = untrack(() => closing).filter((id) => ids.has(id));
    requestAnimationFrame(updateOverflow);
  });

  $effect(() => {
    if (!activeId || !scroller) return;
    const element = scroller.querySelector<HTMLElement>(`[data-id="${activeId}"]`);
    element?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  });

  const startClose = (id: string) => {
    if (closing.includes(id)) return;
    if (reduceMotion.matches) {
      window.shell.closeTab?.(id);
      return;
    }
    closing = [...closing, id];
    setTimeout(() => window.shell.closeTab?.(id), CLOSE_MS);
  };

  const onPointerDown = (event: PointerEvent, tab: TabState, index: number) => {
    if (event.button !== 0 || (event.target as HTMLElement).closest('.close')) return;
    if (tab.id !== activeId) window.shell.activateTab?.(tab.id);
    const element = event.currentTarget as HTMLElement;
    element.setPointerCapture(event.pointerId);
    drag = {
      id: tab.id,
      index,
      startX: event.clientX,
      dx: 0,
      width: element.getBoundingClientRect().width,
      active: false,
      target: index,
      settling: false,
    };
  };

  const onPointerMove = (event: PointerEvent) => {
    if (!drag || drag.settling) return;
    const raw = event.clientX - drag.startX;
    if (!drag.active && Math.abs(raw) < DRAG_THRESHOLD) return;
    const min = -drag.index * drag.width;
    const max = (tabs.length - 1 - drag.index) * drag.width;
    drag.active = true;
    drag.dx = Math.max(min, Math.min(max, raw));
    drag.target = Math.max(0, Math.min(tabs.length - 1, drag.index + Math.round(drag.dx / drag.width)));
  };

  const finishDrag = () => {
    if (!drag) return;
    if (!drag.active) {
      drag = null;
      return;
    }
    const { id, index, target, width } = drag;
    const commit = () => {
      if (target !== index) window.shell.moveTab?.(id, target);
      drag = null;
    };
    if (reduceMotion.matches) {
      commit();
      return;
    }
    drag.settling = true;
    drag.dx = (target - index) * width;
    setTimeout(commit, SETTLE_MS);
  };

  const shiftFor = (index: number) => {
    if (!drag?.active) return 0;
    if (index === drag.index) return drag.dx;
    if (drag.index < drag.target && index > drag.index && index <= drag.target) return -drag.width;
    if (drag.index > drag.target && index < drag.index && index >= drag.target) return drag.width;
    return 0;
  };

  const maskImage = $derived.by(() => {
    if (!overflowLeft && !overflowRight) return;
    const from = overflowLeft ? 'transparent, black 24px' : 'black, black 24px';
    const to = overflowRight ? 'black calc(100% - 24px), transparent' : 'black calc(100% - 24px), black';
    return `linear-gradient(to right, ${from}, ${to})`;
  });

  const tabClass = css({
    position: 'relative',
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    flexShrink: '1',
    width: '[200px]',
    minWidth: '[80px]',
    height: '28px',
    paddingLeft: '12px',
    paddingRight: '6px',
    borderRadius: '8px',
    fontSize: '13px',
    fontWeight: 'medium',
    color: 'text.subtle',
    cursor: 'default',
    outline: 'none',
    transitionProperty: '[width, min-width, padding, opacity, transform, color, background-color, box-shadow]',
    transitionDuration: '[140ms]',
    transitionTimingFunction: '[cubic-bezier(0.23, 1, 0.32, 1)]',
    '&:hover': { color: 'text.default', backgroundColor: 'interactive.hover' },
    '&[data-active="true"]': {
      backgroundColor: 'surface.default',
      color: 'text.default',
      fontWeight: 'semibold',
      boxShadow: '[0 0 0 1px {colors.border.subtle}, 0 1px 2px {colors.shadow.default/6}]',
    },
    '&:hover .close, &[data-active="true"] .close, & .close:focus-visible': { opacity: '100', pointerEvents: 'auto' },
    '&[data-closing="true"], &[data-entering="true"]': {
      width: '[0px]',
      minWidth: '[0px]',
      paddingLeft: '0',
      paddingRight: '0',
      opacity: '0',
      overflow: 'hidden',
    },
    '&[data-entering="true"]': { transitionDuration: '[0ms]' },
    '&[data-dragging="true"]': { zIndex: '2', backgroundColor: 'surface.default', boxShadow: 'small' },
    _focusVisible: { boxShadow: '[inset 0 0 0 2px {colors.accent.brand.default}]' },
    _motionReduce: { transitionDuration: '[0ms]' },
  });

  const closeClass = css({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: '0',
    size: '18px',
    borderRadius: 'full',
    color: 'text.faint',
    opacity: '0',
    pointerEvents: 'none',
    transitionProperty: '[opacity, background-color, color, transform]',
    transitionDuration: '[100ms]',
    _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
    _active: { transform: 'scale(0.94)' },
    _focusVisible: { boxShadow: '[0 0 0 2px {colors.accent.brand.default}]' },
  });

  const iconButtonStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: '0',
    size: '28px',
    marginY: '6px',
    borderRadius: 'full',
    color: 'text.subtle',
    transitionProperty: '[background-color, color, transform, opacity]',
    transitionDuration: '[100ms]',
    _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
    _active: { transform: 'scale(0.95)' },
    _focusVisible: { boxShadow: '[0 0 0 2px {colors.accent.brand.default}]' },
  });

  const iconButtonClass = css(iconButtonStyle);
  const newTabClass = css(iconButtonStyle, { marginLeft: '6px' });
</script>

<div
  style:-webkit-app-region="drag"
  class={css({
    position: 'relative',
    display: 'flex',
    alignItems: 'stretch',
    height: '[40px]',
    paddingLeft: isMac ? '[96px]' : '[8px]',
    paddingRight: isMac ? '[8px]' : '[140px]',
    backgroundColor: 'surface.subtle',
    userSelect: 'none',
    _after: { content: '""', position: 'absolute', left: '0', right: '0', bottom: '0', height: '1px', backgroundColor: 'border.subtle' },
  })}
>
  <div
    bind:this={scroller}
    style:-webkit-app-region="no-drag"
    style:mask-image={maskImage}
    class={css({
      display: 'flex',
      alignItems: 'center',
      gap: '4px',
      flexShrink: '1',
      minWidth: '0',
      height: 'full',
      paddingY: '6px',
      overflowX: 'auto',
      overflowY: 'hidden',
      scrollbarWidth: 'none',
    })}
    onscroll={updateOverflow}
    role="tablist"
  >
    {#each tabs as tab, index (tab.id)}
      {@const active = tab.id === activeId}
      {@const dragging = drag?.active === true && drag.id === tab.id}
      <div
        style:transform={drag?.active ? `translateX(${shiftFor(index)}px)` : undefined}
        style:transition={dragging && !drag?.settling ? 'none' : undefined}
        class={tabClass}
        aria-selected={active}
        data-active={active}
        data-closing={closing.includes(tab.id)}
        data-dragging={dragging}
        data-entering={entering === tab.id}
        data-id={tab.id}
        onauxclick={(event) => {
          if (event.button === 1) startClose(tab.id);
        }}
        onkeydown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            window.shell.activateTab?.(tab.id);
          }
        }}
        onpointercancel={finishDrag}
        onpointerdown={(event) => onPointerDown(event, tab, index)}
        onpointermove={onPointerMove}
        onpointerup={finishDrag}
        role="tab"
        tabindex={active ? 0 : -1}
      >
        <span
          class={css({ flex: '1', minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}
          title={tab.title || undefined}
        >
          {tab.title || (tab.loading ? '불러오는 중…' : '타이피')}
        </span>
        <button
          class={['close', closeClass]}
          aria-label="탭 닫기"
          onclick={(event) => {
            event.stopPropagation();
            startClose(tab.id);
          }}
          onpointerdown={(event) => event.stopPropagation()}
          tabindex="-1"
          type="button"
        >
          <XIcon class={css({ size: '12px' })} />
        </button>
      </div>
    {/each}
  </div>

  <button
    style:-webkit-app-region="no-drag"
    style:visibility={drag?.active ? 'hidden' : undefined}
    class={newTabClass}
    aria-label="새 탭"
    onclick={() => {
      pendingPointerAdd = true;
      window.shell.newTab?.();
    }}
    type="button"
  >
    <PlusIcon class={css({ size: '16px' })} />
  </button>

  <div class={css({ flex: '1' })}></div>

  {@render children?.()}

  {#if !isMac}
    <button
      style:-webkit-app-region="no-drag"
      class={iconButtonClass}
      aria-label="메뉴"
      onclick={() => window.shell.popupMenu?.()}
      type="button"
    >
      <EllipsisIcon class={css({ size: '16px' })} />
    </button>
  {/if}
</div>
