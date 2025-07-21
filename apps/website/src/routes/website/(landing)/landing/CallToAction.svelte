<script lang="ts">
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import { env } from '$env/dynamic/public';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
</script>

<section
  class={css({
    position: 'relative',
    paddingBottom: '160px',
    backgroundColor: 'gray.50',
  })}
>
  <div
    class={css({
      maxWidth: '[1024px]',
      marginX: 'auto',
      backgroundColor: 'white',
      borderRadius: '[24px]',
      padding: '48px',
      borderWidth: '1px',
      borderColor: 'gray.200',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: '60px',
    })}
  >
    <div class={css({ flex: '1' })}>
      <h2
        class={css({
          fontSize: '[36px]',
          fontWeight: 'bold',
          color: 'gray.950',
          fontFamily: 'Paperlogy',
          lineHeight: '[1.3]',
          marginBottom: '16px',
        })}
      >
        지금 바로 시작해 보세요
      </h2>
      <p
        class={css({
          fontSize: '18px',
          fontWeight: 'medium',
          color: 'gray.600',
          fontFamily: 'Pretendard',
          lineHeight: '[1.6]',
        })}
      >
        복잡한 설치 없이, 몇 번의 클릭만으로 새로운 쓰기 경험을 시작할 수 있습니다.
      </p>
    </div>

    <a
      class={cx(
        'group',
        css({
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          paddingX: '24px',
          paddingY: '12px',
          fontSize: '16px',
          fontWeight: 'semibold',
          color: 'white',
          backgroundColor: 'gray.950',
          borderRadius: 'full',
          transition: 'common',
          cursor: 'pointer',
          border: 'none',
          flexShrink: '0',
          _hover: {
            backgroundColor: 'gray.800',
          },
        }),
      )}
      href={env.PUBLIC_AUTH_URL}
      type="button"
    >
      시작하기
      <Icon style={css.raw({ _groupHover: { transform: 'translateX(2px)' } })} icon={ArrowRightIcon} size={16} />
    </a>
  </div>
</section>
