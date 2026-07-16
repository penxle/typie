<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { getContext, setContext, tick } from 'svelte';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { portal } from '../actions';
  import { createHoverFocusHandler } from '../utils';
  import Icon from './Icon.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Component, Snippet } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    listStyle?: SystemStyleObject;
    icon?: Component;
    prefix?: Snippet;
    label: string;
    children?: Snippet;
  };

  let { style, listStyle, icon, prefix, label, children }: Props = $props();

  const parentClose = getContext<undefined | (() => void)>('close');
  setContext('close', () => {
    submenuOpen = false;
    parentClose?.();
  });

  const focusManaged = getContext<boolean>('menuFocusManaged') ?? false;

  const registerTrapContainer = getContext<((el: HTMLElement) => () => void) | undefined>('registerMenuTrapContainer');

  const MENU_ITEM_SELECTOR = '[role="menuitem"]:not(:disabled), [role="menuitemradio"]:not(:disabled)';

  let submenuOpen = $state(false);
  let flipped = false;
  let triggerEl = $state<HTMLDivElement>();
  let submenuEl = $state<HTMLUListElement>();
  let lastPointerPos = { x: 0, y: 0 };
  let focused = $state(false);
  let safeZoneTimeout: ReturnType<typeof setTimeout> | undefined;

  const onMenuClose = getContext<((cb: () => void) => () => void) | undefined>('onMenuClose');
  $effect(() => {
    if (submenuOpen && submenuEl && onMenuClose) {
      const el = submenuEl;
      return onMenuClose(() => {
        el.style.visibility = 'hidden';
      });
    }
  });

  $effect(() => {
    if (submenuOpen && submenuEl && registerTrapContainer) {
      return registerTrapContainer(submenuEl);
    }
  });

  const isPointInTriangle = (px: number, py: number, ax: number, ay: number, bx: number, by: number, cx: number, cy: number) => {
    const d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by);
    const d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy);
    const d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay);
    return !((d1 < 0 || d2 < 0 || d3 < 0) && (d1 > 0 || d2 > 0 || d3 > 0));
  };

  // 서브메뉴 위치 + 포인터 추적
  $effect(() => {
    if (!submenuOpen || !submenuEl || !triggerEl) return;

    const tr = triggerEl.getBoundingClientRect();
    const submenuWidth = submenuEl.offsetWidth;
    flipped = tr.right + 4 + submenuWidth > window.innerWidth;

    submenuEl.style.position = 'fixed';
    submenuEl.style.top = `${tr.top}px`;
    submenuEl.style.left = flipped ? `${tr.left - 4 - submenuWidth}px` : `${tr.right + 4}px`;

    const menuContainer = triggerEl.parentElement;
    if (!menuContainer) return;

    // 다른 메뉴 아이템 호버 시 닫기 (삼각형 영역 내는 제외, 일정 시간 머무르면 닫기)
    const findDirectChild = (target: HTMLElement) => {
      let el: HTMLElement | null = target;
      while (el && el !== menuContainer && el.parentElement !== menuContainer) {
        el = el.parentElement;
      }
      return el && el !== menuContainer ? el : null;
    };

    // submenu에 진입한 적이 있으면 triangle zone 비활성화
    let hasEnteredSubmenu = false;

    const handlePointerOver = (e: PointerEvent) => {
      if (!submenuEl || !triggerEl) return;

      if (!hasEnteredSubmenu) {
        const sr = submenuEl.getBoundingClientRect();
        const safeEdgeX = flipped ? sr.right : sr.left;
        if (
          isPointInTriangle(e.clientX, e.clientY, lastPointerPos.x, lastPointerPos.y, safeEdgeX, sr.top - 10, safeEdgeX, sr.bottom + 10)
        ) {
          clearTimeout(safeZoneTimeout);
          // trigger 위에 있으면 타이머 불필요
          const directChild = findDirectChild(e.target as HTMLElement);
          if (directChild !== triggerEl) {
            // 포인터~서브메뉴 거리에 비례하여 timeout 조절 (가까울수록 짧게)
            const distance = flipped ? e.clientX - sr.right : sr.left - e.clientX;
            const mcr = menuContainer.getBoundingClientRect();
            const maxDistance = flipped ? mcr.right - sr.right : sr.left - mcr.left;
            const ratio = Math.max(0, Math.min(1, distance / Math.max(1, maxDistance)));
            const timeout = 150 + ratio * 250;

            const parkedTarget = e.target as HTMLElement;
            safeZoneTimeout = setTimeout(() => {
              submenuOpen = false;
              // No pointermove follows a stationary close, so hand focus to the item parked under the pointer.
              const item = parkedTarget.closest(MENU_ITEM_SELECTOR);
              if (item instanceof HTMLElement && item.matches(':hover')) {
                item.focus({ preventScroll: true });
              }
            }, timeout);
          }
          return;
        }
      }

      clearTimeout(safeZoneTimeout);
      const directChild = findDirectChild(e.target as HTMLElement);
      if (directChild && directChild !== triggerEl) {
        submenuOpen = false;
        const item = (e.target as HTMLElement).closest(MENU_ITEM_SELECTOR);
        if (item instanceof HTMLElement) {
          item.focus({ preventScroll: true });
        }
      }
    };

    const exitSafezone = () => {
      hasEnteredSubmenu = true;
      delete menuContainer.dataset.submenuSafezone;
    };

    const handleSubmenuEnter = () => {
      clearTimeout(safeZoneTimeout);
      exitSafezone();
    };

    // safezone 상태에서 main menu 클릭 시 focus 방지 + submenu만 닫고 클릭 차단
    const isSafezoneTarget = (e: Event) => {
      if (hasEnteredSubmenu) return false;
      const directChild = findDirectChild(e.target as HTMLElement);
      return directChild !== null && directChild !== triggerEl;
    };

    const handlePointerDown = (e: PointerEvent) => {
      if (isSafezoneTarget(e)) {
        e.preventDefault(); // focus 방지
      }
    };

    const handleClick = (e: MouseEvent) => {
      if (!isSafezoneTarget(e)) {
        return;
      }

      e.stopPropagation();
      e.preventDefault();
      submenuOpen = false;
      const item = (e.target as HTMLElement).closest(MENU_ITEM_SELECTOR);
      if (item instanceof HTMLElement) {
        item.focus({ preventScroll: true });
      }
    };

    menuContainer.dataset.submenuSafezone = '';
    menuContainer.addEventListener('pointerover', handlePointerOver);
    menuContainer.addEventListener('pointerdown', handlePointerDown, { capture: true });
    menuContainer.addEventListener('click', handleClick, { capture: true });
    submenuEl.addEventListener('pointerenter', handleSubmenuEnter);
    return () => {
      clearTimeout(safeZoneTimeout);
      delete menuContainer.dataset.submenuSafezone;
      menuContainer.removeEventListener('pointerover', handlePointerOver);
      menuContainer.removeEventListener('pointerdown', handlePointerDown, true);
      menuContainer.removeEventListener('click', handleClick, true);
      submenuEl?.removeEventListener('pointerenter', handleSubmenuEnter);
    };
  });

  const getSubmenuItems = () => {
    return submenuEl?.querySelectorAll(MENU_ITEM_SELECTOR);
  };

  const hoverFocus = createHoverFocusHandler(MENU_ITEM_SELECTOR);
