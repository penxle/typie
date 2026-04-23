<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import { inview } from './inview';
</script>

<section
  class={css({
    position: 'relative',
    paddingX: { sm: '24px', lg: '80px' },
    paddingY: { sm: '80px', lg: '120px' },
    backgroundColor: 'dark.gray.950',
    borderTopWidth: '1px',
    borderTopColor: 'dark.gray.900',
  })}
>
  <div
    class={css({
      position: 'absolute',
      left: { sm: '16px', lg: '48px' },
      top: '0',
      bottom: '0',
      width: '1px',
      backgroundColor: 'dark.gray.800',
      display: { sm: 'none', lg: 'block' },
    })}
  ></div>

  <div
    class={css({
      maxWidth: '[1200px]',
      marginX: 'auto',
      opacity: '0',
      transform: 'translate3d(0, 28px, 0)',
      transition: '[opacity 0.8s cubic-bezier(0.16, 1, 0.3, 1), transform 0.8s cubic-bezier(0.16, 1, 0.3, 1)]',
      '&.in-view': {
        opacity: '100',
        transform: 'translate3d(0, 0, 0)',
      },
    })}
    {@attach inview}
  >
    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: { sm: '1fr', lg: '[1fr auto]' },
        gap: { sm: '32px', lg: '80px' },
        alignItems: 'end',
        paddingBottom: { sm: '48px', lg: '64px' },
      })}
    >
      <div>
        <span
          class={css({
            display: 'block',
            fontSize: '[11px]',
            fontFamily: 'mono',
            color: 'dark.gray.500',
            letterSpacing: '[0.1em]',
            textTransform: 'uppercase',
            marginBottom: '24px',
          })}
        >
          Get Started
        </span>

        <h2
          class={css({
            fontSize: { sm: '[32px]', lg: '[48px]' },
            fontWeight: 'medium',
            color: 'dark.gray.100',
            lineHeight: '[1.2]',
            letterSpacing: '[-0.02em]',
            fontFamily: 'Paperlogy',
            marginBottom: '20px',
          })}
        >
          지금 시작하면
          <br />
          오늘부터 쓸 수 있어요.
        </h2>

        <p
          class={css({
            fontSize: { sm: '16px', lg: '18px' },
            color: 'dark.gray.400',
            lineHeight: '[1.65]',
            maxWidth: '[400px]',
          })}
        >
          쓰고 싶었던 이야기, 여기서 시작하세요.
        </p>
      </div>

      <a
        class={cx(
          'group',
          css({
            display: 'inline-flex',
            alignItems: 'center',
            gap: '12px',
            paddingX: '32px',
            paddingY: '18px',
            fontSize: '16px',
            fontWeight: 'semibold',
            color: 'dark.gray.950',
            backgroundColor: 'dark.brand.300',
            transition: '[all 0.2s ease-out]',
            _hover: {
              backgroundColor: 'dark.brand.200',
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
              transform: 'translateX(4px)',
            },
          })}
          icon={ArrowRightIcon}
          size={18}
        />
      </a>
    </div>
  </div>
</section>
