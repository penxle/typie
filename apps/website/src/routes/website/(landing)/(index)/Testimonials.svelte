<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { inview } from './inview';

  type Testimonial = {
    avatar: string;
    author: string;
    handle: string;
    content: string;
    url: string;
  };

  // spell-checker:disable
  const testimonials: Testimonial[] = [
    {
      avatar: 'https://pbs.twimg.com/profile_images/1963278993085624320/UvQIYXE3_400x400.jpg',
      author: '백지',
      handle: '@paperr_white',
      content: '타이피가 좋다… 쓸데없는 플랫폼이 아니라 그냥 정말 글 쓰는 기능만 있어서 더 좋다…',
      url: 'https://x.com/paperr_white/status/1933026516767756512',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1947651866332172288/3kMyY1bb_400x400.jpg',
      author: '주성이',
      handle: '@izatsuyu_dream',
      content: '타이피는 정말 좋은 어플이다\n타이피어플출시되고나서 나의 연성생활달라졌다',
      url: 'https://x.com/izatsuyu_dream/status/1947480997010018709?s=46&t=CT-W3Ige047OuUmFWL09YA',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1990274009926422528/ydVC8oAB_400x400.jpg',
      author: '8484',
      handle: '@aibo8484',
      content:
        '타이피 진짜 좋죠 퇴근길에 타이피를 생각해 보세요 집에서 기다리고 있는 마누라 같고 포근하고 말랑하고 달콤한 무언가를 떠올리게 해요',
      url: 'https://x.com/aibo8484/status/1945484259067863163',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1946559332167962624/8VjxAiKX_400x400.jpg',
      author: '쀼뀨쀼',
      handle: '@pp_ggpp',
      content: '사용 한줄평: 최고 장점은 PC-모바일 동시 작업 가능한 한국어 글쓰기앱 + 포스팅 기능\n평점: ★★★★☆(4.5/5.0)',
      url: 'https://x.com/pp_ggpp/status/1936411971525775724',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1912656734738632704/uWHFBpb__400x400.jpg',
      author: '킴쓔',
      handle: '@soosoo_mocha',
      content:
        '이번 마감을 빨리 끝낼 수 있었던 일등공신이었습니다 특히 저처럼 여러 기기를 옮겨가며 글을 쓰시는 분께 추천 드립니다. 모바일과 PC에서 동시에 열고 쓸 수도 있고, 자동저장 기능이 정말 훌륭합니다. 구독료가 아깝지 않아서 앞으로도 계속 쓸 계획입니다',
      url: 'https://x.com/soosoo_mocha/status/1939327966271697303',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1796169430214553600/BgeyWus2_400x400.jpg',
      author: '도라방스',
      handle: '@purpledora2',
      content:
        '저도 타이피를 아주 잘 쓰고 있어서 장점을 공유해 봅니다\n\n1. 우선 가장 좋은 점은 직관적인 인터페이스에요.\n내가 지금 어떤 작품의 몇 화를 쓰고 있는지 바로 보이고, 다른 화수로 이동하기도 용이합니다. 전 여러 작품을 동시에 볼 때가 많은데 파일을 일일이 백업할 필요가 없어서 좋았어요.',
      url: 'https://x.com/purpledora2/status/1937485680575434818',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1907472029370167296/vHvEpnCv_400x400.jpg',
      author: '레슈',
      handle: '@skydowny',
      content:
        '타이피 좋아요...\n다른 것도 다 좋은데 매일 얼마나 썼는지 기록이 되는 게 제일 좋음... 글 열심히 쓸 수 있게 동기부여가 되어 주는 것 같아요 오늘 날짜에 색칠하려고 조금이라도 쓰게 돼서 참 조음\n덕분에 글 샥샥이 기계가 됐다네요 굿 타이피',
      url: 'https://x.com/skydowny/status/1936639292354547957',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1909293418960031744/2J4lqgcX_400x400.jpg',
      author: '초재벌갑부박쥐',
      handle: '@sugobat',
      content:
        '언젠가 이런 서비스가 오리라 믿고 있었습니다.\n진짜 디테일하게 여러 기능들이 많고요\n써보면서 개인적으로 마음에 들었던 것들 두서 없이 적어보겠습니다\n\n0. 데스크탑/모바일 UI가 통일되어있음\n1. 지금까지 쓴 글자수 확인 가능 (전일/특정 달 글자수 비교 가능)\n2. ★폰트를 직접 추가 가능★',
      url: 'https://x.com/sugobat/status/1937516814310989972',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1907316934905131008/DmV5HFLn_400x400.jpg',
      author: '마눌고양이',
      handle: '@Giantsquidbig',
      content: '타이피 정말 글쓰기 좋은 툴임',
      url: 'https://x.com/Giantsquidbig/status/1940645747948769482',
    },
    {
      avatar: 'https://pbs.twimg.com/profile_images/1729053424979951617/8tloydYo_400x400.jpg',
      author: '보미',
      handle: '@bombom_mabi',
      content: '타이피 정말 맘에 들어. 갈수록 맘에 들어.',
      url: 'https://x.com/bombom_mabi/status/1943148481918701846',
    },
  ];
  // spell-checker:enable

  let currentIndex = $state(0);
  let isPaused = $state(false);

  const cardHeight = 180;
  const activeCard = $derived(testimonials[currentIndex]);

  const getOffset = (idx: number, center: number) => {
    const total = testimonials.length;
    let diff = idx - center;
    if (diff > total / 2) diff -= total;
    if (diff < -total / 2) diff += total;
    return diff;
  };

  onMount(() => {
    const interval = setInterval(() => {
      if (!isPaused) {
        currentIndex = (currentIndex + 1) % testimonials.length;
      }
    }, 4000);

    return () => clearInterval(interval);
  });