</script>

<!-- 트리거 -->
<div
  bind:this={triggerEl}
  class={cx(
    'group',
    css(
      {
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        borderRadius: '6px',
        marginX: '2px',
        paddingX: '8px',
        paddingY: '4px',
        fontSize: '13px',
        fontWeight: 'medium',
        textAlign: 'left',
        color: 'text.subtle',
        transition: 'common',
        cursor: 'pointer',
        _focus: { backgroundColor: 'surface.muted' },
      },
      !focusManaged && { _hover: { backgroundColor: 'surface.muted' } },
      submenuOpen && { backgroundColor: 'surface.muted' },
      style,
    ),
  )}
  aria-expanded={submenuOpen}
  aria-haspopup="menu"
  onblur={(e) => {
    focused = false;
    // Keyboard nav in the parent menu moves focus off the trigger; close the submenu unless focus went into it.
    if (submenuOpen && !(e.relatedTarget instanceof Node && submenuEl?.contains(e.relatedTarget))) {
      submenuOpen = false;
    }
  }}
  onfocus={() => (focused = true)}
  onkeydown={(e) => {
    if (e.key === 'ArrowRight' || e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      e.stopPropagation();
      submenuOpen = true;
      tick().then(() => {
        const items = getSubmenuItems();
        if (items && items.length > 0) {
          (items[0] as HTMLElement).focus();
        }
      });
    } else if (e.key === 'ArrowLeft' && submenuOpen) {
      e.preventDefault();
      e.stopPropagation();
      submenuOpen = false;
    }
  }}
  onpointerenter={() => {
    submenuOpen = true;
  }}
  onpointermove={(e) => {
    // pointerenter alone cannot reopen after a keyboard close while the pointer never left the trigger.
    if (e.clientX !== lastPointerPos.x || e.clientY !== lastPointerPos.y) {
      submenuOpen = true;
    }
    lastPointerPos = { x: e.clientX, y: e.clientY };
  }}
  role="menuitem"
  tabindex={focused ? 0 : -1}
