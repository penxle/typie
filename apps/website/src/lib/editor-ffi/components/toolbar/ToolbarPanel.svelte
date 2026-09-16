<script lang="ts" module>
  import type { Component } from 'svelte';

  export type ToolbarPanelItem = {
    id: string;
    label: string;
    keywords?: string[];
    icon?: Component;
    selected?: boolean;
    swatch?: { color: string | null; shape: 'circle' | 'square' };
    chipLabel?: string;
    drill?: boolean;
  };

  export type ToolbarPanelAction = { id: string; label: string; icon: Component };

  let pointerX = -1;
  let pointerY = -1;
</script>

<script lang="ts">
  import { flip, shift } from '@floating-ui/dom';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions, scrollFog, tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { canBeChoseong, disassemble, getChoseong } from 'es-hangul';
  import { onMount, tick, untrack } from 'svelte';
  import CheckIcon from '~icons/lucide/check';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CornerDownLeftIcon from '~icons/lucide/corner-down-left';
  import SearchIcon from '~icons/lucide/search';
  import SlashIcon from '~icons/lucide/slash';
  import ToolbarPanelSurface from './ToolbarPanelSurface.svelte';
  import type { Snippet } from 'svelte';

  type Props = {
    items: ToolbarPanelItem[];
    recentIds?: string[];
    placeholder?: string;
    bare?: boolean;
    rowHeight?: number;
    rowPaddingX?: string;
    rowPaddingY?: string;
    rowJustify?: 'flex-start' | 'center';
    free?: { accepts: (query: string) => boolean; apply: (query: string) => void; display: (query: string) => string };
    render?: Snippet<[ToolbarPanelItem]>;
    submenu?: Snippet<[ToolbarPanelItem]>;
    actions?: ToolbarPanelAction[];
    onselect: (id: string) => void;
    onActionSelect?: (id: string) => void;
  };

  let {
    items,
    recentIds = [],
    placeholder = '',
    bare = false,
    rowHeight = 28,
    rowPaddingX = '8px',
    rowPaddingY = '0',
    rowJustify = 'flex-start',
    free,
    render,
    submenu,
    actions = [],
    onselect,
    onActionSelect,
  }: Props = $props();

  type Zone = 'free' | 'strip' | 'list' | 'action';
  type Entry = { key: string; zone: Zone; item?: ToolbarPanelItem; action?: ToolbarPanelAction };

  const RECENT_LABEL = '최근 사용한 항목';
  const LIST_MAX = 280;
  const LIST_PAD = 4;
  const ROW_GAP = 1;
  const FOG_SIZE = 16;
  const SAFE_ZONE_PADDING = 10;
  const AIM_MIN_DISTANCE = 2;
  const LINGER_MIN_DELAY = 150;
  const LINGER_DELAY_RANGE = 250;
  const listId = `toolbar-panel-${Math.random().toString(36).slice(2, 8)}`;

  const listMaxHeight = $derived.by(() => {
    const stride = rowHeight + ROW_GAP;
    const peek = Math.round((rowHeight * 2) / 3);
    const rows = Math.max(1, Math.floor((LIST_MAX - LIST_PAD - peek) / stride));
    return LIST_PAD + rows * stride + peek;
  });

  let query = $state('');
  let active = $state(-1);
  let nav = $state<'pointer' | 'keyboard'>('pointer');
  let input = $state<HTMLInputElement>();
  let root = $state<HTMLElement>();
  let list = $state<HTMLElement>();
  let submenuEl = $state<HTMLElement>();
  let openId = $state<string | null>(null);
  let closeTimer: ReturnType<typeof setTimeout> | null = null;
  let triangleOrigin = { x: 0, y: 0 };
  let enteredSubmenu = false;
  let inSafeZone = false;

  const normalized = $derived(query.trim().toLowerCase());
  const queryJamo = $derived(disassemble(normalized));
  const queryIsChoseong = $derived(normalized.length > 0 && [...normalized].every((char) => canBeChoseong(char)));

  const matches = (item: ToolbarPanelItem) =>
    [item.label, ...(item.keywords ?? [])].some((text) => {
      const lower = text.toLowerCase();
      if (disassemble(lower).includes(queryJamo)) return true;
      return queryIsChoseong && getChoseong(lower).includes(normalized);
    });

  const recentItems = $derived(
    normalized.length === 0 ? recentIds.map((id) => items.find((item) => item.id === id)).filter((item) => item !== undefined) : [],
  );
  const filtered = $derived(normalized.length === 0 ? items : items.filter((item) => matches(item)));
  const freeRow = $derived(
    free !== undefined &&
      normalized.length > 0 &&
      free.accepts(query.trim()) &&
      filtered.every((item) => item.label.toLowerCase() !== normalized && !item.keywords?.includes(query.trim())),
  );

  const entries = $derived.by<Entry[]>(() => {
    const out: Entry[] = [];
    if (freeRow) out.push({ key: 'free', zone: 'free' });
    for (const item of recentItems) out.push({ key: `strip:${item.id}`, zone: 'strip', item });
    for (const item of filtered) out.push({ key: item.id, zone: 'list', item });
    if (normalized.length === 0) for (const action of actions) out.push({ key: `action:${action.id}`, zone: 'action', action });
    return out;
  });

  const stripStart = $derived(entries.findIndex((entry) => entry.zone === 'strip'));
  const stripEnd = $derived(stripStart === -1 ? -1 : stripStart + recentItems.length - 1);
  const firstAction = $derived(entries.findIndex((entry) => entry.zone === 'action'));

  $effect(() => {
    const empty = normalized.length === 0;
    untrack(() => {
      const selected = entries.findIndex((entry) => entry.zone === 'list' && entry.item?.selected);
      const firstList = entries.findIndex((entry) => entry.zone === 'list');
      if (empty && selected !== -1) active = selected;
      else if (entries[0]?.zone === 'free') active = 0;
      else active = firstList;
    });
  });

  const reveal = async () => {
    await tick();
    list?.querySelector<HTMLElement>(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' });
  };

  const move = (direction: 1 | -1) => {
    const count = entries.length;
    if (count === 0) return;
    if (active === -1) {
      active = direction === 1 ? 0 : count - 1;
    } else if (entries[active]?.zone === 'strip') {
      active = direction === 1 ? (stripEnd + 1 < count ? stripEnd + 1 : 0) : (stripStart - 1 + count) % count;
    } else {
      const next = (active + direction + count) % count;
      active = entries[next]?.zone === 'strip' ? stripStart : next;
    }
    void reveal();
  };

  const commit = (index: number) => {
    const entry = entries[index];
    if (!entry) return;
    if (enterSubmenu(index)) return;
    if (entry.zone === 'free') free?.apply(query.trim());
    else if (entry.action) onActionSelect?.(entry.action.id);
    else if (entry.item) onselect(entry.item.id);
  };

  const onKeydown = (e: KeyboardEvent) => {
    if (e.isComposing) return;
    const inStrip = entries[active]?.zone === 'strip';
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      nav = 'keyboard';
      move(e.key === 'ArrowDown' ? 1 : -1);
    } else if (inStrip && (e.key === 'ArrowRight' || e.key === 'ArrowLeft')) {
      e.preventDefault();
      nav = 'keyboard';
      active = Math.min(stripEnd, Math.max(stripStart, active + (e.key === 'ArrowRight' ? 1 : -1)));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      commit(active);
    } else if (e.key === 'ArrowRight' && enterSubmenu(active)) {
      e.preventDefault();
    }
  };

  const trackPointer = (event: PointerEvent) => {
    if (event.clientX === pointerX && event.clientY === pointerY) return false;
    pointerX = event.clientX;
    pointerY = event.clientY;
    nav = 'pointer';
    return true;
  };

  const entryAt = (target: EventTarget | null) => {
    const element = target instanceof Element ? target.closest<HTMLElement>('[data-entry]') : null;
    const index = element ? Number(element.dataset.entry) : -1;
    return Number.isSafeInteger(index) && index >= 0 ? { index, element, entry: entries[index] } : null;
  };

  const onPointerMove = (event: PointerEvent) => {
    const from = { x: pointerX, y: pointerY };
    const moved = trackPointer(event);
    if (!moved && nav === 'keyboard') return;

    const hit = entryAt(event.target);

    if (openId !== null) {
      const neutral = hit === null || hit.entry?.zone === 'strip' || hit.entry?.item?.id === openId;
      if (neutral) {
        triangleOrigin = { x: event.clientX, y: event.clientY };
        clearCloseTimer();
        inSafeZone = false;
        return;
      }
      if (insideSafeZone(event) || headingToSubmenu(from, event)) {
        inSafeZone = true;
        armLingerClose(event);
        return;
      }
      inSafeZone = false;
      closeSubmenu();
    }

    if (!hit) return;
    active = hit.index;
    openSubmenu(hit.index, hit.element ?? undefined);
  };

  const chipSelect = async (index: number) => {
    const item = entries[index]?.item;
    const target = submenu && item?.drill ? entries.findIndex((entry) => entry.zone === 'list' && entry.item?.id === item.id) : -1;
    if (target === -1) {
      commit(index);
      return;
    }
    active = target;
    await reveal();
    enterSubmenu(target);
  };

  const clickRow = (index: number) => {
    if (inSafeZone) {
      closeSubmenu();
      active = index;
      return;
    }
    commit(index);
  };

  const adoptPointerRow = () => {
    if (pointerX < 0 || pointerY < 0) return;
    const row = document.elementFromPoint(pointerX, pointerY)?.closest<HTMLElement>('[data-index]');
    if (!row || !list?.contains(row)) return;
    const index = Number(row.dataset.index);
    if (Number.isSafeInteger(index)) active = index;
  };

  const { floating: floatingSubmenu, setReference: setSubmenuReference } = createFloatingActions({
    placement: 'right-start',
    offset: 4,
    middleware: [flip(), shift({ padding: 8 })],
  });

  const submenuItem = $derived(entries.find((entry) => entry.item?.id === openId)?.item);

  const clearCloseTimer = () => {
    if (!closeTimer) return;
    clearTimeout(closeTimer);
    closeTimer = null;
  };

  const closeSubmenu = () => {
    clearCloseTimer();
    inSafeZone = false;
    if (openId === null) return;
    openId = null;
    setSubmenuReference(null);
  };

  const openSubmenu = (index: number, row?: HTMLElement) => {
    const entry = entries[index];
    const item = entry?.item;
    if (!submenu || entry?.zone !== 'list' || !item?.drill) return false;
    clearCloseTimer();
    inSafeZone = false;
    enteredSubmenu = false;
    triangleOrigin = { x: pointerX, y: pointerY };
    setSubmenuReference(row ?? list?.querySelector<HTMLElement>(`[data-index="${index}"]`) ?? null);
    openId = item.id;
    return true;
  };

  const enterSubmenu = (index: number) => {
    if (!openSubmenu(index)) return false;
    tick().then(() => submenuEl?.querySelector<HTMLElement>('[data-toolbar-panel]')?.focus({ preventScroll: true }));
    return true;
  };

  const isPointInTriangle = (px: number, py: number, ax: number, ay: number, bx: number, by: number, cx: number, cy: number) => {
    const d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by);
    const d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy);
    const d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay);
    return !((d1 < 0 || d2 < 0 || d3 < 0) && (d1 > 0 || d2 > 0 || d3 > 0));
  };

  const safeZoneGeometry = () => {
    if (enteredSubmenu || !submenuEl || !root) return null;
    const submenuRect = submenuEl.getBoundingClientRect();
    const panelRect = root.getBoundingClientRect();
    const flipped = submenuRect.left < panelRect.left;
    return { submenuRect, panelRect, flipped, edgeX: flipped ? submenuRect.right : submenuRect.left };
  };

  const rayHitsRect = (x: number, y: number, dx: number, dy: number, rect: DOMRect, padding: number) => {
    const left = rect.left - padding;
    const right = rect.right + padding;
    const top = rect.top - padding;
    const bottom = rect.bottom + padding;
    let near = 0;
    let far = Infinity;
    if (dx === 0) {
      if (x < left || x > right) return false;
    } else {
      const first = (left - x) / dx;
      const second = (right - x) / dx;
      near = Math.max(near, Math.min(first, second));
      far = Math.min(far, Math.max(first, second));
    }
    if (dy === 0) {
      if (y < top || y > bottom) return false;
    } else {
      const first = (top - y) / dy;
      const second = (bottom - y) / dy;
      near = Math.max(near, Math.min(first, second));
      far = Math.min(far, Math.max(first, second));
    }
    return far >= near;
  };

  const headingToSubmenu = (from: { x: number; y: number }, event: PointerEvent) => {
    const geometry = safeZoneGeometry();
    if (!geometry) return false;
    const dx = event.clientX - from.x;
    const dy = event.clientY - from.y;
    if (Math.hypot(dx, dy) < AIM_MIN_DISTANCE) return inSafeZone;
    return rayHitsRect(event.clientX, event.clientY, dx, dy, geometry.submenuRect, SAFE_ZONE_PADDING);
  };

  const insideSafeZone = (event: PointerEvent) => {
    const geometry = safeZoneGeometry();
    if (!geometry) return false;
    return isPointInTriangle(
      event.clientX,
      event.clientY,
      triangleOrigin.x,
      triangleOrigin.y,
      geometry.edgeX,
      geometry.submenuRect.top - SAFE_ZONE_PADDING,
      geometry.edgeX,
      geometry.submenuRect.bottom + SAFE_ZONE_PADDING,
    );
  };

  const armLingerClose = (event: PointerEvent) => {
    const geometry = safeZoneGeometry();
    if (!geometry) return;
    const { submenuRect, panelRect, flipped } = geometry;
    const distance = flipped ? event.clientX - submenuRect.right : submenuRect.left - event.clientX;
    const maxDistance = flipped ? panelRect.right - submenuRect.right : submenuRect.left - panelRect.left;
    const ratio = Math.max(0, Math.min(1, distance / Math.max(1, maxDistance)));
    clearCloseTimer();
    closeTimer = setTimeout(
      () => {
        closeTimer = null;
        closeSubmenu();
        adoptPointerRow();
      },
      LINGER_MIN_DELAY + ratio * LINGER_DELAY_RANGE,
    );
  };

  $effect(() => {
    if (openId === null) return;
    return pushEscapeHandler(() => {
      closeSubmenu();
      root?.focus({ preventScroll: true });
      return true;
    });
  });

  $effect(() => {
    void normalized;
    untrack(closeSubmenu);
  });

  onMount(() => {
    adoptPointerRow();
    const timer = setTimeout(() => {
      input?.focus({ preventScroll: true });
      void reveal();
    }, 0);
    return () => clearTimeout(timer);
  });
