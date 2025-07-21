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
    height: '64px',
    gap: '32px',
  })}
>
  <a href="/landing">
    <Wordmark class={css({ height: '24px', color: 'gray.900' })} />
  </a>

  <div class={flex({ alignItems: 'center', gap: '32px' })}>
    <a
      class={css({
        fontSize: '15px',
        fontWeight: 'medium',
        color: 'gray.700',
        transition: 'common',
        marginLeft: 'auto',
        _hover: {
          color: 'gray.900',
        },
      })}
      href="/pricing"
    >
      요금제
    </a>

    <a
      class={css({
        fontSize: '15px',
        fontWeight: 'medium',
        color: 'gray.700',
        transition: 'common',
        marginLeft: 'auto',
        _hover: {
          color: 'gray.900',
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
          paddingX: '20px',
          paddingY: '8px',
          borderRadius: 'full',
          backgroundColor: 'gray.950',
          color: 'gray.50',
          fontSize: '15px',
          fontWeight: 'bold',
          transition: 'common',
          _hover: {
            backgroundColor: 'gray.800',
          },
        }),
      )}
      href={env.PUBLIC_AUTH_URL}
    >
      시작하기
      <Icon style={css.raw({ _groupHover: { transform: 'translateX(2px)' } })} icon={ArrowRightIcon} />
    </a>
  </div>
</header>
