<script lang="ts">
  import { animate } from 'motion';
  import { onMount } from 'svelte';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import { env } from '$env/dynamic/public';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  let status = $state<'relative' | 'relative-to-fixed' | 'fixed' | 'fixed-to-relative'>('relative');
  let observed = false;

  let containerEl = $state<HTMLElement>();

  onMount(() => {
    if (!containerEl) return;

    let animation = animate([
      ['[data-element="actionbar-login"]', { color: '#ffba00' }, { duration: 0.6, ease: 'linear' }],
      ['[data-element="actionbar"]', { width: '350px' }, { ease: 'easeInOut', duration: 0.3, at: '<' }],
      ['[data-element="actionbar-item"]', { opacity: 1, display: 'flex', duration: 0.3 }],
    ]);

    animation.cancel();

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach(async (entry) => {
          if (!observed) {
            if (entry.isIntersecting) {
              status = 'relative';
              animation.cancel();
            } else {
              status = 'fixed';
              animation.complete();
            }

            observed = true;
            return;
          }

          if (entry.isIntersecting) {
            status = 'fixed-to-relative';
            animation.speed = -1;
            animation.play();
            await animation;
            status = 'relative';
          } else {
            status = 'relative-to-fixed';
            animation.speed = 1;
            animation.play();
            await animation;
            status = 'fixed';
          }
        });
      },
      { threshold: 1 },
    );

    observer.observe(containerEl);

    return () => {
      observer.disconnect();
      animation.stop();
    };
  });
</script>

<div bind:this={containerEl} class={css({ position: 'relative', height: '50px', width: '1px', top: '-20px', zIndex: '50' })}>
  <div
    class={cx(
      'group',
      flex({
        top: '20px',
        left: '1/2',
        transform: 'translateX(-50%)',
        gap: '16px',
        flexDirection: 'row-reverse',
        zIndex: '50',
        borderRadius: 'full',
        backgroundColor: 'gray.900',
        alignItems: 'center',
        fontSize: '16px',
        fontWeight: 'bold',
        userSelect: 'none',
        boxShadow: '[0 20px 40px -10px rgba(0, 0, 0, 0.3), 0 10px 20px -5px rgba(0, 0, 0, 0.1)]',
        '&[data-floating="fixed"]': {
          position: 'fixed',
          justifyContent: 'space-between',
          paddingX: '36px',
          paddingY: '6px',
        },
        '&[data-floating="fixed-to-relative"]': {
          position: 'absolute',
          justifyContent: 'space-between',
          paddingX: '36px',
          paddingY: '6px',
        },
        '&[data-floating="relative-to-fixed"]': {
          position: 'fixed',
          justifyContent: 'space-between',
          paddingX: '36px',
          paddingY: '6px',
        },
        '&[data-floating="relative"]': {
          position: 'absolute',
          justifyContent: 'center',
          width: 'max!',
          cursor: 'pointer',
          transitionProperty: 'box-shadow',
          transitionDuration: '350ms',
          transitionTimingFunction: 'ease',
          _hover: {
            boxShadow: '[0 25px 50px -12px rgba(0, 0, 0, 0.35), 0 15px 25px -7px rgba(0, 0, 0, 0.2)]',
          },
        },
      }),
    )}
    data-element="actionbar"
    data-floating={status}
  >
    <a
      class={cx(
        'group',
        center({
          gap: '8px',
          paddingX: '36px',
          color: '[#fafafa]',
          '[data-floating="relative"] &': {
            paddingY: '12px',
            transitionProperty: 'gap, padding-inline',
            transitionDuration: '350ms',
            transitionTimingFunction: 'ease',
          },
          '[data-floating="relative"]:hover &': {
            paddingX: '40px',
            gap: '10px',
          },
          '[data-floating="fixed-to-relative"] &': {
            paddingY: '6px',
            marginX: '-36px',
          },
          '[data-floating="relative-to-fixed"] &': {
            paddingY: '6px',
            marginX: '-36px',
          },
          '[data-floating="fixed"] &': {
            paddingY: '6px',
            marginX: '0',
            paddingX: '0',
          },
        }),
      )}
      data-element="actionbar-login"
      href={env.PUBLIC_AUTH_URL}
    >
      시작하기

      <Icon
        style={css.raw({
          transitionProperty: 'transform',
          '[data-floating="fixed"] .group:hover &': { transform: 'translateX(2px)' },
        })}
        icon={ArrowRightIcon}
      />
    </a>

    <a class={css({ color: 'gray.500', display: 'none', opacity: '0', paddingY: '6px' })} data-element="actionbar-item" href="/pricing">
      요금제
    </a>

    <div class={center({ display: 'none', opacity: '0', gap: '8px' })} data-element="actionbar-item">
      <div class={css({ size: '8px', backgroundColor: 'amber.400', borderRadius: 'full' })}></div>
      <div class={css({ color: 'gray.50', paddingY: '6px' })}>소개</div>
    </div>
  </div>
</div>
