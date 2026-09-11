<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import MenuIcon from '~icons/lucide/menu';
  import { EnvironmentBanner } from '$lib/components';
  import MobileMenu from './MobileMenu.svelte';
  import Wordmark from './Wordmark.svelte';

  type Props = { floating: boolean };

  let { floating }: Props = $props();

  let menuOpen = $state(false);

  const NAV_LINKS = [
    { label: '구독 안내', path: '/pricing' },
    { label: '업데이트 노트', path: '/changelog' },
    { label: '다운로드', path: '/download' },
  ] as const;

  const EASE = '320ms cubic-bezier(0.4, 0, 0.2, 1)';
  const SHELL_MOTION = `[transform ${EASE}, max-width ${EASE}]` as const;
  const PILL_MOTION = `[border-radius ${EASE}, background-color ${EASE}, box-shadow ${EASE}, backdrop-filter ${EASE}]` as const;
  const RIM_MOTION = `[opacity ${EASE}]` as const;
  const ROW_MOTION = `[padding-left ${EASE}, padding-right ${EASE}]` as const;
  const GUTTER = '[max(10px, calc((100vw - 1600px) / 2))]';

  const shellClass = css({
    position: 'fixed',
    top: '0',
    left: '[50%]',
    zIndex: '50',
    width: 'full',
    maxWidth: '[100vw]',
    transform: '[translate(-50%, 0)]',
    fontFamily: 'landing',
    transition: SHELL_MOTION,
    _motionReduce: { transition: '[none]' },
    '&[data-floating="true"]': {
      transform: { base: '[translate(-50%, 8px)]', lg: '[translate(-50%, 12px)]' },
      maxWidth: { base: '[calc(100vw - 16px)]', lg: '[min(1600px, calc(100vw - 400px))]' },
    },
  });
  const pillClass = css({
    position: 'relative',
    height: '56px',
    borderRadius: '[0px]',
    backgroundColor: 'surface.canvas',
    boxShadow: '[none]',
    backdropFilter: '[blur(0px) saturate(100%)]',
    transition: PILL_MOTION,
    _motionReduce: { transition: '[none]' },
    _after: {
      content: '""',
      position: 'absolute',
      inset: '0',
      borderRadius: '[inherit]',
      padding: '1px',
      background:
        '[linear-gradient(180deg, color-mix(in srgb, token(colors.text.default) 14%, transparent), color-mix(in srgb, token(colors.text.default) 2%, transparent))]',
      mask: '[linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0)]',
      WebkitMaskComposite: 'xor',
      maskComposite: 'exclude',
      opacity: '0',
      transition: RIM_MOTION,
      pointerEvents: 'none',
    },
    '&[data-floating="true"]': {
      borderRadius: '[28px]',
      backgroundColor: 'surface.canvas/75',
      backdropFilter: '[blur(20px) saturate(150%)]',
      boxShadow:
        '[inset 0 1px 0 color-mix(in srgb, token(colors.text.default) 8%, transparent), 0 12px 32px color-mix(in srgb, token(colors.shadow.default) 35%, transparent)]',
      _after: { opacity: '100' },
    },
  });
  const rowClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '1fr auto', md: '1fr auto 1fr' },
    alignItems: 'center',
    gap: { base: '16px', md: '32px' },
    height: 'full',
    paddingLeft: GUTTER,
    paddingRight: GUTTER,
    transition: ROW_MOTION,
    _motionReduce: { transition: '[none]' },
    '[data-floating="true"] &': { paddingLeft: '10px', paddingRight: '10px' },
  });
  const wordmarkClass = flex({
    alignItems: 'center',
    height: '36px',
    paddingX: '16px',
    transition: '[opacity 0.2s ease-out]',
    _hover: { opacity: '[0.7]' },
  });
  const navClass = flex({ alignItems: 'center', justifySelf: 'center', display: { base: 'none', md: 'flex' } });
  const linkClass = css({
    display: 'inline-flex',
    alignItems: 'center',
    height: '36px',
    paddingX: '16px',
    borderRadius: 'full',
    fontSize: '14px',
    fontWeight: 'medium',
    color: 'text.muted',
    transition: '[color 0.2s ease-out]',
    _hover: { color: 'text.default' },
  });
  const ctaClass = cx(
    'group',
    css({
      display: { base: 'none', md: 'inline-flex' },
      alignItems: 'center',
      gap: '8px',
      height: '36px',
      paddingLeft: '16px',
      paddingRight: '14px',
      borderRadius: 'full',
      backgroundColor: 'surface.inverse',
      color: 'text.on.inverse',
      fontSize: '14px',
      fontWeight: 'semibold',
      letterSpacing: '-0.01em',
      transition: '[background-color 0.2s ease-out]',
      _hover: { backgroundColor: '[color-mix(in oklab, token(colors.surface.inverse) 88%, token(colors.surface.canvas))]' },
    }),
  );
  const menuClass = flex({
    display: { base: 'flex', md: 'none' },
    alignItems: 'center',
    justifyContent: 'center',
    size: '36px',
    borderRadius: 'full',
    color: 'text.muted',
    transition: '[color 0.2s ease-out]',
    _hover: { color: 'text.default' },
  });
</script>

<header class={shellClass} data-floating={floating}>
  <EnvironmentBanner />
  <div class={pillClass} data-floating={floating}>
    <div class={rowClass}>
      <a class={wordmarkClass} href="/"><Wordmark height="18px" /></a>
      <nav class={navClass}>
        {#each NAV_LINKS as link (link.path)}
          <a class={linkClass} href={link.path}>{link.label}</a>
        {/each}
      </nav>
      <div class={flex({ alignItems: 'center', justifySelf: 'end', gap: '8px' })}>
        <a class={ctaClass} href="/start">
          시작하기
          <Icon
            style={css.raw({ transition: '[transform 0.2s ease-out]', _groupHover: { transform: 'translateX(2px)' } })}
            icon={ArrowRightIcon}
            size={14}
          />
        </a>
        <button
          class={menuClass}
          aria-controls="landing-mobile-menu"
          aria-expanded={menuOpen}
          aria-label="메뉴 열기"
          onclick={() => (menuOpen = true)}
          type="button"
        >
          <Icon icon={MenuIcon} size={20} />
        </button>
      </div>
    </div>
  </div>
</header>

<MobileMenu bind:open={menuOpen} />
