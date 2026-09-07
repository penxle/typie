<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import { page } from '$app/state';
  import ShareLinkPopover from '$lib/usersite/ShareLinkPopover.svelte';
  import { getUsersiteChrome } from './chrome.svelte';
  import ThemeFlipButton from './ThemeFlipButton.svelte';

  const chrome = getUsersiteChrome();

  const href = $derived(chrome.post?.url ?? page.url.origin);

  const buttonStyle = css.raw({
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    marginLeft: '0',
    padding: '0',
    width: { base: '40px', md: '36px' },
    height: { base: '36px', md: '32px' },
    borderRadius: '8px',
    color: 'text.muted',
    transition: 'colors',
    _hover: { color: 'text.default', backgroundColor: 'transparent' },
  });

  const scrollToTop = () => {
    window.scrollTo({ top: 0, behavior: prefersReducedMotion.current ? 'auto' : 'smooth' });
  };
</script>

<div
  class={css({
    position: 'fixed',
    left: '0',
    right: '0',
    bottom: '0',
    zIndex: '40',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-around',
    height: '48px',
    paddingX: '12px',
    borderTopWidth: '1px',
    borderColor: 'border.default',
    backgroundColor: 'surface.default',
    transition: '[transform 200ms cubic-bezier(0.32, 0.72, 0, 1)]',
    '&[data-hidden]': { transform: '[translateY(100%)]', transitionDuration: '[160ms]', transitionTimingFunction: 'ease-in' },
    md: {
      left: '1/2',
      right: '[auto]',
      bottom: '24px',
      width: 'auto',
      height: '44px',
      gap: '4px',
      paddingX: '6px',
      borderWidth: '0',
      borderRadius: 'full',
      transform: '[translateX(-50%)]',
      backgroundColor: '[color-mix(in srgb, token(colors.surface.default) 88%, transparent)]',
      backdropFilter: '[blur(12px) saturate(1.2)]',
      boxShadow: '[0 0 0 1px token(colors.border.hairline), token(shadows.lg)]',
      '&[data-hidden]': { transform: '[translate(-50%, 80px)]' },
    },
    _motionReduce: { transition: '[none]' },
  })}
  data-hidden={chrome.retreat || undefined}
>
  <ThemeFlipButton style={buttonStyle} tooltipPlacement="top" via="toolbar" />
  <ShareLinkPopover style={buttonStyle} {href} iconSize={18} />
  <button
    class={css(buttonStyle)}
    aria-label="맨 위로"
    onclick={scrollToTop}
    type="button"
    use:tooltip={{ message: '맨 위로', placement: 'top' }}
  >
    <Icon icon={ArrowUpIcon} size={18} />
  </button>
</div>
