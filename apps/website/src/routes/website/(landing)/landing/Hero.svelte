<script lang="ts">
  import { onMount } from 'svelte';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import SparklesIcon from '~icons/lucide/sparkles';
  import ZapIcon from '~icons/lucide/zap';
  import { env } from '$env/dynamic/public';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import heroImage from './images/hero.png';

  let element = $state<HTMLElement>();

  onMount(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add('in-view');
          }
        });
      },
      {
        threshold: 0.05,
        rootMargin: '0px 0px 100px 0px',
      },
    );

    if (element) observer.observe(element);

    return () => {
      if (element) observer.unobserve(element);
    };
  });
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
      width: '[600px]',
      height: '[600px]',
      backgroundImage: 'radial-gradient(circle, token(colors.amber.300), transparent)',
      opacity: '[0.4]',
      filter: '[blur(180px)]',
      pointerEvents: 'none',
      zIndex: '1',
    })}
  ></div>

  <div
    class={css({
      position: 'absolute',
      bottom: '[10%]',
      left: '[-5%]',
      width: '[500px]',
      height: '[500px]',
      backgroundImage: 'radial-gradient(circle, token(colors.gray.300), transparent)',
      opacity: '[0.5]',
      filter: '[blur(150px)]',
      pointerEvents: 'none',
      zIndex: '1',
    })}
  ></div>

  <section
    bind:this={element}
    class={css({
      position: 'relative',
      paddingTop: { sm: '80px', lg: '120px' },
      paddingBottom: { sm: '60px', lg: '100px' },
      paddingX: { sm: '16px', lg: '24px' },
      zIndex: '2',
      opacity: '0',
      transform: { sm: 'translateY(40px) scale(0.95)', lg: 'translateY(40px) rotate(-1deg) scale(0.95)' },
      transition: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
      '&.in-view': {
        opacity: '100',
        transform: { sm: 'translateY(0) scale(1)', lg: 'translateY(0) rotate(0) scale(1)' },
      },
    })}
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
          transform: 'rotate(-2deg) scale(0)',
          transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.3s]',
          '.in-view &': {
            transform: 'rotate(-2deg) scale(1)',
          },
        })}
      >
        <Icon icon={SparklesIcon} size={16} />
        NEW EDITOR EXPERIENCE
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
          transform: 'translateY(20px)',
          transition: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s]',
          '.in-view &': {
            opacity: '100',
            transform: 'translateY(0)',
          },
        })}
      >
        글쓰기의
        <br />
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: { sm: '16px', lg: '24px' },
            display: 'inline-block',
            transform: 'rotate(1deg) scale(0)',
            transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.6s]',
            '.in-view &': {
              transform: 'rotate(1deg) scale(1)',
            },
          })}
        >
          모든 과정
        </span>
        <br />
        하나로 완성
      </h1>

      <p
        class={css({
          fontSize: { sm: '16px', lg: '24px' },
          fontWeight: 'medium',
          color: 'gray.700',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          marginBottom: { sm: '40px', lg: '64px' },
          lineHeight: '[1.6]',
          maxWidth: '[800px]',
          marginX: 'auto',
          opacity: '0',
          transform: 'translateY(10px)',
          transition: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.7s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.7s]',
          '.in-view &': {
            opacity: '100',
            transform: 'translateY(0)',
          },
        })}
      >
        쓰는 것이 즐거워지는 새로운 편집 경험.
        <br />
        흩어지고 끊기던 글쓰기의 흐름이, 이제는 자연스럽게 이어집니다.
      </p>

      <div
        class={css({
          display: 'flex',
          gap: { sm: '16px', lg: '20px' },
          justifyContent: 'center',
          flexWrap: 'wrap',
          marginBottom: { sm: '60px', lg: '80px' },
          opacity: '0',
          transform: 'translateY(10px)',
          transition: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.8s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.8s]',
          '.in-view &': {
            opacity: '100',
            transform: 'translateY(0)',
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
              transform: { sm: 'rotate(0)', lg: 'rotate(-1deg)' },
              textTransform: 'uppercase',
              letterSpacing: '[0.05em]',
              boxShadow: '[6px 6px 0 0 #000]',
              _hover: {
                transform: { sm: 'translate(-2px, -2px)', lg: 'translate(-2px, -2px) rotate(0)' },
                boxShadow: '[8px 8px 0 0 #000]',
              },
              _active: {
                transform: 'translate(2px, 2px)',
                boxShadow: '[4px 4px 0 0 #000]',
              },
            }),
          )}
          href={env.PUBLIC_AUTH_URL}
        >
          <Icon icon={ZapIcon} size={20} />
          무료로 시작하기
          <Icon
            style={css.raw({
              transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              _groupHover: {
                transform: 'translateX(4px) rotate(-15deg)',
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
          transform: { sm: 'translateY(20px)', lg: 'translateY(20px) rotate(-0.5deg)' },
          transition: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.9s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.9s]',
          '.in-view &': {
            opacity: '100',
            transform: { sm: 'translateY(0)', lg: 'translateY(0) rotate(0)' },
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
            transform: 'rotate(-3deg)',
            boxShadow: '[4px 4px 0 0 #000]',
            zIndex: '10',
          })}
        >
          LIVE DEMO
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
              transform: { sm: 'translate(-2px, -2px)', lg: 'translate(-4px, -4px)' },
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
