<script lang="ts">
  import { APP_STORE_URL, DESKTOP_MAC_ARM64_URL, DESKTOP_MAC_X64_URL, DESKTOP_WIN_X64_URL, PLAY_STORE_URL } from '@typie/lib/const';
  import { css, cx } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import { onMount } from 'svelte';
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
  import DownloadIcon from '~icons/lucide/download';
  import AppleIcon from '~icons/simple-icons/apple';
  import AppStoreIcon from '~icons/simple-icons/appstore';
  import GooglePlayIcon from '~icons/simple-icons/googleplay';
  import WindowsIcon from '~icons/simple-icons/windows';
  import { inview } from '../(index)/inview';
  import type { Component } from 'svelte';

  type Download = { id: string; icon: Component; platform: string; detail: string; url: string; external?: boolean };

  const desktop: Download[] = [
    {
      id: 'mac-arm64',
      icon: AppleIcon,
      platform: 'macOS',
      detail: 'Apple Silicon · .dmg',
      url: DESKTOP_MAC_ARM64_URL,
    },
    {
      id: 'mac-x64',
      icon: AppleIcon,
      platform: 'macOS',
      detail: 'Intel · .dmg',
      url: DESKTOP_MAC_X64_URL,
    },
    {
      id: 'win-x64',
      icon: WindowsIcon,
      platform: 'Windows',
      detail: 'x64 · .exe',
      url: DESKTOP_WIN_X64_URL,
    },
  ];

  const mobile: Download[] = [
    { id: 'ios', icon: AppStoreIcon, platform: 'iOS', detail: 'App Store', url: APP_STORE_URL, external: true },
    { id: 'android', icon: GooglePlayIcon, platform: 'Android', detail: 'Google Play', url: PLAY_STORE_URL, external: true },
  ];

  let detected = $state<'mac' | 'windows' | 'ios' | 'android' | null>(null);

  onMount(() => {
    const ua = navigator.userAgent;
    if (/iPhone|iPad|iPod/.test(ua) || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1)) detected = 'ios';
    else if (/Android/.test(ua)) detected = 'android';
    else if (/Macintosh/.test(ua)) detected = 'mac';
    else if (/Windows/.test(ua)) detected = 'windows';
  });

  const primaryByPlatform = {
    mac: { item: desktop[0], label: 'macOS용 다운로드', note: 'macOS 11 이상 · Apple Silicon' },
    windows: { item: desktop[2], label: 'Windows용 다운로드', note: 'Windows 10 이상 · x64' },
    ios: { item: mobile[0], label: 'App Store에서 받기', note: 'iPhone · iPad' },
    android: { item: mobile[1], label: 'Google Play에서 받기', note: 'Android' },
  } as const;

  const primary = $derived(detected ? primaryByPlatform[detected] : null);

  const revealClass = css({
    opacity: '0',
    transform: 'translate3d(0, 24px, 0)',
    transition: '[opacity 0.8s cubic-bezier(0.16, 1, 0.3, 1), transform 0.8s cubic-bezier(0.16, 1, 0.3, 1)]',
    '&.in-view': { opacity: '100', transform: 'translate3d(0, 0, 0)' },
  });

  const rowClass = cx(
    'group',
    flex({
      alignItems: 'center',
      gap: '16px',
      paddingX: { sm: '20px', lg: '24px' },
      paddingY: '18px',
      color: 'dark.gray.100',
      transition: '[background-color 0.2s ease-out]',
      _hover: { backgroundColor: 'dark.gray.900' },
      '& + &': { borderTopWidth: '1px', borderTopColor: 'dark.gray.800' },
    }),
  );
</script>

<Helmet description="타이피 데스크톱 앱과 모바일 앱을 내려받으세요." title="다운로드" />

