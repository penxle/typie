<script lang="ts">
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import SparklesIcon from '~icons/lucide/sparkles';
  import { page } from '$app/state';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
</script>

<section
  class={css({
    position: 'relative',
    paddingY: { sm: '80px', lg: '120px' },
    backgroundColor: 'gray.50',
  })}
>
  <div class={css({ paddingX: { sm: '16px', lg: '40px' }, maxWidth: '[1024px]', marginX: 'auto' })}>
    <div
      class={css({
        position: 'relative',
        backgroundColor: 'amber.400',
        border: '4px solid',
        borderColor: 'gray.900',
        boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
        padding: { sm: '40px', lg: '60px' },
        transform: { sm: 'translate3d(0, 0, 0) scale(0.95)', lg: 'rotate(-1deg) scale(0.95)' },
        opacity: '0',
        transition: {
          sm: '[transform 0.4s ease-out, opacity 0.4s ease-out, box-shadow 0.3s ease-out]',
          lg: '[transform 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94), opacity 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
        },
        willChange: 'transform, opacity',
        '&.in-view': {
          opacity: '100',
          transform: { sm: 'translate3d(0, 0, 0) scale(1)', lg: 'rotate(-1deg) scale(1)' },
        },
        _hover: {
          transform: { sm: 'translate3d(0, 0, 0) scale(1)', lg: 'rotate(0deg) scale(1)' },
          boxShadow: { sm: '[12px 12px 0 0 #000]', lg: '[16px 16px 0 0 #000]' },
        },
      })}
      data-observe
    >
      <div
        class={css({
          position: 'absolute',
          top: { sm: '-16px', lg: '-20px' },
          right: { sm: '40px', lg: '60px' },
          backgroundColor: 'gray.900',
          padding: { sm: '8px', lg: '12px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[4px 4px 0 0 #fff]',
          transform: { sm: 'scale(0)', lg: 'rotate(12deg) scale(0)' },
          opacity: '0',
          transition: {
            sm: '[transform 0.3s ease-out 0.2s, opacity 0.3s ease-out 0.2s]',
            lg: '[transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s, opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s]',
          },
          willChange: 'transform',
          '&.in-view': {
            opacity: '100',
            transform: { sm: 'scale(1)', lg: 'rotate(12deg) scale(1)' },
          },
        })}
        data-observe
      >
        <Icon style={css.raw({ color: 'white' })} icon={SparklesIcon} size={24} />
      </div>

      <div class={center({ flexDirection: 'column', textAlign: 'center' })}>
        <h2
          class={css({
            fontSize: { sm: '[36px]', lg: '[56px]' },
            fontWeight: 'black',
            color: 'gray.900',
            fontFamily: 'Paperlogy',
            lineHeight: '[1.1]',
            marginBottom: { sm: '16px', lg: '24px' },
            textTransform: 'uppercase',
          })}
        >
          글쓰기,
          <br />
          <span
            class={css({
              backgroundColor: 'white',
              paddingX: { sm: '16px', lg: '24px' },
              paddingY: { sm: '6px', lg: '8px' },
              display: 'inline-block',
              transform: { sm: 'rotate(0)', lg: 'rotate(-2deg)' },
              border: '4px solid',
              borderColor: 'gray.900',
              boxShadow: '[6px 6px 0 0 #000]',
              marginTop: { sm: '4px', lg: '8px' },
            })}
          >
            지금 시작하세요
          </span>
        </h2>
        <p
          class={css({
            fontSize: { sm: '16px', lg: '20px' },
            fontWeight: 'semibold',
            color: 'gray.700',
            lineHeight: '[1.6]',
            marginBottom: { sm: '32px', lg: '40px' },
            maxWidth: '[800px]',
            textAlign: 'center',
            marginX: 'auto',
          })}
        >
          별다른 준비 없이도 바로 시작할 수 있어요.
          <br />
          지금 가장 편한 글쓰기 경험을 만나보세요.
        </p>

        <a
          class={cx(
            'group',
            css({
              display: 'inline-flex',
              alignItems: 'center',
              gap: { sm: '8px', lg: '12px' },
              paddingX: { sm: '24px', lg: '32px' },
              paddingY: { sm: '12px', lg: '16px' },
              fontSize: { sm: '16px', lg: '18px' },
              fontWeight: 'black',
              color: 'white',
              backgroundColor: 'gray.900',
              cursor: 'pointer',
              textTransform: 'uppercase',
              letterSpacing: '[0.05em]',
              border: '4px solid',
              borderColor: 'gray.900',
              boxShadow: '[6px 6px 0 0 #fff]',
              transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              _hover: {
                transform: 'translate3d(-4px, -4px, 0)',
                boxShadow: '[10px 10px 0 0 #fff]',
              },
            }),
          )}
          href={page.data.startUrl}
        >
          시작하기
          <Icon
            style={css.raw({
              transition: 'transform',
              transitionDuration: '200ms',
              _groupHover: {
                transform: 'translate3d(4px, 0, 0)',
              },
            })}
            icon={ArrowRightIcon}
            size={20}
          />
        </a>
      </div>

      <div
        class={css({
          position: 'absolute',
          bottom: { sm: '24px', lg: '40px' },
          left: { sm: '24px', lg: '40px' },
          width: { sm: '60px', lg: '80px' },
          height: { sm: '60px', lg: '80px' },
          backgroundColor: 'white',
          border: '4px solid',
          borderColor: 'gray.900',
          transform: { sm: 'scale(0)', lg: 'rotate(45deg) scale(0)' },
          opacity: '0',
          boxShadow: '[4px 4px 0 0 #000]',
          transition: {
            sm: '[transform 0.3s ease-out 0.3s, opacity 0.3s ease-out 0.3s]',
            lg: '[transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s, opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s]',
          },
          willChange: 'transform',
          '&.in-view': {
            opacity: '100',
            transform: { sm: 'scale(1)', lg: 'rotate(45deg) scale(1)' },
          },
        })}
        data-observe
      ></div>

      <div
        class={css({
          position: 'absolute',
          top: { sm: '24px', lg: '40px' },
          left: { sm: '40px', lg: '60px' },
          width: { sm: '40px', lg: '60px' },
          height: { sm: '40px', lg: '60px' },
          backgroundColor: 'gray.900',
          transform: { sm: 'scale(0)', lg: 'rotate(-15deg) scale(0)' },
          opacity: '0',
          transition: {
            sm: '[transform 0.3s ease-out 0.4s, opacity 0.3s ease-out 0.4s]',
            lg: '[transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.5s, opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.5s]',
          },
          willChange: 'transform',
          '&.in-view': {
            opacity: '100',
            transform: { sm: 'scale(1)', lg: 'rotate(-15deg) scale(1)' },
          },
        })}
        data-observe
      ></div>
    </div>
  </div>
</section>
