<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { focusTrap, portal, scrollLock } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { cubicOut } from 'svelte/easing';
  import { fade, fly } from 'svelte/transition';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import XIcon from '~icons/lucide/x';
  import { SOCIAL_LINKS } from '$lib/footer';
  import { siteUrl } from '$lib/site';
  import Wordmark from './Wordmark.svelte';

  type Props = { open: boolean };

  let { open = $bindable() }: Props = $props();

  const NAV_LINKS = [
    { label: '구독 안내', path: '/pricing', local: true },
    { label: '업데이트 노트', path: '/changelog', local: true },
    { label: '다운로드', path: '/download', local: true },
  ] as const;

  const reduced = $derived(prefersReducedMotion.current);

  const close = () => {
    open = false;
  };

  $effect(() => {
    if (!open) return;
    return pushEscapeHandler(() => {
      close();
      return true;
    });
  });

  const hostClass = css({ position: 'fixed', inset: '0', zIndex: 'modal', display: { base: 'block', md: 'none' } });
  const backdropClass = css({ position: 'absolute', inset: '0', backgroundColor: 'scrim' });
  const panelClass = flex({
    position: 'absolute',
    inset: '0',
    direction: 'column',
    backgroundColor: 'surface.canvas',
    overflowY: 'auto',
    overscrollBehavior: 'contain',
  });
  const barClass = flex({
    align: 'center',
    justify: 'space-between',
    flexShrink: '0',
    height: '56px',
    paddingLeft: '20px',
    paddingRight: '10px',
  });
  const closeClass = flex({
    align: 'center',
    justify: 'center',
    size: '44px',
    borderRadius: 'full',
    color: 'text.muted',
    transition: '[color 0.2s ease-out]',
    _hover: { color: 'text.default' },
  });
  const navClass = flex({ direction: 'column', flexGrow: '1', paddingX: '20px', paddingTop: '12px' });
  const linkClass = css({
    display: 'flex',
    alignItems: 'center',
    minHeight: '[56px]',
    borderBottomWidth: '1px',
    borderBottomColor: 'border.hairline',
    fontFamily: 'ui',
    fontSize: '20px',
    fontWeight: 'semibold',
    letterSpacing: '[-0.02em]',
    color: 'text.default',
  });
  const footClass = flex({ direction: 'column', gap: '16px', flexShrink: '0', paddingX: '20px', paddingY: '24px' });
  const ctaClass = cx(
    'group',
    flex({
      align: 'center',
      justify: 'center',
      gap: '8px',
      height: '52px',
      borderRadius: 'full',
      backgroundColor: 'surface.inverse',
      color: 'text.on.inverse',
      fontFamily: 'ui',
      fontSize: '16px',
      fontWeight: 'semibold',
      letterSpacing: '[-0.01em]',
    }),
  );
  const noteClass = css({ fontFamily: 'ui', fontSize: '13px', textAlign: 'center', color: 'text.hint' });
  const socialClass = flex({ align: 'center', justify: 'center', gap: '20px', paddingTop: '8px' });
  const socialLinkClass = css({
    display: 'flex',
    padding: '8px',
    color: 'text.hint',
    transition: '[color 0.2s ease-out]',
    _hover: { color: 'text.default' },
  });
</script>

{#if open}
  <div
    id="landing-mobile-menu"
    class={hostClass}
    use:focusTrap={{ escapeDeactivates: false, returnFocusOnDeactivate: true, allowOutsideClick: true }}
    use:portal
    use:scrollLock
  >
    <div class={backdropClass} onclick={close} role="none" transition:fade|global={{ duration: reduced ? 0 : 160, easing: cubicOut }}></div>

    <div
      class={panelClass}
      aria-label="메뉴"
      role="dialog"
      transition:fly|global={{ y: -8, duration: reduced ? 0 : 200, easing: cubicOut }}
    >
      <div class={barClass}>
        <a href="/" onclick={close}><Wordmark height="18px" /></a>
        <button class={closeClass} aria-label="메뉴 닫기" onclick={close} type="button">
          <Icon icon={XIcon} size={20} />
        </button>
      </div>

      <nav class={navClass}>
        {#each NAV_LINKS as link (link.path)}
          <a class={linkClass} href={link.local ? link.path : siteUrl(link.path)} onclick={close}>{link.label}</a>
        {/each}
      </nav>

      <div class={footClass}>
        <a class={ctaClass} href={siteUrl('/start')} onclick={close}>
          시작하기
          <Icon
            style={css.raw({ transition: '[transform 0.2s ease-out]', _groupHover: { transform: 'translateX(2px)' } })}
            icon={ArrowRightIcon}
            size={16}
          />
        </a>
        <p class={noteClass}>2주 무료 · 카드 등록 없이 시작할 수 있어요</p>
        <div class={socialClass}>
          {#each SOCIAL_LINKS as social (social.label)}
            <a class={socialLinkClass} aria-label={social.label} href={social.href} rel="noopener noreferrer" target="_blank">
              <Icon icon={social.icon} size={20} />
            </a>
          {/each}
        </div>
      </div>
    </div>
  </div>
{/if}
