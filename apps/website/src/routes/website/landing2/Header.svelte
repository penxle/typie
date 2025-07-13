<script lang="ts">
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import Wordmark from '$assets/logos/wordmark.svg?component';
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
  <Wordmark class={css({ height: '24px', color: 'gray.900' })} />

  <button
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
    type="button"
  >
    요금제
  </button>

  <button
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
    type="button"
  >
    시작하기
    <Icon style={css.raw({ _groupHover: { transform: 'translateX(2px)' } })} icon={ArrowRightIcon} />
  </button>
</header>
