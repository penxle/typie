<script lang="ts">
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import PenLineIcon from '~icons/lucide/pen-line';
  import ZapIcon from '~icons/lucide/zap';
  import { page } from '$app/state';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import heroImage from './images/hero.webp';
</script>

<div
  class={css({
    position: 'relative',
    minHeight: '[100vh]',
    overflow: 'hidden',
    backgroundColor: 'white',
  })}
>
  <div
    class={css({
      position: 'absolute',
      inset: '0',
      backgroundImage:
        'linear-gradient(to bottom, token(colors.white), token(colors.gray.50) 25%, token(colors.gray.50) 75%, token(colors.white))',
      zIndex: '0',
    })}
  ></div>

  <div
    class={css({
      position: 'absolute',
      inset: '0',
      backgroundImage: 'radial-gradient(circle at 1px 1px, token(colors.gray.200) 1px, transparent 1px)',
      backgroundSize: '[50px 50px]',
      opacity: '[0.3]',
      pointerEvents: 'none',
      zIndex: '1',
    })}
  ></div>

  <div
    class={css({
      position: 'absolute',
      top: '[20%]',
      right: '[10%]',
      width: { sm: '[300px]', lg: '[600px]' },
      height: { sm: '[300px]', lg: '[600px]' },
      backgroundImage: 'radial-gradient(circle, token(colors.amber.300), transparent)',
      opacity: { sm: '[0.2]', lg: '[0.4]' },
      filter: { sm: '[blur(80px)]', lg: '[blur(180px)]' },
      pointerEvents: 'none',
      zIndex: '1',
      willChange: 'transform',
      display: { sm: 'none', md: 'block' },
    })}
  ></div>

  <div
    class={css({
      position: 'absolute',
      bottom: '[10%]',
      left: '[-5%]',
      width: { sm: '[250px]', lg: '[500px]' },
      height: { sm: '[250px]', lg: '[500px]' },
      backgroundImage: 'radial-gradient(circle, token(colors.gray.300), transparent)',
      opacity: { sm: '[0.3]', lg: '[0.5]' },
      filter: { sm: '[blur(60px)]', lg: '[blur(150px)]' },
      pointerEvents: 'none',
      zIndex: '1',
      willChange: 'transform',
      display: { sm: 'none', md: 'block' },
    })}
  ></div>

  <section
    class={css({
      position: 'relative',
      paddingTop: { sm: '80px', lg: '120px' },
      paddingBottom: { sm: '60px', lg: '100px' },
      paddingX: { sm: '16px', lg: '24px' },
      zIndex: '2',
      opacity: '0',
      transform: { sm: 'translate3d(0, 20px, 0)', lg: 'translate3d(0, 40px, 0) rotate(-1deg) scale(0.95)' },
      transition: {
        sm: '[opacity 0.4s ease-out, transform 0.4s ease-out]',
        lg: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
      },
      willChange: 'transform, opacity',
      '&.in-view': {
        opacity: '100',
        transform: { sm: 'translate3d(0, 0, 0)', lg: 'translate3d(0, 0, 0) rotate(0) scale(1)' },
      },
    })}
    data-observe
  >
    <div class={center({ flexDirection: 'column', maxWidth: '[1200px]', marginX: 'auto' })}>
      <div
        class={css({
          display: 'inline-flex',
          alignItems: 'center',
          gap: '8px',
          backgroundColor: 'gray.900',
          color: 'white',
          paddingX: { sm: '16px', lg: '20px' },
          paddingY: { sm: '6px', lg: '8px' },
          fontSize: { sm: '12px', lg: '14px' },
          fontWeight: 'bold',
          marginBottom: { sm: '32px', lg: '48px' },
          letterSpacing: '[0.1em]',
          textTransform: 'uppercase',
          transform: { sm: 'scale(0)', lg: 'rotate(-2deg) scale(0)' },
          transition: {
            sm: '[transform 0.3s ease-out 0.2s]',
            lg: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.3s]',
          },
          willChange: 'transform',
          '.in-view &': {
            transform: { sm: 'scale(1)', lg: 'rotate(-2deg) scale(1)' },
          },
        })}
      >
        <Icon icon={PenLineIcon} size={16} />
        Built for Focused Writing
      </div>

      <h1
        class={css({
          fontSize: { sm: '[48px]', lg: '[112px]' },
          fontWeight: 'black',
          color: 'gray.900',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          lineHeight: '[1]',
          marginBottom: { sm: '24px', lg: '40px' },
          textTransform: 'uppercase',
          opacity: '0',
          transform: 'translate3d(0, 20px, 0)',
          transition: {
            sm: '[opacity 0.4s ease-out 0.3s, transform 0.4s ease-out 0.3s]',
            lg: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s]',
          },
          willChange: 'transform, opacity',
          '.in-view &': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
      >
        작가를 위한
        <br />
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: { sm: '16px', lg: '24px' },
            display: 'inline-block',
            transform: { sm: 'scale(0)', lg: 'rotate(1deg) scale(0)' },
            transition: {
              sm: '[transform 0.3s ease-out 0.4s]',
              lg: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.6s]',
            },
            willChange: 'transform',
            '.in-view &': {
              transform: { sm: 'scale(1)', lg: 'rotate(1deg) scale(1)' },
            },
          })}
        >
          올인원 글쓰기 도구
        </span>
      </h1>

      <p
        class={css({
          fontSize: { sm: '16px', lg: '24px' },
          fontWeight: 'medium',
          color: 'gray.700',
          textAlign: 'center',
          marginBottom: { sm: '40px', lg: '64px' },
          lineHeight: '[1.6]',
          maxWidth: '[800px]',
          marginX: 'auto',
          opacity: '0',
          transform: 'translate3d(0, 10px, 0)',
          transition: {
            sm: '[opacity 0.4s ease-out 0.5s, transform 0.4s ease-out 0.5s]',
            lg: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.7s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.7s]',
          },
          willChange: 'transform, opacity',
          '.in-view &': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
      >
        원고 작성부터 자료 정리, 실시간 협업과 공유까지.
        <br />
        글쓰기에 필요한 모든 기능을 한곳에 담았어요.
      </p>

      <div
        class={css({
          display: 'flex',
          gap: { sm: '16px', lg: '20px' },
          justifyContent: 'center',
          flexWrap: 'wrap',
          marginBottom: { sm: '60px', lg: '80px' },
          opacity: '0',
          transform: 'translate3d(0, 10px, 0)',
          transition: {
            sm: '[opacity 0.4s ease-out 0.6s, transform 0.4s ease-out 0.6s]',
            lg: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.8s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.8s]',
          },
          willChange: 'transform, opacity',
          '.in-view &': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
      >
        <a
          class={cx(
            'group',
            css({
              display: 'inline-flex',
              alignItems: 'center',
              gap: { sm: '8px', lg: '10px' },
              paddingX: { sm: '24px', lg: '32px' },
              paddingY: { sm: '12px', lg: '16px' },
              fontSize: { sm: '16px', lg: '18px' },
              fontWeight: 'black',
              color: 'gray.900',
              backgroundColor: 'amber.400',
              border: '4px solid',
              borderColor: 'gray.900',
              transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              transform: { sm: 'translate3d(0, 0, 0)', lg: 'rotate(-1deg)' },
              textTransform: 'uppercase',
              letterSpacing: '[0.05em]',
              boxShadow: '[6px 6px 0 0 #000]',
              _hover: {
                transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-2px, -2px, 0) rotate(0)' },
                boxShadow: '[8px 8px 0 0 #000]',
              },
              _active: {
                transform: 'translate3d(2px, 2px, 0)',
                boxShadow: '[4px 4px 0 0 #000]',
              },
            }),
          )}
          href={page.data.startUrl}
        >
          <Icon icon={ZapIcon} size={20} />
          지금 시작하기
          <Icon
            style={css.raw({
              transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              _groupHover: {
                transform: { sm: 'translate3d(4px, 0, 0)', lg: 'translate3d(4px, 0, 0) rotate(-15deg)' },
              },
            })}
            icon={ArrowRightIcon}
            size={20}
          />
        </a>
      </div>

      <div
        class={css({
          position: 'relative',
          maxWidth: '[1024px]',
          marginX: 'auto',
          width: 'full',
          opacity: '0',
          transform: { sm: 'translate3d(0, 20px, 0)', lg: 'translate3d(0, 20px, 0) rotate(-0.5deg)' },
          transition: {
            sm: '[opacity 0.4s ease-out 0.7s, transform 0.4s ease-out 0.7s]',
            lg: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.9s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.9s]',
          },
          willChange: 'transform, opacity',
          '.in-view &': {
            opacity: '100',
            transform: { sm: 'translate3d(0, 0, 0)', lg: 'translate3d(0, 0, 0) rotate(0)' },
          },
        })}
      >
        <div
          class={css({
            position: 'absolute',
            top: { sm: '-16px', lg: '-20px' },
            left: { sm: '16px', lg: '20px' },
            backgroundColor: 'amber.400',
            color: 'gray.900',
            paddingX: { sm: '16px', lg: '20px' },
            paddingY: { sm: '8px', lg: '10px' },
            fontSize: { sm: '12px', lg: '14px' },
            fontWeight: 'black',
            textTransform: 'uppercase',
            letterSpacing: '[0.1em]',
            border: '4px solid',
            borderColor: 'gray.900',
            transform: { sm: 'rotate(0)', lg: 'rotate(-3deg)' },
            boxShadow: '[4px 4px 0 0 #000]',
            zIndex: '10',
          })}
        >
          PREVIEW
        </div>

        <div
          class={css({
            backgroundColor: 'white',
            padding: { sm: '4px', lg: '8px' },
            border: '4px solid',
            borderColor: 'gray.900',
            boxShadow: { sm: '[8px 8px 0 0 #000]', lg: '[12px 12px 0 0 #000]' },
            transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
            _hover: {
              transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0)' },
              boxShadow: { sm: '[10px 10px 0 0 #000]', lg: '[16px 16px 0 0 #000]' },
            },
          })}
        >
          <img
            class={css({
              width: 'full',
              height: 'auto',
              display: 'block',
              objectFit: 'cover',
            })}
            alt="타이피 에디터 미리보기"
            src={heroImage}
          />
        </div>
      </div>
    </div>
  </section>
</div>
