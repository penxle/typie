<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { APP_STORE_URL, PLAY_STORE_URL } from '@typie/lib/const';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { generate } from 'lean-qr';
  import { toSvgSource } from 'lean-qr/extras/svg';
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import { untrack } from 'svelte';
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import EclipseIcon from '~icons/lucide/eclipse';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import KeyboardIcon from '~icons/lucide/keyboard';
  import LogOutIcon from '~icons/lucide/log-out';
  import MessageCircleQuestionIcon from '~icons/lucide/message-circle-question';
  import MonitorIcon from '~icons/lucide/monitor';
  import MoonIcon from '~icons/lucide/moon';
  import NewspaperIcon from '~icons/lucide/newspaper';
  import QrCodeIcon from '~icons/lucide/qr-code';
  import Repeat2Icon from '~icons/lucide/repeat-2';
  import SettingsIcon from '~icons/lucide/settings';
  import ShieldUserIcon from '~icons/lucide/shield-user';
  import SmartphoneIcon from '~icons/lucide/smartphone';
  import SunIcon from '~icons/lucide/sun';
  import AppleIcon from '~icons/simple-icons/apple';
  import DiscordIcon from '~icons/simple-icons/discord';
  import GooglePlayIcon from '~icons/simple-icons/googleplay';
  import XBrandIcon from '~icons/simple-icons/x';
  import { pushState } from '$app/navigation';
  import { env } from '$env/dynamic/public';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import type { DashboardLayout_Profile_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_Profile_user$key;
    open?: boolean;
  };

  let { user$key, open = $bindable(false) }: Props = $props();

  const app = getAppContext();
  const theme = getThemeContext();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_Profile_user on User {
        id
        name
        role

        avatar {
          id
          ...Img_image
        }

        subscription {
          id
        }
      }
    `),
    () => user$key,
  );

  let panelEl = $state<HTMLDivElement>();
  let mobileQrOpen = $state(false);
  const qrSvg = toSvgSource(generate(`${env.PUBLIC_WEBSITE_URL}/app`), {
    on: 'currentColor',
    off: 'transparent',
    pad: 0,
    width: 200,
    height: 200,
  });

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

      document.addEventListener('click', handleClickOutside, { capture: true });

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
    if (!open) {
      return;
    }

    untrack(() => app.state.openMenuCount++);
    return () => {
      untrack(() => app.state.openMenuCount--);
    };
  });

  const staggerChildren = (node: HTMLElement) => {
    const update = () => {
      const { children } = node;
      for (let i = 0; i < children.length; i++) {
        (children[i] as HTMLElement).style.setProperty('--i', String(children.length - 1 - i));
      }
    };
    update();
    const observer = new MutationObserver(update);
    observer.observe(node, { childList: true });
    return { destroy: () => observer.disconnect() };
  };
</script>

<div bind:this={panelEl} class={css({ position: 'relative', paddingX: '12px', paddingY: '8px' })}>
  <!-- 트리거 버튼 높이(paddingY 6px × 2 + 24px 콘텐츠 + border 1px × 2 = 38px) 만큼 공간 확보 -->
  <div class={css({ height: '38px' })}></div>

  <div
    class={css({
      position: 'absolute',
      bottom: '8px',
      left: '12px',
      right: '12px',
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
              color: 'text.danger',
              transition: 'common',
              cursor: 'pointer',
              _hover: { backgroundColor: 'surface.muted' },
            })}
            onclick={() => {
              mixpanel.track('logout', { via: 'sidebar' });

              location.assign(
                qs.stringifyUrl({
                  url: '/logout',
                  query: {
                    redirect_uri: env.PUBLIC_WEBSITE_URL,
                  },
                }),
              );
            }}
            type="button"
          >
            <Icon style={css.raw({ flexShrink: '0' })} icon={LogOutIcon} size={14} />
            <span>로그아웃</span>
          </button>

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
              mobileQrOpen = true;
            }}
            type="button"
          >
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={SmartphoneIcon} size={14} />
            <span>타이피 모바일</span>
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={QrCodeIcon} size={12} />
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
            href="https://penxle.channel.io"
            rel="noopener noreferrer"
            target="_blank"
          >
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={MessageCircleQuestionIcon} size={14} />
            <span>고객센터</span>
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={ArrowUpRightIcon} size={12} />
          </a>

          <!-- 더보기 서브메뉴 트리거 -->
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
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={EllipsisIcon} size={14} />
            <span>더보기</span>
            <Icon style={css.raw({ marginLeft: 'auto', flexShrink: '0', color: 'text.faint' })} icon={ChevronRightIcon} size={12} />
          </div>

          <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />

          <div
            class={flex({
              alignItems: 'center',
              gap: '8px',
              paddingX: '8px',
              paddingTop: '6px',
              paddingBottom: '4px',
            })}
          >
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={EclipseIcon} size={14} />
            <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>테마</span>
            <div
              class={flex({
                marginLeft: 'auto',
                alignItems: 'center',
                borderRadius: '6px',
                padding: '2px',
                backgroundColor: 'surface.muted',
              })}
            >
              {#each [{ value: 'auto' as const, icon: MonitorIcon }, { value: 'light' as const, icon: SunIcon }, { value: 'dark' as const, icon: MoonIcon }] as t (t.value)}
                <button
                  class={center({
                    borderRadius: '4px',
                    size: '20px',
                    color: theme.currentTheme === t.value ? 'text.default' : 'text.faint',
                    backgroundColor: theme.currentTheme === t.value ? 'surface.default' : 'transparent',
                    boxShadow: theme.currentTheme === t.value ? 'small' : '[none]',
                    transition: 'common',
                    cursor: 'pointer',
                    _hover: theme.currentTheme === t.value ? {} : { color: 'text.subtle' },
                  })}
                  onclick={() => {
                    mixpanel.track('switch_theme', { old: theme.currentTheme, new: t.value, via: 'sidebar' });
                    theme.currentTheme = t.value;
                  }}
                  type="button"
                >
                  <Icon icon={t.icon} size={12} />
                </button>
              {/each}
            </div>
          </div>

          {#if user.data.role === 'ADMIN'}
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
              href="/admin"
            >
              <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={ShieldUserIcon} size={14} />
              <span>어드민</span>
            </a>
          {/if}

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
              pushState('', { shallowRoute: '/preference/profile' });
              mixpanel.track('open_preference_modal', { via: 'sidebar' });
            }}
            type="button"
          >
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={SettingsIcon} size={14} />
            <span>설정</span>
          </button>
        </div>
      </div>
    </div>

    {#if open}
      <div class={css({ paddingX: '4px' })}>
        <HorizontalDivider style={css.raw({ marginBottom: '2px' })} color="secondary" />
      </div>
    {/if}

    <!-- 프로필 트리거 -->
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
      <Img
        style={css.raw({ size: '24px', borderRadius: 'full', flexShrink: '0' })}
        alt={user.data.name}
        image$key={user.data.avatar}
        size={32}
      />
      <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', truncate: true })}>
        {user.data.name}
      </span>
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
        icon={ChevronUpIcon}
        size={14}
      />
    </button>
  </div>
</div>

<!-- 커뮤니티 서브메뉴 (포탈) -->
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
    {#if user.data.subscription}
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
          whiteSpace: 'nowrap',
          transition: 'common',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        href="https://discord.gg/MteQ9AMa4B"
        rel="noopener noreferrer"
        target="_blank"
      >
        <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={DiscordIcon} size={14} />
        <span>유저 커뮤니티</span>
        <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={ArrowUpRightIcon} size={12} />
      </a>
    {/if}

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
        whiteSpace: 'nowrap',
        transition: 'common',
        _hover: { backgroundColor: 'surface.muted' },
      })}
      href="https://x.com/typieofficial"
      rel="noopener noreferrer"
      target="_blank"
    >
      <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={Repeat2Icon} size={14} />
      <span class={flex({ alignItems: 'center', gap: '4px' })}>공식 <Icon icon={XBrandIcon} size={10} /></span>
      <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={ArrowUpRightIcon} size={12} />
    </a>

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
        whiteSpace: 'nowrap',
        transition: 'common',
        _hover: { backgroundColor: 'surface.muted' },
      })}
      href="/changelog"
      rel="noopener noreferrer"
      target="_blank"
    >
      <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={NewspaperIcon} size={14} />
      <span>업데이트 노트</span>
      <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={ArrowUpRightIcon} size={12} />
    </a>

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
        color: 'text.default',
        whiteSpace: 'nowrap',
        transition: 'common',
        cursor: 'pointer',
        _hover: { backgroundColor: 'surface.muted' },
      })}
      onclick={() => {
        close();
        app.state.shortcutsOpen = true;
        mixpanel.track('open_shortcuts_modal', { via: 'sidebar' });
      }}
      type="button"
    >
      <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={KeyboardIcon} size={14} />
      <span>단축키</span>
    </button>
  </div>
{/if}

<Modal
  style={css.raw({ maxWidth: '320px', padding: '0' })}
  onclose={() => {
    mobileQrOpen = false;
  }}
  open={mobileQrOpen}
>
  <div class={flex({ direction: 'column', alignItems: 'center', gap: '16px', padding: '32px' })}>
    <div class={css({ size: '200px', color: 'text.default' })}>
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      {@html qrSvg}
    </div>
    <div class={flex({ direction: 'column', alignItems: 'center', gap: '4px' })}>
      <span class={css({ fontSize: '15px', fontWeight: 'semibold', color: 'text.default' })}>타이피 - 작가를 위한 글쓰기 도구</span>
      <span class={css({ fontSize: '13px', color: 'text.faint' })}>모바일에서도 글쓰기를 이어갈 수 있어요</span>
    </div>

    <div class={flex({ gap: '8px', width: 'full' })}>
      <a
        class={flex({
          flex: '1',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '6px',
          paddingY: '10px',
          borderRadius: '8px',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.default',
          backgroundColor: 'surface.muted',
          transition: 'common',
          _hover: { backgroundColor: 'interactive.hover' },
        })}
        href={APP_STORE_URL}
        rel="noopener noreferrer"
        target="_blank"
      >
        <Icon icon={AppleIcon} size={14} />
        App Store
      </a>
      <a
        class={flex({
          flex: '1',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '6px',
          paddingY: '10px',
          borderRadius: '8px',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.default',
          backgroundColor: 'surface.muted',
          transition: 'common',
          _hover: { backgroundColor: 'interactive.hover' },
        })}
        href={PLAY_STORE_URL}
        rel="noopener noreferrer"
        target="_blank"
      >
        <Icon icon={GooglePlayIcon} size={14} />
        Google Play
      </a>
    </div>
  </div>
</Modal>
