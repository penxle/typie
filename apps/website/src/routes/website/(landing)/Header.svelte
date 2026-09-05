<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { outsideClick } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import MenuIcon from '~icons/lucide/menu';
  import XIcon from '~icons/lucide/x';
  import { afterNavigate } from '$app/navigation';
  import WordmarkWhite from '$assets/logos/wordmark-white.svg?component';
  import { EnvironmentBanner } from '$lib/components';

  let mobileMenuOpen = false;

  afterNavigate(() => {
    mobileMenuOpen = false;
  });
</script>

<header
  class={css({
    backgroundColor: 'surface.canvas',
    borderBottomWidth: '1px',
    borderBottomColor: 'border.hairline',
    position: 'fixed',
    top: '0',
    left: '0',
    right: '0',
    zIndex: '50',
  })}
>
  <EnvironmentBanner />
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      height: { sm: '56px', lg: '64px' },
      gap: { sm: '16px', lg: '32px' },
      maxWidth: '[1200px]',
      marginX: 'auto',
      paddingX: { sm: '24px', lg: '80px' },
    })}
  >
    <a
      class={css({
        position: 'relative',
        display: 'block',
        transition: '[opacity 0.2s ease-out]',
        _hover: {
          opacity: '[0.7]',
        },
      })}
      href="/"
    >
      <WordmarkWhite class={css({ height: { sm: '18px', lg: '20px' } })} />
    </a>

    <div class={flex({ alignItems: 'center', gap: '0', display: { sm: 'none', lg: 'flex' } })}>
      <a
        class={css({
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'text.muted',
          paddingY: '8px',
          paddingX: '20px',
          transition: '[color 0.2s ease-out]',
          _hover: {
            color: 'text.default',
          },
        })}
        href="/pricing"
      >
        구독 안내
      </a>

      <a
        class={css({
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'text.muted',
          paddingY: '8px',
          paddingX: '20px',
          transition: '[color 0.2s ease-out]',
          _hover: {
            color: 'text.default',
          },
        })}
        href="/changelog"
      >
        업데이트 노트
      </a>

      <a
        class={css({
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'text.muted',
          paddingY: '8px',
          paddingX: '20px',
          transition: '[color 0.2s ease-out]',
          _hover: {
            color: 'text.default',
          },
        })}
        href="/download"
      >
        다운로드
      </a>

      <a
        class={cx(
          'group',
          css({
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            marginLeft: '16px',
            paddingX: '20px',
            paddingY: '10px',
            backgroundColor: 'surface.inset',
            color: 'text.default',
            fontSize: '14px',
            fontWeight: 'medium',
            transition: '[all 0.2s ease-out]',
            _hover: {
              backgroundColor: 'surface.hover',
            },
          }),
        )}
        href="/start"
      >
        시작하기
        <Icon
          style={css.raw({
            transition: '[transform 0.2s ease-out]',
            _groupHover: {
              transform: 'translateX(2px)',
            },
          })}
          icon={ArrowRightIcon}
          size={14}
        />
      </a>
    </div>

    <div class={flex({ alignItems: 'center', gap: '8px', display: { sm: 'flex', lg: 'none' } })}>
      <button
        class={css({
          padding: '8px',
          backgroundColor: mobileMenuOpen ? 'surface.active' : 'transparent',
          color: mobileMenuOpen ? 'text.default' : 'text.muted',
          transition: '[color 0.2s ease-out]',
          _hover: {
            color: 'text.default',
          },
        })}
        onclick={() => (mobileMenuOpen = !mobileMenuOpen)}
        type="button"
      >
        <Icon icon={mobileMenuOpen ? XIcon : MenuIcon} size={20} />
      </button>
    </div>
  </div>
</header>

{#if mobileMenuOpen}
  <div
    class={css({
      position: 'fixed',
      top: '56px',
      left: '0',
      right: '0',
      backgroundColor: 'surface.canvas',
      borderBottomWidth: '1px',
      borderBottomColor: 'border.hairline',
      zIndex: '40',
      display: { sm: 'block', lg: 'none' },
    })}
    onoutsideclick={() => (mobileMenuOpen = false)}
    use:outsideClick
  >
    <a
      class={css({
        display: 'block',
        fontSize: '15px',
        fontWeight: 'medium',
        color: 'text.muted',
        paddingY: '16px',
        paddingX: '24px',
        borderBottomWidth: '1px',
        borderBottomColor: 'border.hairline',
        transition: '[color 0.2s ease-out]',
        _hover: {
          color: 'text.default',
        },
      })}
      href="/"
    >
      소개
    </a>

    <a
      class={css({
        display: 'block',
        fontSize: '15px',
        fontWeight: 'medium',
        color: 'text.muted',
        paddingY: '16px',
        paddingX: '24px',
        borderBottomWidth: '1px',
        borderBottomColor: 'border.hairline',
        transition: '[color 0.2s ease-out]',
        _hover: {
          color: 'text.default',
        },
      })}
      href="/pricing"
    >
      구독 안내
    </a>

    <a
      class={css({
        display: 'block',
        fontSize: '15px',
        fontWeight: 'medium',
        color: 'text.muted',
        paddingY: '16px',
        paddingX: '24px',
        borderBottomWidth: '1px',
        borderBottomColor: 'border.hairline',
        transition: '[color 0.2s ease-out]',
        _hover: {
          color: 'text.default',
        },
      })}
      href="/changelog"
    >
      업데이트 노트
    </a>

    <a
      class={css({
        display: 'block',
        fontSize: '15px',
        fontWeight: 'medium',
        color: 'text.muted',
        paddingY: '16px',
        paddingX: '24px',
        borderBottomWidth: '1px',
        borderBottomColor: 'border.hairline',
        transition: '[color 0.2s ease-out]',
        _hover: {
          color: 'text.default',
        },
      })}
      href="/download"
    >
      다운로드
    </a>

    <div class={css({ paddingX: '24px', paddingY: '16px' })}>
      <a
        class={cx(
          'group',
          css({
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            gap: '8px',
            paddingX: '20px',
            paddingY: '14px',
            backgroundColor: 'surface.inset',
            color: 'text.default',
            fontSize: '15px',
            fontWeight: 'medium',
            transition: '[all 0.2s ease-out]',
            _hover: {
              backgroundColor: 'surface.hover',
            },
          }),
        )}
        href="/start"
      >
        시작하기
        <Icon icon={ArrowRightIcon} size={14} />
      </a>
    </div>
  </div>
{/if}
