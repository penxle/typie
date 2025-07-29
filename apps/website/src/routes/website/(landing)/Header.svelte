<script lang="ts">
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import MenuIcon from '~icons/lucide/menu';
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import WordmarkBlack from '$assets/logos/wordmark-black.svg?component';
  import { outsideClick } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  let mobileMenuOpen = false;

  afterNavigate(() => {
    mobileMenuOpen = false;
  });
</script>

<header
  class={flex({
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingX: { sm: '16px', lg: '24px' },
    height: { sm: '64px', lg: '80px' },
    gap: { sm: '16px', lg: '32px' },
    backgroundColor: 'white',
    border: '4px solid',
    borderColor: 'gray.900',
    position: 'fixed',
    top: { sm: '8px', lg: '16px' },
    left: { sm: '8px', lg: '16px' },
    right: { sm: '8px', lg: '16px' },
    zIndex: '50',
    boxShadow: '[8px 8px 0 0 #000]',
  })}
>
  <a
    class={css({
      position: 'relative',
      display: 'block',
      transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
      _hover: {
        transform: 'translateY(-2px) rotate(-1deg)',
      },
    })}
    href="/"
  >
    <WordmarkBlack class={css({ height: { sm: '18px', lg: '22px' } })} />
  </a>

  <div class={flex({ alignItems: 'center', gap: '24px', display: { sm: 'none', lg: 'flex' } })}>
    <a
      class={css({
        fontSize: '16px',
        fontWeight: 'bold',
        color: 'gray.900',
        textTransform: 'uppercase',
        letterSpacing: '[0.05em]',
        paddingY: '6px',
        paddingX: '16px',
        position: 'relative',
        transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
        _hover: {
          backgroundColor: 'amber.400',
          transform: 'translateY(-2px) rotate(-1deg)',
        },
      })}
      href="/pricing"
    >
      구독 안내
    </a>

    <a
      class={css({
        fontSize: '16px',
        fontWeight: 'bold',
        color: 'gray.900',
        textTransform: 'uppercase',
        letterSpacing: '[0.05em]',
        paddingY: '6px',
        paddingX: '16px',
        position: 'relative',
        transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
        _hover: {
          backgroundColor: 'amber.400',
          transform: 'translateY(-2px) rotate(-1deg)',
        },
      })}
      href="/changelog"
    >
      업데이트 노트
    </a>

    <a
      class={cx(
        'group',
        center({
          gap: '8px',
          paddingX: '24px',
          paddingY: '10px',
          backgroundColor: 'gray.900',
          color: 'white',
          fontSize: '16px',
          fontWeight: 'black',
          textTransform: 'uppercase',
          letterSpacing: '[0.05em]',
          border: '4px solid',
          borderColor: 'gray.900',
          position: 'relative',
          boxShadow: '[4px 4px 0 0 #000]',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-2px, -2px)',
            boxShadow: '[6px 6px 0 0 #000]',
          },
          _active: {
            transform: 'translate(2px, 2px)',
            boxShadow: '[2px 2px 0 0 #000]',
          },
        }),
      )}
      href={page.data.startUrl}
    >
      시작하기
      <Icon
        style={css.raw({
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _groupHover: {
            transform: 'translateX(4px) rotate(-15deg)',
          },
        })}
        icon={ArrowRightIcon}
      />
    </a>
  </div>

  <div class={flex({ alignItems: 'center', gap: '8px', display: { sm: 'flex', lg: 'none' } })}>
    <button
      class={css({
        padding: '8px',
        backgroundColor: 'gray.900',
        color: 'white',
        border: '2px solid',
        borderColor: 'gray.900',
        transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
      })}
      onclick={() => (mobileMenuOpen = !mobileMenuOpen)}
      type="button"
    >
      <Icon icon={MenuIcon} />
    </button>
  </div>
</header>

{#if mobileMenuOpen}
  <div
    class={css({
      position: 'fixed',
      top: '88px',
      left: '8px',
      right: '8px',
      backgroundColor: 'white',
      border: '4px solid',
      borderColor: 'gray.900',
      boxShadow: '[8px 8px 0 0 #000]',
      zIndex: '40',
      display: { sm: 'block', lg: 'none' },
      paddingY: '16px',
    })}
    onoutsideclick={() => (mobileMenuOpen = false)}
    use:outsideClick
  >
    <a
      class={css({
        display: 'block',
        fontSize: '16px',
        fontWeight: 'bold',
        color: 'gray.900',
        textTransform: 'uppercase',
        letterSpacing: '[0.05em]',
        paddingY: '12px',
        paddingX: '24px',
        transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
      })}
      href="/"
    >
      소개
    </a>

    <a
      class={css({
        display: 'block',
        fontSize: '16px',
        fontWeight: 'bold',
        color: 'gray.900',
        textTransform: 'uppercase',
        letterSpacing: '[0.05em]',
        paddingY: '12px',
        paddingX: '24px',
        transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
      })}
      href="/pricing"
    >
      구독 안내
    </a>

    <a
      class={css({
        display: 'block',
        fontSize: '16px',
        fontWeight: 'bold',
        color: 'gray.900',
        textTransform: 'uppercase',
        letterSpacing: '[0.05em]',
        paddingY: '12px',
        paddingX: '24px',
        transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
      })}
      href="/changelog"
    >
      업데이트 노트
    </a>

    <a
      class={cx(
        'group',
        css({
          display: 'block',
          marginX: '24px',
          marginTop: '16px',
          paddingX: '24px',
          paddingY: '12px',
          backgroundColor: 'gray.900',
          color: 'white',
          fontSize: '16px',
          fontWeight: 'black',
          textTransform: 'uppercase',
          letterSpacing: '[0.05em]',
          border: '4px solid',
          borderColor: 'gray.900',
          textAlign: 'center',
          boxShadow: '[4px 4px 0 0 #000]',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
        }),
      )}
      href={page.data.startUrl}
    >
      시작하기
    </a>
  </div>
{/if}
