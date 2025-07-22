<script lang="ts">
  import { onMount } from 'svelte';
  import GoalIcon from '~icons/lucide/goal';
  import TrendingUpIcon from '~icons/lucide/trending-up';
  import TypeIcon from '~icons/lucide/type';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex, grid } from '$styled-system/patterns';

  let elements = $state<HTMLElement[]>([]);
  let heatmapElement = $state<HTMLElement>();

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

    if (heatmapElement) observer.observe(heatmapElement);
    elements.forEach((element) => {
      if (element) observer.observe(element);
    });

    return () => {
      if (heatmapElement) observer.unobserve(heatmapElement);
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
    borderBottom: '8px solid',
    borderColor: 'gray.900',
  })}
>
  <div class={css({ width: 'full', position: 'relative', marginBottom: '120px' })}>
    <div
      bind:this={heatmapElement}
      class={grid({
        gridTemplateRows: 'repeat(7, minmax(0, 1fr))',
        gridTemplateColumns: 'repeat(52, minmax(0, 1fr))',
        gridAutoFlow: 'column',
        gap: '3px',
        width: 'full',
        justifyContent: 'stretch',
        opacity: '0',
        transform: 'translateY(40px)',
        transition: '[opacity 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
        '&.in-view': {
          opacity: '100',
          transform: 'translateY(0)',
        },
      })}
    >
      {#each Array.from({ length: 364 }, (_, i) => i) as cellIndex (cellIndex)}
        <div
          style:grid-row={(cellIndex % 7) + 1}
          style:grid-column={Math.floor(cellIndex / 7) + 1}
          class={css(
            {
              aspectRatio: '1/1',
              position: 'relative',
              border: '2px solid',
              borderColor: 'gray.900',
              transition: 'transform',
              transitionDuration: '100ms',
              cursor: 'pointer',
              _hover: {
                transform: 'scale(1.2)',
                zIndex: '10',
              },
            },
            (() => {
              const seed = cellIndex * 2_654_435_761;
              const random = ((seed ^ (seed >>> 16)) * 0x4_5d_9f_3b) >>> 0;
              const normalized = (random % 100) / 100;

              if (normalized < 0.05) return { backgroundColor: 'gray.200' };
              if (normalized < 0.3) return { backgroundColor: 'amber.200' };
              if (normalized < 0.6) return { backgroundColor: 'amber.300' };
              if (normalized < 0.8) return { backgroundColor: 'amber.400' };
              if (normalized < 0.95) return { backgroundColor: 'amber.500' };
              return { backgroundColor: 'amber.600' };
            })(),
          )}
        >
          {#if cellIndex === 256}
            <div
              class={flex({
                position: 'absolute',
                flexDirection: 'column',
                paddingX: '12px',
                paddingY: '8px',
                color: 'gray.900',
                backgroundColor: 'amber.400',
                zIndex: '20',
                fontSize: '12px',
                fontWeight: 'bold',
                whiteSpace: 'nowrap',
                bottom: '[calc(50% + 32px)]',
                left: '[50%]',
                transform: 'translateX(-50%) rotate(-2deg) scale(0)',
                border: '4px solid',
                borderColor: 'gray.900',
                boxShadow: '[4px 4px 0 0 #000]',
                textTransform: 'uppercase',
                opacity: '0',
                transformOrigin: 'bottom center',
                transition:
                  '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s]',
                '.in-view &': {
                  opacity: '100',
                  transform: 'translateX(-50%) rotate(-2deg) scale(1)',
                },
              })}
            >
              <div class={css({ fontWeight: 'black' })}>10월 10일</div>
              <div class={css({ fontWeight: 'black' })}>3,241자 작성</div>
              <div
                class={css({
                  position: 'absolute',
                  bottom: '-8px',
                  left: '[50%]',
                  transform: 'translateX(-50%)',
                  width: '0',
                  height: '0',
                  borderLeft: '[8px solid transparent]',
                  borderRight: '[8px solid transparent]',
                  borderTop: '[8px solid]',
                  borderTopColor: 'gray.900',
                })}
              ></div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  <div class={css({ paddingX: '24px', maxWidth: '[1024px]', marginX: 'auto' })}>
    <div
      bind:this={elements[1]}
      class={center({
        flexDirection: 'column',
        marginBottom: '80px',
        opacity: '0',
        transform: 'translateY(20px) rotate(-1deg)',
        transition: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s]',
        '&.in-view': {
          opacity: '100',
          transform: 'translateY(0) rotate(0)',
        },
      })}
    >
      <div
        class={css({
          display: 'inline-flex',
          alignItems: 'center',
          gap: '8px',
          backgroundColor: 'gray.900',
          color: 'white',
          paddingX: '20px',
          paddingY: '8px',
          fontSize: '14px',
          fontWeight: 'bold',
          marginBottom: '40px',
          letterSpacing: '[0.1em]',
          textTransform: 'uppercase',
          transform: 'rotate(-2deg)',
        })}
      >
        <Icon icon={TrendingUpIcon} size={16} />
        WRITING ANALYTICS
      </div>

      <h2
        class={css({
          fontSize: { base: '[48px]', md: '[64px]' },
          fontWeight: 'black',
          color: 'gray.900',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          marginBottom: '24px',
          lineHeight: '[1.1]',
          textTransform: 'uppercase',
        })}
      >
        데이터로 보는
        <br />
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: '20px',
            display: 'inline-block',
            transform: 'rotate(1deg)',
          })}
        >
          작성 기록
        </span>
      </h2>
      <p
        class={css({
          fontSize: { base: '18px', md: '20px' },
          fontWeight: 'medium',
          color: 'gray.700',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          maxWidth: '[700px]',
          lineHeight: '[1.6]',
        })}
      >
        글쓰기 목표를 설정하고 통계로 과정을 추적해 보세요.
        <br />
        객관적인 데이터가 목표 달성을 함께 돕습니다.
      </p>
    </div>

    <div
      bind:this={elements[2]}
      class={css({
        display: 'grid',
        gridTemplateColumns: { base: '1fr', lg: '1fr 1fr' },
        gap: '32px',
        opacity: '0',
        transform: 'translateY(20px)',
        transition: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.3s]',
        '&.in-view': {
          opacity: '100',
          transform: 'translateY(0)',
        },
      })}
    >
      <div
        class={css({
          backgroundColor: 'white',
          padding: { base: '32px', md: '48px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #000]',
          position: 'relative',
          transform: 'rotate(-0.5deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #000]',
          },
        })}
      >
        <div
          class={center({
            width: '48px',
            height: '48px',
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            marginBottom: '24px',
            transform: 'rotate(45deg)',
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={TypeIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: '24px',
            fontWeight: 'black',
            color: 'gray.900',
            marginBottom: '16px',
            textTransform: 'uppercase',
          })}
        >
          실시간 글자 수
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          실시간으로 업데이트되는 글자 수 통계로 마감이나 분량 목표를 쉽게 관리할 수 있습니다.
        </p>
      </div>

      <div
        class={css({
          backgroundColor: 'gray.900',
          padding: { base: '32px', md: '40px' },
          border: '4px solid',
          borderColor: 'gray.900',
          boxShadow: '[8px 8px 0 0 #fbbf24]',
          position: 'relative',
          transform: 'rotate(0.5deg)',
          transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
          _hover: {
            transform: 'translate(-4px, -4px) rotate(0)',
            boxShadow: '[12px 12px 0 0 #fbbf24]',
          },
        })}
      >
        <div
          class={center({
            width: '48px',
            height: '48px',
            backgroundColor: 'amber.400',
            border: '3px solid',
            borderColor: 'gray.900',
            marginBottom: '24px',
            transform: 'rotate(45deg)',
          })}
        >
          <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={GoalIcon} size={24} />
        </div>
        <h3
          class={css({
            fontSize: '24px',
            fontWeight: 'black',
            color: 'white',
            marginBottom: '16px',
            textTransform: 'uppercase',
          })}
        >
          오늘의 기록
        </h3>
        <p class={css({ fontSize: '16px', color: 'gray.300', lineHeight: '[1.6]', fontWeight: 'medium' })}>
          오늘 추가하고 삭제한 글자 수를 추적하여, 순수한 작성량을 정확히 파악할 수 있습니다.
        </p>
      </div>
    </div>
  </div>
</section>
