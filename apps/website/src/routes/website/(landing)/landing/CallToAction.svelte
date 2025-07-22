<script lang="ts">
  import { onMount } from 'svelte';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import SparklesIcon from '~icons/lucide/sparkles';
  import { env } from '$env/dynamic/public';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center } from '$styled-system/patterns';

  let boxElement = $state<HTMLElement>();
  let sparkleElement = $state<HTMLElement>();
  let decorElement1 = $state<HTMLElement>();
  let decorElement2 = $state<HTMLElement>();

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
        threshold: 0.1,
        rootMargin: '0px 0px 50px 0px',
      },
    );

    const elements = [boxElement, sparkleElement, decorElement1, decorElement2];
    elements.forEach((element) => {
      if (element) observer.observe(element);
    });

    return () => {
      elements.forEach((element) => {
        if (element) observer.unobserve(element);
      });
    };
  });
</script>

<section
  class={css({
    position: 'relative',
    paddingY: '120px',
    backgroundColor: 'gray.50',
  })}
>
  <div class={css({ paddingX: '40px', maxWidth: '[1024px]', marginX: 'auto' })}>
    <div
      bind:this={boxElement}
      class={css({
        position: 'relative',
        backgroundColor: 'amber.400',
        border: '4px solid',
        borderColor: 'gray.900',
        boxShadow: '[12px 12px 0 0 #000]',
        padding: '60px',
        transform: 'rotate(-1deg) scale(0.95)',
        opacity: '0',
        transition:
          '[transform 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94), opacity 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
        '&.in-view': {
          opacity: '100',
          transform: 'rotate(-1deg) scale(1)',
        },
        _hover: {
          transform: 'rotate(0deg) scale(1)',
          boxShadow: '[16px 16px 0 0 #000]',
        },
      })}
    >
      <div
        bind:this={sparkleElement}
        class={css({
          position: 'absolute',
          top: '-20px',
          right: '60px',
          backgroundColor: 'gray.900',
          padding: '12px',
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[4px 4px 0 0 #fff]',
          transform: 'rotate(12deg) scale(0)',
          opacity: '0',
          transition: '[transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s, opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s]',
          '&.in-view': {
            opacity: '100',
            transform: 'rotate(12deg) scale(1)',
          },
        })}
      >
        <Icon style={css.raw({ color: 'white' })} icon={SparklesIcon} size={24} />
      </div>

      <div class={center({ flexDirection: 'column', textAlign: 'center' })}>
        <h2
          class={css({
            fontSize: '[56px]',
            fontWeight: 'black',
            color: 'gray.950',
            fontFamily: 'Paperlogy',
            lineHeight: '[1.1]',
            marginBottom: '24px',
            textTransform: 'uppercase',
          })}
        >
          지금 바로
          <br />
          <span
            class={css({
              backgroundColor: 'white',
              paddingX: '24px',
              paddingY: '8px',
              display: 'inline-block',
              transform: 'rotate(-2deg)',
              border: '4px solid',
              borderColor: 'gray.900',
              boxShadow: '[6px 6px 0 0 #000]',
              marginTop: '8px',
            })}
          >
            시작하세요!
          </span>
        </h2>
        <p
          class={css({
            fontSize: '20px',
            fontWeight: 'bold',
            color: 'gray.900',
            fontFamily: 'Pretendard',
            lineHeight: '[1.6]',
            marginBottom: '40px',
            maxWidth: '600px',
          })}
        >
          복잡한 설치 없이, 몇 번의 클릭만으로
          <br />
          새로운 쓰기 경험을 시작할 수 있습니다.
        </p>

        <a
          class={cx(
            'group',
            css({
              display: 'inline-flex',
              alignItems: 'center',
              gap: '12px',
              paddingX: '32px',
              paddingY: '16px',
              fontSize: '18px',
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
                transform: 'translate(-4px, -4px)',
                boxShadow: '[10px 10px 0 0 #fff]',
              },
            }),
          )}
          href={env.PUBLIC_AUTH_URL}
        >
          시작하기
          <Icon
            style={css.raw({
              transition: 'transform',
              transitionDuration: '200ms',
              _groupHover: {
                transform: 'translateX(4px)',
              },
            })}
            icon={ArrowRightIcon}
            size={20}
          />
        </a>
      </div>

      <div
        bind:this={decorElement1}
        class={css({
          position: 'absolute',
          bottom: '40px',
          left: '40px',
          width: '80px',
          height: '80px',
          backgroundColor: 'white',
          border: '4px solid',
          borderColor: 'gray.900',
          transform: 'rotate(45deg) scale(0)',
          opacity: '0',
          boxShadow: '[4px 4px 0 0 #000]',
          transition: '[transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s, opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s]',
          '&.in-view': {
            opacity: '100',
            transform: 'rotate(45deg) scale(1)',
          },
        })}
      ></div>

      <div
        bind:this={decorElement2}
        class={css({
          position: 'absolute',
          top: '40px',
          left: '60px',
          width: '60px',
          height: '60px',
          backgroundColor: 'gray.900',
          transform: 'rotate(-15deg) scale(0)',
          opacity: '0',
          transition: '[transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.5s, opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.5s]',
          '&.in-view': {
            opacity: '100',
            transform: 'rotate(-15deg) scale(1)',
          },
        })}
      ></div>
    </div>
  </div>
</section>