>
  {#if prefix}
    {@render prefix()}
  {:else if icon}
    <Icon
      style={css.raw({
        color: 'text.faint',
        _groupFocus: { color: 'text.subtle' },
        _groupHover: focusManaged ? undefined : { color: 'text.subtle' },
      })}
      {icon}
      size={14}
    />
  {/if}
  <span>{label}</span>
  <Icon style={css.raw({ marginLeft: 'auto', flexShrink: '0', color: 'text.faint' })} icon={ChevronRightIcon} size={12} />
</div>

<!-- 서브메뉴 (포탈) -->
{#if submenuOpen}
  <ul
    bind:this={submenuEl}
    class={css(
      {
        display: 'flex',
        flexDirection: 'column',
        gap: '2px',
        borderRadius: '8px',
        paddingY: '2px',
        width: '[max-content]',
        minWidth: '160px',
        backgroundColor: 'surface.default',
        boxShadow: '[0 4px 16px rgba(0, 0, 0, 0.12), 0 1px 4px rgba(0, 0, 0, 0.08)]',
        _dark: {
          boxShadow: '[0 4px 16px rgba(0, 0, 0, 0.4), 0 1px 4px rgba(0, 0, 0, 0.25)]',
        },
        zIndex: 'tooltip',
        pointerEvents: 'auto',
        transformOrigin: 'left top',
        animation: 'fadeIn',
        animationDuration: '150ms',
        animationTimingFunction: 'ease-out',
      },
      listStyle,
    )}
    onkeydown={(e) => {
      const target = e.target as HTMLElement;

      if (e.key === 'ArrowLeft' || e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        submenuOpen = false;
        triggerEl?.focus();
        return;
      }

      const items = getSubmenuItems();
      if (!items || items.length === 0) return;

      // eslint-disable-next-line unicorn/prefer-spread
      const pos = Array.from(items).indexOf(target);

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        e.stopPropagation();
        const next = (items[pos + 1] || items[0]) as HTMLElement;
        next?.focus();
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        e.stopPropagation();
        // eslint-disable-next-line unicorn/prefer-at
        const prev = (items[pos - 1] || items[items.length - 1]) as HTMLElement;
        prev?.focus();
      }
    }}
    onpointermove={hoverFocus}
    role="menu"
    use:portal
  >
    {@render children?.()}
  </ul>
{/if}
