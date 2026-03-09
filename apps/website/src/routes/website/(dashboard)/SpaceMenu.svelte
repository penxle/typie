<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import ChevronsUpDownIcon from '~icons/lucide/chevrons-up-down';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import PlusIcon from '~icons/lucide/plus';
  import SettingsIcon from '~icons/lucide/settings';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { pushState } from '$app/navigation';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { getPaneGroup } from './[slug]/@pane/context.svelte';
  import CreateSiteModal from './CreateSiteModal.svelte';
  import { PlanUpgradeDialog } from './plan-upgrade-dialog.svelte';
  import type { DashboardLayout_SpaceMenu_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_SpaceMenu_user$key;
    open?: boolean;
  };

  let { user$key, open = $bindable(false) }: Props = $props();

  const app = getAppContext();
  const paneGroup = getPaneGroup();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_SpaceMenu_user on User {
        id

        subscription {
          id
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
      }
    `),
    () => user$key,
  );

  const site = $derived(user.data.sites.find((s) => s.id === app.preference.current.currentSiteId) ?? user.data.sites[0]);

  let panelEl = $state<HTMLDivElement>();
  let createSiteModalOpen = $state(false);

  const close = () => {
    open = false;
  };

  $effect(() => {
    if (open) {
      const handleClickOutside = (e: MouseEvent) => {
        if (e.target === document.documentElement) return;
        if (submenuEl?.contains(e.target as Node)) return;
        if (panelEl && !panelEl.contains(e.target as Node)) {
          close();
        }
      };

      document.addEventListener('click', handleClickOutside, true);

      const cleanup = pushEscapeHandler(() => {
        close();
        return true;
      });

      return () => {
        document.removeEventListener('click', handleClickOutside, true);
        cleanup();
      };
    }
  });

  // 서브메뉴
  let submenuOpen = $state(false);
  let submenuTriggerEl = $state<HTMLElement>();
  let submenuEl = $state<HTMLElement>();
  let lastPointerPos = { x: 0, y: 0 };

  const portal = (node: HTMLElement) => {
    const container = document.querySelector('.tooltip-container') ?? document.body;
    container.append(node);
    return { destroy: () => node.remove() };
  };

  const isPointInTriangle = (px: number, py: number, ax: number, ay: number, bx: number, by: number, cx: number, cy: number) => {
    const d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by);
    const d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy);
    const d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay);
    return !((d1 < 0 || d2 < 0 || d3 < 0) && (d1 > 0 || d2 > 0 || d3 > 0));
  };

  // 서브메뉴 위치 + 포인터 추적
  $effect(() => {
    if (!submenuOpen || !submenuEl || !submenuTriggerEl) return;

    const tr = submenuTriggerEl.getBoundingClientRect();
    submenuEl.style.position = 'fixed';
    submenuEl.style.top = `${tr.top}px`;
    submenuEl.style.left = `${tr.right + 4}px`;

    const menuContainer = submenuTriggerEl.parentElement;
    if (!menuContainer) return;

    // 다른 메뉴 아이템 호버 시 닫기 (삼각형 영역 내는 제외)
    const handlePointerOver = (e: PointerEvent) => {
      if (!submenuEl || !submenuTriggerEl) return;

      const sr = submenuEl.getBoundingClientRect();
      if (isPointInTriangle(e.clientX, e.clientY, lastPointerPos.x, lastPointerPos.y, sr.left, sr.top - 10, sr.left, sr.bottom + 10))
        return;

      let el = e.target as HTMLElement | null;
      while (el && el !== menuContainer && el.parentElement !== menuContainer) {
        el = el.parentElement;
      }
      if (el && el !== menuContainer && el !== submenuTriggerEl) {
        submenuOpen = false;
      }
    };

    menuContainer.addEventListener('pointerover', handlePointerOver);
    return () => menuContainer.removeEventListener('pointerover', handlePointerOver);
  });

  $effect(() => {
    if (!open) {
      submenuOpen = false;
    }
  });

  $effect(() => {
    if (open) {
      untrack(() => app.state.openMenuCount++);
      return () => {
        untrack(() => app.state.openMenuCount--);
      };
    }
  });

  const staggerChildren = (node: HTMLElement) => {
    const update = () => {
      const { children } = node;
      for (const [i, child] of [...children].entries()) {
        (child as HTMLElement).style.setProperty('--i', String(i));
      }
    };
    update();
    const observer = new MutationObserver(update);
    observer.observe(node, { childList: true });
    return { destroy: () => observer.disconnect() };
  };
</script>

<div bind:this={panelEl} class={css({ position: 'relative', flex: '1', minWidth: '0' })}>
  <div
    class={css({
      position: 'absolute',
      top: '0',
      left: '0',
      right: '0',
      borderRadius: '8px',
      borderWidth: '1px',
      borderColor: open ? 'border.default' : 'transparent',
      boxShadow: open ? '[0 2px 8px rgba(0, 0, 0, 0.08), 0 1px 2px rgba(0, 0, 0, 0.06)]' : '[none]',
      _dark: {
        boxShadow: open ? '[0 2px 8px rgba(0, 0, 0, 0.3), 0 1px 2px rgba(0, 0, 0, 0.2)]' : '[none]',
      },
      backgroundColor: open ? 'surface.default' : 'transparent',
      transitionProperty: '[border-color, box-shadow, background-color]',
      transitionDuration: '200ms',
      transitionTimingFunction: 'ease-out',
      zIndex: '1',
    })}
  >
    <!-- 트리거 -->
    <button
      class={flex({
        alignItems: 'center',
        gap: '8px',
        width: 'full',
        paddingX: '8px',
        paddingY: '6px',
        cursor: 'pointer',
        transition: 'common',
        ...(!open && {
          _hover: { backgroundColor: 'surface.muted' },
        }),
      })}
      onclick={() => {
        open = !open;
      }}
      type="button"
    >
      <Img style={css.raw({ size: '20px', borderRadius: '4px', flexShrink: '0' })} alt={site.name} image$key={site.logo} size={32} />
      <span class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default', truncate: true })}>{site.name}</span>
      <Icon
        style={css.raw({
          marginLeft: 'auto',
          flexShrink: '0',
          color: 'text.faint',
          transitionProperty: '[transform]',
          transitionDuration: '200ms',
          transitionTimingFunction: 'ease',
          transform: open ? 'rotate(-180deg)' : 'rotate(0deg)',
        })}
        icon={ChevronDownIcon}
        size={14}
      />
    </button>

    {#if open}
      <div class={css({ paddingX: '4px' })}>
        <HorizontalDivider style={css.raw({ marginTop: '2px' })} color="secondary" />
      </div>
    {/if}

    <div
      class={css({
        display: 'grid',
        gridTemplateRows: open ? '1fr' : '0fr',
        transitionProperty: '[grid-template-rows]',
        transitionDuration: '200ms',
        transitionTimingFunction: 'cubic-bezier(0.76, 0, 0.24, 1)',
      })}
    >
      <div
        class={css({
          overflow: 'hidden',
        })}
      >
        <div
          class={css({
            display: 'flex',
            flexDirection: 'column',
            padding: '4px',
            '& > *': {
              opacity: open ? '100' : '0',
              transitionProperty: '[opacity]',
              transitionDuration: open ? '100ms' : '80ms',
              transitionDelay: open ? '[calc(150ms + var(--i, 0) * 10ms)]' : '[0ms]',
              transitionTimingFunction: 'ease-out',
            },
          })}
          inert={!open}
          use:staggerChildren
        >
          <button
            class={flex({
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingY: '6px',
              borderRadius: '6px',
              fontSize: '13px',
              fontWeight: 'medium',
              color: 'text.default',
              transition: 'common',
              cursor: 'pointer',
              _hover: { backgroundColor: 'surface.muted' },
            })}
            onclick={() => {
              close();
              pushState('', { shallowRoute: '/site-settings/general' });
              mixpanel.track('open_site_settings', { via: 'sidebar' });
            }}
            type="button"
          >
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={SettingsIcon} size={14} />
            <span>스페이스 설정</span>
          </button>

          <a
            class={flex({
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingY: '6px',
              borderRadius: '6px',
              fontSize: '13px',
              fontWeight: 'medium',
              color: 'text.default',
              transition: 'common',
              _hover: { backgroundColor: 'surface.muted' },
            })}
            href={site.url}
            rel="noopener noreferrer"
            target="_blank"
          >
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={ExternalLinkIcon} size={14} />
            <span>스페이스 열기</span>
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={ArrowUpRightIcon} size={12} />
          </a>

          <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />

          <button
            class={flex({
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingY: '6px',
              borderRadius: '6px',
              fontSize: '13px',
              fontWeight: 'medium',
              color: 'text.default',
              transition: 'common',
              cursor: 'pointer',
              _hover: { backgroundColor: 'surface.muted' },
            })}
            onclick={() => {
              close();
              app.state.trashOpen = true;
              mixpanel.track('open_trash_modal', { via: 'sidebar' });
            }}
            type="button"
          >
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={Trash2Icon} size={14} />
            <span>휴지통</span>
          </button>

          <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />

          <!-- 스페이스 전환 서브메뉴 트리거 -->
          <div
            bind:this={submenuTriggerEl}
            class={flex({
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingY: '6px',
              borderRadius: '6px',
              fontSize: '13px',
              fontWeight: 'medium',
              color: 'text.default',
              transition: 'common',
              cursor: 'pointer',
              backgroundColor: submenuOpen ? 'surface.muted' : 'transparent',
              _hover: { backgroundColor: 'surface.muted' },
            })}
            onpointerenter={() => {
              submenuOpen = true;
            }}
            onpointermove={(e) => {
              lastPointerPos = { x: e.clientX, y: e.clientY };
            }}
            role="button"
            tabindex="0"
          >
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={ChevronsUpDownIcon} size={14} />
            <span>스페이스 전환</span>
            <Icon style={css.raw({ marginLeft: 'auto', flexShrink: '0', color: 'text.faint' })} icon={ChevronRightIcon} size={12} />
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- 트리거 버튼 높이(paddingY 6px × 2 + 20px 콘텐츠 + border 1px × 2 = 34px) 만큼 공간 확보 -->
  <div class={css({ height: '34px' })}></div>
</div>

<!-- 스페이스 전환 서브메뉴 (포탈) -->
{#if submenuOpen}
  <div
    bind:this={submenuEl}
    class={css({
      borderRadius: '8px',
      padding: '4px',
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
      minWidth: '160px',
    })}
    use:portal
  >
    {#each user.data.sites as s (s.id)}
      <button
        class={flex({
          alignItems: 'center',
          gap: '8px',
          width: 'full',
          paddingX: '8px',
          paddingY: '6px',
          borderRadius: '6px',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.default',
          whiteSpace: 'nowrap',
          transition: 'common',
          cursor: 'pointer',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        onclick={() => {
          paneGroup.switchToSite(s.id);
          close();
        }}
        type="button"
      >
        <Img style={css.raw({ size: '16px', borderRadius: '4px', flexShrink: '0' })} alt={s.name} image$key={s.logo} size={32} />
        <span class={css({ truncate: true })}>{s.name}</span>
        {#if s.id === site.id}
          <Icon
            style={css.raw({ marginLeft: 'auto', paddingLeft: '4px', flexShrink: '0', color: 'text.brand' })}
            icon={CheckIcon}
            size={14}
          />
        {/if}
      </button>
    {/each}

    <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />

    <button
      class={flex({
        alignItems: 'center',
        gap: '8px',
        width: 'full',
        paddingX: '8px',
        paddingY: '6px',
        borderRadius: '6px',
        fontSize: '13px',
        fontWeight: 'medium',
        color: 'text.muted',
        whiteSpace: 'nowrap',
        transition: 'common',
        cursor: 'pointer',
        _hover: { backgroundColor: 'surface.muted' },
      })}
      onclick={() => {
        if (!user.data.subscription) {
          close();
          PlanUpgradeDialog.show({
            message: 'FULL ACCESS로 업그레이드하면\n여러 스페이스를 만들어 관리할 수 있어요.',
          });
          return;
        }

        close();
        createSiteModalOpen = true;
      }}
      type="button"
    >
      <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={PlusIcon} size={14} />
      <span>새 스페이스 생성</span>
    </button>
  </div>
{/if}

<CreateSiteModal userId={user.data.id} bind:open={createSiteModalOpen} />
