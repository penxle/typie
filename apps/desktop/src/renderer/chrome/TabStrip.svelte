<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { entityIconMap, getEntityIconColor } from '@typie/ui/constants';
  import { untrack } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileXIcon from '~icons/lucide/file-x';
  import PlusIcon from '~icons/lucide/plus';
  import XIcon from '~icons/lucide/x';
  import type { Snippet } from 'svelte';

  type Props = { tabs: TabState[]; activeId: string | null; fullscreen?: boolean; children?: Snippet };
  let { tabs, activeId, fullscreen = false, children }: Props = $props();

  const isMac = window.shell.platform === 'darwin';
  const extraIcons = new Map([['file-x', FileXIcon]]);
  const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
  const CLOSE_MS = 160;
  const SETTLE_MIN_MS = 80;
  const SETTLE_MAX_MS = 200;
  const COMMIT_TIMEOUT_MS = 400;
  const DRAG_THRESHOLD = 8;
  const SWAP_HYSTERESIS = 0.1;
  const EASE_OUT = 'cubic-bezier(0.23, 1, 0.32, 1)';
  const EASE_IN_OUT = 'cubic-bezier(0.77, 0, 0.175, 1)';

  type Drag = {
    id: string;
    index: number;
    startX: number;
    dx: number;
    step: number;
    active: boolean;
    target: number;
    settling: boolean;
    settleMs: number;
    committed: boolean;
  };

  let scroller = $state<HTMLDivElement | null>(null);
  let overflowLeft = $state(false);
  let overflowRight = $state(false);
  let closing = $state<string[]>([]);
  let entering = $state<string | null>(null);
  const animating = new SvelteMap<string, number>();
  let drag = $state<Drag | null>(null);
  let holdTransition = $state(false);
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
    const added = tabs.filter((tab) => !knownIds.has(tab.id));
    if (knownIds.size > 0 && added.length === 1 && !reduceMotion.matches) {
      const id = added[0].id;
      entering = id;
      const sibling = tabs.find((tab) => tab.id !== id);
      untrack(() => holdWidth(id, sibling ? tabWidth(sibling.id) : undefined));
      requestAnimationFrame(() => requestAnimationFrame(() => (entering = null)));
      setTimeout(() => releaseWidth(id), 240);
    }
    knownIds = ids;
    const pending = untrack(() => drag);
    if (pending?.committed && tabs.findIndex((tab) => tab.id === pending.id) === pending.target) releaseDrag();
    const stale = untrack(() => closing).filter((id) => !ids.has(id));
    if (stale.length > 0) closing = untrack(() => closing).filter((id) => ids.has(id));
    for (const id of untrack(() => [...animating.keys()])) {
      if (!ids.has(id)) releaseWidth(id);
    }
    requestAnimationFrame(updateOverflow);
  });

  $effect(() => {
    if (!activeId || !scroller) return;
    const element = scroller.querySelector<HTMLElement>(`[data-id="${activeId}"]`);
    element?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  });

  const closable = $derived(tabs.length - closing.length > 1);

  const tabWidth = (id: string) => scroller?.querySelector<HTMLElement>(`[data-id="${id}"]`)?.getBoundingClientRect().width;

  const holdWidth = (id: string, width: number | undefined) => {
    if (width !== undefined) animating.set(id, width);
  };

  const releaseWidth = (id: string) => {
    animating.delete(id);
  };

  const releaseDrag = () => {
    drag = null;
    holdTransition = true;
    requestAnimationFrame(() => requestAnimationFrame(() => (holdTransition = false)));
  };

  const startClose = (id: string) => {
    if (!closable || closing.includes(id)) return;
    if (reduceMotion.matches) {
      window.shell.closeTab?.(id);
      return;
    }
    holdWidth(id, tabWidth(id));
    if (id === activeId) {
      const index = tabs.findIndex((tab) => tab.id === id);
      const neighbor = [...tabs.slice(index + 1), ...tabs.slice(0, index).toReversed()].find((tab) => !closing.includes(tab.id));
      if (neighbor) window.shell.activateTab?.(neighbor.id);
    }
    closing = [...closing, id];
    setTimeout(() => window.shell.closeTab?.(id), CLOSE_MS);
  };

  $effect(() => {
    const off = window.shell.onCloseTabRequest?.(() => {
      const id = untrack(() => activeId);
      if (id) startClose(id);
    });
    return () => off?.();
  });

  const onPointerDown = (event: PointerEvent, tab: TabState, index: number) => {
    if (event.button !== 0 || (event.target as HTMLElement).closest('.close')) return;
    if (tab.id !== activeId) window.shell.activateTab?.(tab.id);
    const element = event.currentTarget as HTMLElement;
    element.setPointerCapture(event.pointerId);
    const gap = scroller ? Number.parseFloat(getComputedStyle(scroller).columnGap) || 0 : 0;
    drag = {
      id: tab.id,
      index,
      startX: event.clientX,
      dx: 0,
      step: element.getBoundingClientRect().width + gap,
      active: false,
      target: index,
      settling: false,
      settleMs: SETTLE_MAX_MS,
      committed: false,
    };
  };

  const onPointerMove = (event: PointerEvent) => {
    if (!drag || drag.settling) return;
    const raw = event.clientX - drag.startX;
    if (!drag.active && Math.abs(raw) < DRAG_THRESHOLD) return;
    const min = -drag.index * drag.step;
    const max = (tabs.length - 1 - drag.index) * drag.step;
    drag.active = true;
    drag.dx = Math.max(min, Math.min(max, raw));
    const slots = drag.dx / drag.step;
    let target = drag.target;
    while (target < tabs.length - 1 && slots > target - drag.index + 0.5 + SWAP_HYSTERESIS) target += 1;
    while (target > 0 && slots < target - drag.index - 0.5 - SWAP_HYSTERESIS) target -= 1;
    drag.target = target;
  };

  const finishDrag = () => {
    if (!drag) return;
    if (!drag.active) {
      drag = null;
      return;
    }
    const { id, index, target, step } = drag;
    const commit = () => {
      if (drag?.id !== id) return;
      if (target === index) {
        drag = null;
        return;
      }
      window.shell.moveTab?.(id, target);
      drag.committed = true;
      setTimeout(() => {
        if (drag?.id === id && drag.committed) releaseDrag();
      }, COMMIT_TIMEOUT_MS);
    };
    if (reduceMotion.matches) {
      commit();
      return;
    }
    const remaining = Math.abs((target - index) * step - drag.dx);
    const settleMs = Math.round(Math.max(SETTLE_MIN_MS, Math.min(SETTLE_MAX_MS, (remaining / step) * SETTLE_MAX_MS)));
    drag.settling = true;
    drag.settleMs = settleMs;
    drag.dx = (target - index) * step;
    setTimeout(commit, settleMs);
  };

  const transitionFor = (id: string) => {
    if (holdTransition || drag?.committed) return 'none';
    if (!drag?.active) return;
    if (drag.id !== id) return `transform 200ms ${EASE_IN_OUT}`;
    return drag.settling ? `transform ${drag.settleMs}ms ${EASE_OUT}` : 'none';
  };

  const shiftFor = (index: number) => {
    if (!drag?.active) return 0;
    if (index === drag.index) return drag.dx;
    if (drag.index < drag.target && index > drag.index && index <= drag.target) return -drag.step;
    if (drag.index > drag.target && index < drag.index && index >= drag.target) return drag.step;
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
    flexShrink: '1',
    width: '[200px]',
    minWidth: '[80px]',
    height: '28px',
    borderRadius: '8px',
    fontSize: '13px',
    fontWeight: 'medium',
    color: 'text.subtle',
    cursor: 'default',
    outline: 'none',
    transition:
      '[width 160ms cubic-bezier(0.23, 1, 0.32, 1), min-width 160ms cubic-bezier(0.23, 1, 0.32, 1), transform 160ms cubic-bezier(0.23, 1, 0.32, 1), opacity 160ms cubic-bezier(0.23, 1, 0.32, 1), color 120ms ease, background-color 120ms ease, box-shadow 120ms ease]',
    '&:hover': { color: 'text.default', backgroundColor: 'surface.muted' },
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
      opacity: '0',
      overflow: 'hidden',
    },
    '&[data-animating="true"]': { overflow: 'hidden' },
    '&[data-closing="true"]': {
      transition:
        '[width 160ms cubic-bezier(0.23, 1, 0.32, 1), min-width 160ms cubic-bezier(0.23, 1, 0.32, 1), opacity 100ms cubic-bezier(0.23, 1, 0.32, 1)]',
    },
    '&[data-entering="true"]': { transition: '[none]' },
    '&[data-dragging="true"]': { zIndex: '2', backgroundColor: 'surface.default', boxShadow: 'small' },
    _focusVisible: { boxShadow: '[inset 0 0 0 2px {colors.accent.brand.default}]' },
    _motionReduce: { transitionDuration: '[0ms]' },
  });

  const contentClass = css({
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    flex: '1',
    minWidth: '0',
    height: 'full',
    paddingLeft: '12px',
    paddingRight: '6px',
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
    transitionTimingFunction: '[cubic-bezier(0.23, 1, 0.32, 1)]',
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
    '&[data-fullscreen="true"]': { paddingLeft: '[8px]', paddingRight: '[8px]' },
  })}
  data-fullscreen={fullscreen}
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
        style:transition={transitionFor(tab.id)}
        class={tabClass}
        aria-selected={active}
        data-active={active}
        data-animating={animating.has(tab.id)}
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
        ontransitionend={(event) => {
          if (event.target === event.currentTarget && event.propertyName === 'width') releaseWidth(tab.id);
        }}
        role="tab"
        tabindex={active ? 0 : -1}
      >
        <div
          style:flex={animating.has(tab.id) ? 'none' : undefined}
          style:width={animating.has(tab.id) ? `${animating.get(tab.id)}px` : undefined}
          class={contentClass}
        >
          {#if tab.icon}
            {@const TabIcon = entityIconMap.get(tab.icon.icon) ?? extraIcons.get(tab.icon.icon)}
            {#if TabIcon}
              <span
                style:color={tab.icon.color ? getEntityIconColor(tab.icon.color) : undefined}
                class={css({ display: 'flex', flexShrink: '0', color: 'text.faint' })}
              >
                <TabIcon class={css({ size: '14px' })} />
              </span>
            {/if}
          {/if}
          <span
            class={css({ flex: '1', minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}
            title={tab.title || undefined}
          >
            {tab.title || '불러오는 중…'}
          </span>
          {#if closable}
            <button
              style:opacity={animating.has(tab.id) ? '0' : undefined}
              style:pointer-events={animating.has(tab.id) ? 'none' : undefined}
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
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <button style:-webkit-app-region="no-drag" class={newTabClass} aria-label="새 탭" onclick={() => window.shell.newTab?.()} type="button">
    <PlusIcon class={css({ size: '16px' })} />
  </button>

  <div class={css({ flex: '1' })}></div>

  <div class={css({ display: 'flex', alignItems: 'center', gap: '4px' })}>
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
</div>