<div class={css({ position: 'relative', minHeight: '[100vh]', backgroundColor: 'dark.gray.950' })}>
  <div
    class={css({
      position: 'absolute',
      left: { sm: '16px', lg: '48px' },
      top: '0',
      bottom: '0',
      width: '1px',
      backgroundColor: 'dark.gray.800',
      display: { sm: 'none', lg: 'block' },
    })}
  ></div>

  <section
    class={cx(
      css({
        position: 'relative',
        paddingTop: { sm: '100px', lg: '140px' },
        paddingBottom: { sm: '48px', lg: '64px' },
        paddingX: { sm: '24px', lg: '80px' },
      }),
      revealClass,
    )}
    {@attach inview}
  >
    <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
      <span
        class={css({
          display: 'block',
          fontSize: '[11px]',
          fontFamily: 'mono',
          color: 'dark.gray.500',
          letterSpacing: '[0.1em]',
          textTransform: 'uppercase',
          marginBottom: '24px',
        })}
      >
        Download
      </span>

      <h1
        class={css({
          fontSize: { sm: '[36px]', lg: '[56px]' },
          fontWeight: 'medium',
          color: 'dark.gray.100',
          lineHeight: '[1.2]',
          letterSpacing: '[-0.02em]',
          fontFamily: 'Paperlogy',
          marginBottom: '20px',
        })}
      >
        타이피 다운로드
      </h1>

      <p class={css({ fontSize: { sm: '16px', lg: '18px' }, color: 'dark.gray.400', lineHeight: '[1.65]', maxWidth: '[480px]' })}>
        데스크톱과 모바일, 어디서든 이어 쓰세요.
      </p>

      {#if primary}
        <div
          class={css({
            display: 'flex',
            flexDirection: { sm: 'column', lg: 'row' },
            alignItems: { sm: 'flex-start', lg: 'center' },
            gap: { sm: '16px', lg: '24px' },
            marginTop: { sm: '32px', lg: '40px' },
          })}
        >
          <a
            class={cx(
              'group',
              css({
                display: 'inline-flex',
                alignItems: 'center',
                gap: '10px',
                paddingX: '28px',
                paddingY: '16px',
                fontSize: '15px',
                fontWeight: 'semibold',
                color: 'dark.gray.950',
                backgroundColor: 'dark.brand.300',
                transition: '[background-color 0.2s ease-out]',
                _hover: { backgroundColor: 'dark.brand.200' },
              }),
            )}
            href={primary.item.url}
            rel={primary.item.external ? 'noopener noreferrer' : 'noopener'}
            target={primary.item.external ? '_blank' : undefined}
          >
            <Icon icon={primary.item.external ? ArrowUpRightIcon : DownloadIcon} size={18} />
            {primary.label}
          </a>
          <span class={css({ fontSize: '13px', color: 'dark.gray.500' })}>{primary.note}</span>
        </div>
      {/if}
    </div>
  </section>

  <section class={css({ position: 'relative', paddingBottom: { sm: '80px', lg: '120px' }, paddingX: { sm: '24px', lg: '80px' } })}>
    <div
      class={cx(
        grid({ columns: { sm: 1, lg: 2 }, gap: { sm: '32px', lg: '20px' } }),
        css({ maxWidth: '[1200px]', marginX: 'auto' }),
        revealClass,
        css({ transitionDelay: '[0.15s]' }),
      )}
      {@attach inview}
    >
      {#each [{ title: '데스크톱', note: 'macOS 11 이상 · Windows 10 이상', items: desktop }, { title: '모바일', note: 'iOS · Android', items: mobile }] as group (group.title)}
        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          <div class={flex({ alignItems: 'baseline', justifyContent: 'space-between', paddingX: '4px' })}>
            <h2 class={css({ fontSize: '15px', fontWeight: 'medium', color: 'dark.gray.100' })}>{group.title}</h2>
            <span class={css({ fontSize: '12px', color: 'dark.gray.500' })}>{group.note}</span>
          </div>
          <div class={css({ borderWidth: '1px', borderColor: 'dark.gray.800', backgroundColor: 'dark.gray.950' })}>
            {#each group.items as item (item.id)}
              <a
                class={rowClass}
                href={item.url}
                rel={item.external ? 'noopener noreferrer' : 'noopener'}
                target={item.external ? '_blank' : undefined}
              >
                <Icon style={css.raw({ color: 'dark.gray.400', flexShrink: '0' })} icon={item.icon} size={20} />
                <span class={css({ fontSize: '15px', fontWeight: 'medium' })}>{item.platform}</span>
                <span class={css({ fontSize: '13px', fontFamily: 'mono', color: 'dark.gray.500' })}>{item.detail}</span>
                <span class={css({ flex: '1' })}></span>
                <Icon
                  style={css.raw({ color: 'dark.gray.500', transition: '[color 0.2s ease-out]', _groupHover: { color: 'dark.brand.300' } })}
                  icon={item.external ? ArrowUpRightIcon : DownloadIcon}
                  size={18}
                />
              </a>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </section>
</div>
