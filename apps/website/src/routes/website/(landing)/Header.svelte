<script lang="ts">
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import Wordmark from '$assets/logos/wordmark.svg?component';
  import { env } from '$env/dynamic/public';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
</script>

<header
  class={flex({
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingX: '24px',
    width: 'full',
    height: '80px',
    gap: '32px',
    backgroundColor: 'white',
    borderBottom: '4px solid',
    borderColor: 'gray.900',
    position: 'relative',
    zIndex: '10',
  })}
>
  <a
    class={css({
      position: 'relative',
      transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
      _hover: {
        transform: 'translateY(-2px) rotate(-1deg)',
      },
    })}
    href="/landing"
  >
    <Wordmark class={css({ height: '28px', color: 'gray.900' })} />
  </a>

  <div class={flex({ alignItems: 'center', gap: '24px' })}>
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
      요금제
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
      href={env.PUBLIC_AUTH_URL}
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
</header>