</script>

<section
  class={css({
    position: 'relative',
    paddingX: { sm: '24px', lg: '80px' },
    paddingY: { sm: '80px', lg: '120px' },
    backgroundColor: 'dark.gray.950',
    borderTopWidth: '1px',
    borderTopColor: 'dark.gray.900',
    overflow: 'hidden',
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

  <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: { sm: '1fr', lg: '[1fr 1fr]' },
        gap: { sm: '48px', lg: '80px' },
        alignItems: 'center',
      })}
    >
      <div
        class={css({
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
          Testimonials
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
          지금도 누구나
          <br />
          쓰고 고치는 중.
        </h2>

        <p
          class={css({
            fontSize: { sm: '16px', lg: '18px' },
            color: 'dark.gray.400',
            lineHeight: '[1.65]',
            maxWidth: '[400px]',
          })}
        >
          왜 타이피인지, 직접 확인해보세요.
        </p>

        <div class={css({ display: 'flex', gap: '8px', marginTop: '32px' })}>
          {#each testimonials as testimonial, idx (testimonial.handle)}
            <button
              class={css({
                width: '8px',
                height: '8px',
                backgroundColor: idx === currentIndex ? 'dark.brand.400' : 'dark.gray.700',
                border: 'none',
                cursor: 'pointer',
                transition: '[background-color 0.3s ease-out]',
                _hover: {
                  backgroundColor: idx === currentIndex ? 'dark.brand.400' : 'dark.gray.600',
                },
              })}
              aria-label={testimonial.author}
              onclick={() => (currentIndex = idx)}
              type="button"
            ></button>
          {/each}
        </div>
      </div>

      <div
        class={css({
          position: 'relative',
          height: { sm: '[500px]', lg: '[600px]' },
          overflow: 'hidden',
          opacity: '0',
          transform: 'translate3d(0, 20px, 0)',
          transition: '[opacity 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s, transform 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s]',
          '&.in-view': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
        {@attach inview}
        onmouseenter={() => (isPaused = true)}
        onmouseleave={() => (isPaused = false)}
        role="region"
      >
        <div
          class={css({
            position: 'absolute',
            top: '0',
            left: '0',
            right: '0',
            height: '[120px]',
            background: '[linear-gradient(to bottom, token(colors.dark.gray.950), transparent)]',
            zIndex: '20',
            pointerEvents: 'none',
          })}
        ></div>
        <div
          class={css({
            position: 'absolute',
            bottom: '0',
            left: '0',
            right: '0',
            height: '[120px]',
            background: '[linear-gradient(to top, token(colors.dark.gray.950), transparent)]',
            zIndex: '20',
            pointerEvents: 'none',
          })}
        ></div>

        <button
          class={css({
            position: 'absolute',
            top: '[50%]',
            transform: 'translateY(-50%)',
            right: '0',
            width: { sm: '[calc(100% - 16px)]', lg: '[calc(100% - 24px)]' },
            height: '[180px]',
            paddingX: '24px',
            paddingY: '20px',
            backgroundColor: 'dark.gray.900',
            borderWidth: '1px',
            borderColor: 'dark.brand.400',
            textAlign: 'left',
            cursor: 'pointer',
            overflow: 'hidden',
            zIndex: '[15]',
            transition: '[border-color 0.3s ease-out]',
            _hover: {
              borderColor: 'dark.brand.300',
            },
          })}
          onclick={() => window.open(activeCard.url, '_blank', 'noopener,noreferrer')}
          type="button"
        >
          {#key currentIndex}
            <div
              class={css({ position: 'absolute', inset: '0', paddingX: '24px', paddingY: '20px' })}
              in:fade={{ duration: 200, delay: 150 }}
              out:fade={{ duration: 150 }}
            >
              <p
                class={css({
                  fontSize: '15px',
                  color: 'dark.gray.200',
                  lineHeight: '[1.65]',
                  marginBottom: '20px',
                  whiteSpace: 'pre-line',
                  lineClamp: '3',
                  overflow: 'hidden',
                })}
              >
                {activeCard.content}
              </p>

              <div class={css({ display: 'flex', alignItems: 'center', gap: '12px', position: 'absolute', bottom: '20px', left: '24px' })}>
                <img
                  class={css({
                    size: '36px',
                    objectFit: 'cover',
                    borderRadius: 'full',
                  })}
                  alt={activeCard.author}
                  src={activeCard.avatar}
                />
                <div>
                  <span class={css({ display: 'block', fontSize: '14px', fontWeight: 'medium', color: 'dark.gray.100' })}>
                    {activeCard.author}
                  </span>
                  <span class={css({ display: 'block', fontSize: '12px', color: 'dark.gray.500' })}>
                    {activeCard.handle}
                  </span>
                </div>
              </div>
            </div>
          {/key}
        </button>

        <div
          class={css({
            position: 'absolute',
            top: '[50%]',
            right: '0',
            width: { sm: '[calc(100% - 16px)]', lg: '[calc(100% - 24px)]' },
            zIndex: '[10]',
          })}
        >
          {#each testimonials as testimonial, idx (testimonial.handle)}
            {@const offset = getOffset(idx, currentIndex)}
            {@const isActive = offset === 0}
            {@const isVisible = Math.abs(offset) <= 1}
            <button
              style:transform={`translateY(${offset * cardHeight - cardHeight / 2}px)`}
              style:opacity={isActive || !isVisible ? 0 : 1}
              class={css({
                position: 'absolute',
                top: '0',
                left: '0',
                width: 'full',
                height: '[180px]',
                backgroundColor: 'dark.gray.950',
                borderTopWidth: '0',
                borderBottomWidth: '0',
                borderLeftWidth: '1px',
                borderRightWidth: '1px',
                borderColor: 'dark.gray.900',
                textAlign: 'left',
                cursor: 'pointer',
                overflow: 'hidden',
                transition: '[transform 0.5s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.3s ease-out, border-color 0.2s ease-out]',
                _hover: {
                  borderColor: 'dark.gray.800',
                },
              })}
              onclick={() => (currentIndex = idx)}
              type="button"
            >
              <div class={css({ position: 'absolute', inset: '0', paddingX: '24px', paddingY: '20px' })}>
                <p
                  class={css({
                    fontSize: '15px',
                    color: 'dark.gray.400',
                    lineHeight: '[1.65]',
                    whiteSpace: 'pre-line',
                    lineClamp: '3',
                    overflow: 'hidden',
                  })}
                >
                  {testimonial.content}
                </p>

                <div
                  class={css({ display: 'flex', alignItems: 'center', gap: '12px', position: 'absolute', bottom: '20px', left: '24px' })}
                >
                  <img
                    class={css({
                      size: '32px',
                      objectFit: 'cover',
                      borderRadius: 'full',
                      filter: '[grayscale(100%)]',
                      opacity: '[0.5]',
                    })}
                    alt={testimonial.author}
                    src={testimonial.avatar}
                  />
                  <div>
                    <span class={css({ display: 'block', fontSize: '13px', fontWeight: 'medium', color: 'dark.gray.500' })}>
                      {testimonial.author}
                    </span>
                    <span class={css({ display: 'block', fontSize: '11px', color: 'dark.gray.600' })}>
                      {testimonial.handle}
                    </span>
                  </div>
                </div>
              </div>
            </button>
          {/each}
        </div>
      </div>
    </div>
  </div>
</section>