</script>

{#snippet swatch(item: ToolbarPanelItem, size: string)}
  {#if item.swatch}
    <span
      style:width={size}
      style:height={size}
      style:background-color={item.swatch.color ?? 'transparent'}
      class={center({ flexShrink: '0', borderWidth: '1px', borderRadius: item.swatch.shape === 'circle' ? 'full' : '[3px]' })}
    >
      {#if item.swatch.color === null}
        <Icon style={css.raw({ color: 'text.hint' })} icon={SlashIcon} size={10} />
      {/if}
    </span>
  {/if}
{/snippet}

<div
  bind:this={root}
  class={flex({ flexDirection: 'column', width: '240px', outline: 'none' })}
  data-toolbar-panel
  onkeydown={onKeydown}
  onpointermove={onPointerMove}
  role="presentation"
  tabindex="-1"
>
  {#if !bare}
    <div
      class={flex({
        alignItems: 'center',
        gap: '8px',
        flexShrink: '0',
        height: '36px',
        paddingX: '12px',
        borderBottomWidth: '1px',
        borderColor: 'border.hairline',
      })}
    >
      <Icon style={css.raw({ flexShrink: '0', color: 'text.hint' })} icon={SearchIcon} size={14} />
      <input
        bind:this={input}
        class={css({
          flexGrow: '1',
          minWidth: '0',
          height: 'full',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.default',
          backgroundColor: 'transparent',
          border: 'none',
          outline: 'none',
          _placeholder: { color: 'text.hint', fontWeight: 'normal' },
        })}
        aria-activedescendant={active >= 0 && entries.length > 0 ? `${listId}-${active}` : undefined}
        aria-controls={listId}
        aria-expanded="true"
        aria-label={placeholder}
        {placeholder}
        role="combobox"
        type="text"
        bind:value={query}
      />
    </div>
  {/if}

  {#if stripStart !== -1}
    <div
      class={flex({
        flexDirection: 'column',
        gap: '2px',
        padding: '4px',
        paddingBottom: '8px',
        borderBottomWidth: '1px',
        borderColor: 'border.hairline',
      })}
      aria-label={RECENT_LABEL}
      role="group"
    >
      <div class={css({ paddingX: '8px', paddingY: '4px', fontSize: '12px', color: 'text.hint' })}>{RECENT_LABEL}</div>
      <div class={flex({ alignItems: 'center', flexWrap: 'wrap', gap: '4px', paddingX: '8px' })}>
        {#each entries as entry, index (entry.key)}
          {#if entry.zone === 'strip' && entry.item}
            {@const item = entry.item}
            {@const iconOnly = (item.swatch !== undefined || item.icon !== undefined) && item.chipLabel === undefined}
            <button
              id={`${listId}-${index}`}
              style:padding-inline={iconOnly ? '0' : '8px'}
              class={cx(
                'toolbar-panel-chip',
                css({
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '4px',
                  flexShrink: '0',
                  height: '24px',
                  minWidth: '24px',
                  borderRadius: '6px',
                  fontSize: '12px',
                  fontWeight: 'medium',
                  color: 'text.muted',
                  fontVariantNumeric: 'tabular-nums',
                  whiteSpace: 'nowrap',
                  backgroundColor: 'surface.inset',
                  '&[aria-selected="true"]': { color: 'text.default', backgroundColor: 'surface.active' },
                }),
              )}
              aria-label={item.label}
              aria-selected={active === index}
              data-entry={index}
              onclick={() => chipSelect(index)}
              onpointerdown={(e) => e.preventDefault()}
              role="option"
              tabindex="-1"
              type="button"
              use:tooltip={{ message: iconOnly ? item.label : null, arrow: false }}
            >
              {#if item.swatch}
                {@render swatch(item, '14px')}
              {:else if item.icon}
                <Icon style={css.raw({ '& *': { strokeWidth: '[1.75px]' } })} icon={item.icon} size={14} />
              {/if}
              {#if !iconOnly}
                {item.chipLabel ?? item.label}
              {/if}
            </button>
          {/if}
        {/each}
      </div>
    </div>
  {/if}

  <div
    style:max-height={`${listMaxHeight}px`}
    class={css({ overflowY: 'auto', scrollbar: 'hidden', overscrollBehavior: 'contain' })}
    use:scrollFog={{ orientation: 'vertical', size: FOG_SIZE }}
  >
    <div
      bind:this={list}
      id={listId}
      class={flex({
        flexDirection: 'column',
        gap: '1px',
        padding: '4px',
      })}
      role="listbox"
    >
      {#each entries as entry, index (entry.key)}
        {#if entry.zone !== 'strip'}
          {#if index === firstAction && firstAction > 0 && entries[firstAction - 1]?.zone !== 'strip'}
            <div class={css({ flexShrink: '0', marginX: '-4px', marginY: '4px', height: '1px', backgroundColor: 'border.hairline' })}></div>
          {/if}
          <button
            id={`${listId}-${index}`}
            style:height={`${rowHeight}px`}
            style:scroll-margin-block={`${FOG_SIZE}px`}
            style:padding-inline={rowPaddingX}
            style:padding-block={rowPaddingY}
            style:justify-content={rowJustify}
            class={css({
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              flexShrink: '0',
              width: 'full',
              borderRadius: '6px',
              fontSize: '13px',
              fontWeight: 'medium',
              color: 'text.muted',
              textAlign: 'left',
              fontVariantNumeric: 'tabular-nums',
              '&[aria-selected="true"]': { color: 'text.default', backgroundColor: 'surface.hover' },
            })}
            aria-selected={active === index}
            data-entry={index}
            data-index={index}
            onclick={() => clickRow(index)}
            onpointerdown={(e) => e.preventDefault()}
            role="option"
            tabindex="-1"
            type="button"
          >
            {#if entry.zone === 'free'}
              <span class={css({ minWidth: '0', truncate: true })}>{free?.display(query.trim())}</span>
              <Icon style={css.raw({ marginLeft: 'auto', flexShrink: '0', color: 'text.hint' })} icon={CornerDownLeftIcon} size={14} />
            {:else if entry.action}
              <Icon style={css.raw({ flexShrink: '0', '& *': { strokeWidth: '[1.75px]' } })} icon={entry.action.icon} size={14} />
              <span class={css({ minWidth: '0', truncate: true })}>{entry.action.label}</span>
            {:else if entry.item}
              {@const item = entry.item}
              {#if render}
                {@render render(item)}
              {:else}
                {#if item.swatch}
                  {@render swatch(item, '12px')}
                {:else if item.icon}
                  <Icon style={css.raw({ flexShrink: '0', '& *': { strokeWidth: '[1.75px]' } })} icon={item.icon} size={14} />
                {/if}
                <span class={css({ minWidth: '0', truncate: true })}>{item.label}</span>
              {/if}
              {#if item.drill}
                <Icon style={css.raw({ marginLeft: 'auto', flexShrink: '0', color: 'text.hint' })} icon={ChevronRightIcon} size={14} />
              {:else if item.selected}
                <Icon style={css.raw({ marginLeft: 'auto', flexShrink: '0', color: 'accent.default' })} icon={CheckIcon} size={14} />
              {/if}
            {/if}
          </button>
        {/if}
      {:else}
        {#if normalized.length > 0}
          <p class={css({ paddingX: '8px', paddingY: '10px', fontSize: '12px', color: 'text.hint' })}>일치하는 항목이 없어요.</p>
        {/if}
      {/each}
    </div>
  </div>
</div>

{#if submenuItem && submenu}
  <div
    class={css({ zIndex: 'overEditor' })}
    data-floating-keep-open
    onkeydown={(e) => {
      if (e.key !== 'ArrowLeft') return;
      e.preventDefault();
      closeSubmenu();
      root?.focus({ preventScroll: true });
    }}
    onpointerenter={() => {
      enteredSubmenu = true;
      inSafeZone = false;
      clearCloseTimer();
    }}
    role="presentation"
    use:floatingSubmenu
  >
    <div bind:this={submenuEl}>
      <ToolbarPanelSurface>
        {@render submenu(submenuItem)}
      </ToolbarPanelSurface>
    </div>
  </div>
{/if}

<style>
  :global(.toolbar-panel-chip) {
    transition:
      transform 100ms ease-out,
      background-color 100ms ease-out,
      color 100ms ease-out;
  }

  :global(.toolbar-panel-chip:active) {
    transform: scale(0.96);
  }

  @media (prefers-reduced-motion: reduce) {
    :global(.toolbar-panel-chip) {
      transition: none;
    }

    :global(.toolbar-panel-chip:active) {
      transform: none;
    }
  }
</style>
