<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { outsideClick } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import MenuIcon from '~icons/lucide/menu';
  import XIcon from '~icons/lucide/x';
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import WordmarkWhite from '$assets/logos/wordmark-white.svg?component';

  let mobileMenuOpen = false;

  afterNavigate(() => {
    mobileMenuOpen = false;
  });
</script>

<header
  class={css({
    backgroundColor: 'dark.gray.950',
    borderBottomWidth: '1px',
    borderBottomColor: 'dark.gray.800',
    position: 'fixed',
    top: '0',
    left: '0',
    right: '0',
    zIndex: '50',
    paddingX: { sm: '24px', lg: '80px' },
  })}
>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      height: { sm: '56px', lg: '64px' },
      gap: { sm: '16px', lg: '32px' },
      maxWidth: '[1200px]',
      marginX: 'auto',
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
          color: 'dark.gray.400',
          paddingY: '8px',
          paddingX: '20px',
          transition: '[color 0.2s ease-out]',
          _hover: {
            color: 'dark.gray.100',
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
          color: 'dark.gray.400',
          paddingY: '8px',
          paddingX: '20px',
          transition: '[color 0.2s ease-out]',
          _hover: {
            color: 'dark.gray.100',
          },
        })}
        href="/changelog"
      >
        업데이트 노트
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
            backgroundColor: 'dark.gray.800',
            color: 'dark.gray.100',
            fontSize: '14px',
            fontWeight: 'medium',
            transition: '[all 0.2s ease-out]',
            _hover: {
              backgroundColor: 'dark.gray.700',
            },
          }),
        )}
        href={page.data.startUrl}
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
          backgroundColor: 'transparent',
          color: 'dark.gray.400',
          transition: '[color 0.2s ease-out]',
          _hover: {
            color: 'dark.gray.100',
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
      backgroundColor: 'dark.gray.950',
      borderBottomWidth: '1px',
      borderBottomColor: 'dark.gray.800',
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
        color: 'dark.gray.400',
        paddingY: '16px',
        paddingX: '24px',
        borderBottomWidth: '1px',
        borderBottomColor: 'dark.gray.900',
        transition: '[color 0.2s ease-out]',
        _hover: {
          color: 'dark.gray.100',
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
        color: 'dark.gray.400',
        paddingY: '16px',
        paddingX: '24px',
        borderBottomWidth: '1px',
        borderBottomColor: 'dark.gray.900',
        transition: '[color 0.2s ease-out]',
        _hover: {
          color: 'dark.gray.100',
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
        color: 'dark.gray.400',
        paddingY: '16px',
        paddingX: '24px',
        borderBottomWidth: '1px',
        borderBottomColor: 'dark.gray.900',
        transition: '[color 0.2s ease-out]',
        _hover: {
          color: 'dark.gray.100',
        },
      })}
      href="/changelog"
    >
      업데이트 노트
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
            backgroundColor: 'dark.gray.800',
            color: 'dark.gray.100',
            fontSize: '15px',
            fontWeight: 'medium',
            transition: '[all 0.2s ease-out]',
            _hover: {
              backgroundColor: 'dark.gray.700',
            },
          }),
        )}
        href={page.data.startUrl}
      >
        시작하기
        <Icon icon={ArrowRightIcon} size={14} />
      </a>
    </div>
  </div>
{/if}
