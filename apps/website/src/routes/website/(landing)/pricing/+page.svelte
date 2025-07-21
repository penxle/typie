<script lang="ts">
  import NumberFlow from '@number-flow/svelte';
  import { onMount } from 'svelte';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import SparklesIcon from '~icons/lucide/sparkles';
  import ZapIcon from '~icons/lucide/zap';
  import { env } from '$env/dynamic/public';
  import { Icon } from '$lib/components';
  import { comma } from '$lib/utils/number';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  let billingPeriod = $state<'monthly' | 'annually'>('monthly');
  let elements = $state<HTMLElement[]>([]);
  let expandedFaqIndex = $state<number | null>(null);
  let currentPrice = $derived(billingPeriod === 'monthly' ? 4900 : Math.floor(49_000 / 12));

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

    elements.forEach((element) => {
      if (element) observer.observe(element);
    });

    return () => {
      elements.forEach((element) => {
        if (element) observer.unobserve(element);
      });
    };
  });

  const plans = {
    basic: {
      name: '타이피 BASIC ACCESS',
      price: 0,
      description: '간단한 메모와 가벼운 글쓰기를 시작하세요',
      features: ['총 16,000자까지 작성 가능', '총 20MB까지 파일 업로드 가능'],
    },
    full: {
      name: '타이피 FULL ACCESS',
      price: 4900,
      yearlyPrice: 49_000,
      description: '제한 없이 자유롭게 글쓰기를 즐기세요',
      features: [
        '무제한 글자 수',
        '무제한 파일 업로드',
        '고급 검색',
        '맞춤법 검사',
        '커스텀 게시 주소',
        '커스텀 폰트 업로드',
        '베타 기능 우선 접근',
        '문제 발생시 우선 지원',
        '디스코드 커뮤니티 참여',
        '그리고 더 많은 혜택',
      ],
      badge: 'RECOMMENDED',
    },
  };

  const faqs = [
    {
      question: '유료 플랜에서 무료 플랜으로 다운그레이드하면 기존 데이터는 어떻게 되나요?',
      answer:
        '기존 데이터는 모두 안전하게 보존됩니다. 다만 무료 플랜의 제한을 초과한 콘텐츠는 읽기 전용으로 전환되며, 제한 내로 조정하시면 다시 편집이 가능합니다.',
    },
    {
      question: '언제든지 요금제를 변경할 수 있나요?',
      answer: '네, 언제든지 요금제를 변경할 수 있습니다. 변경된 요금제는 다음 결제 주기부터 자동으로 적용됩니다.',
    },
    {
      question: '결제 수단은 무엇을 지원하나요?',
      answer: '국내 신용카드, 체크카드를 지원합니다.',
    },
    {
      question: '환불 정책은 어떻게 되나요?',
      answer: '결제 후 7일 이내에는 전액 환불이 가능합니다. 이후에는 남은 기간에 대해 일할 계산하여 환불해 드립니다.',
    },
  ];

  const toggleFaq = (index: number) => {
    expandedFaqIndex = expandedFaqIndex === index ? null : index;
  };
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
      top: '[10%]',
      left: '[5%]',
      width: '[600px]',
      height: '[600px]',
      backgroundImage: 'radial-gradient(circle, token(colors.amber.300), transparent)',
      opacity: '[0.3]',
      filter: '[blur(180px)]',
      pointerEvents: 'none',
      zIndex: '1',
    })}
  ></div>

  <div
    class={css({
      position: 'absolute',
      bottom: '[-20%]',
      right: '[-15%]',
      width: '[800px]',
      height: '[800px]',
      backgroundImage: 'radial-gradient(circle, token(colors.gray.300), transparent)',
      opacity: '[0.4]',
      filter: '[blur(150px)]',
      pointerEvents: 'none',
      zIndex: '1',
    })}
  ></div>

  <section
    bind:this={elements[0]}
    class={css({
      position: 'relative',
      paddingTop: '100px',
      paddingBottom: '80px',
      paddingX: '24px',
      zIndex: '2',
      opacity: '0',
      transform: 'translateY(40px) rotate(-1deg) scale(0.95)',
      transition: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
      '&.in-view': {
        opacity: '100',
        transform: 'translateY(0) rotate(0) scale(1)',
      },
    })}
  >
    <div class={center({ flexDirection: 'column', maxWidth: '[1024px]', marginX: 'auto' })}>
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
          transform: 'rotate(-2deg) scale(0)',
          transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.3s]',
          '.in-view &': {
            transform: 'rotate(-2deg) scale(1)',
          },
        })}
      >
        <Icon icon={SparklesIcon} size={16} />
        PRICING
      </div>

      <h1
        class={css({
          fontSize: { base: '[56px]', md: '[80px]', lg: '[96px]' },
          fontWeight: 'black',
          color: 'gray.900',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          lineHeight: '[1]',
          marginBottom: '32px',
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
        심플하고
        <br />
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: '20px',
            display: 'inline-block',
            transform: 'rotate(1deg) scale(0)',
            transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.6s]',
            '.in-view &': {
              transform: 'rotate(1deg) scale(1)',
            },
          })}
        >
          투명한
        </span>
        요금제
      </h1>
      <p
        class={css({
          fontSize: '20px',
          fontWeight: 'medium',
          color: 'gray.700',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          lineHeight: '[1.7]',
          maxWidth: '[700px]',
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
        복잡한 옵션 없이, 필요에 맞는 플랜을 선택하세요.
        <br />
        언제든지 업그레이드하거나 취소할 수 있습니다.
      </p>
    </div>
  </section>

  <section
    class={css({
      position: 'relative',
      paddingTop: '0',
      paddingBottom: '160px',
      paddingX: '24px',
      zIndex: '2',
    })}
  >
    <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
      <div
        bind:this={elements[1]}
        class={css({
          marginBottom: '64px',
          display: 'flex',
          justifyContent: 'center',
          opacity: '0',
          transform: 'translateY(20px) rotate(-1deg)',
          transition: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.1s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.1s]',
          '&.in-view': {
            opacity: '100',
            transform: 'translateY(0) rotate(0)',
          },
        })}
      >
        <div
          class={flex({
            alignItems: 'center',
            gap: '8px',
            paddingY: '4px',
            paddingX: '8px',
            backgroundColor: 'white',
            border: '4px solid',
            borderColor: 'gray.900',
            boxShadow: '[6px 6px 0 0 #000]',
          })}
        >
          <button
            class={css({
              paddingX: '24px',
              paddingY: '12px',
              fontSize: '16px',
              fontWeight: 'bold',
              letterSpacing: '[0.05em]',
              textTransform: 'uppercase',
              transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              backgroundColor: billingPeriod === 'monthly' ? 'gray.900' : 'white',
              color: billingPeriod === 'monthly' ? 'white' : 'gray.900',
              cursor: 'pointer',
              border: 'none',
              position: 'relative',
              zIndex: billingPeriod === 'monthly' ? '2' : '1',
              transform: billingPeriod === 'monthly' ? 'scale(1.05)' : 'scale(1)',
              _hover: {
                backgroundColor: billingPeriod === 'monthly' ? 'gray.800' : 'gray.100',
              },
            })}
            onclick={() => (billingPeriod = 'monthly')}
            type="button"
          >
            월간 결제
          </button>
          <button
            class={css({
              paddingX: '24px',
              paddingY: '12px',
              fontSize: '16px',
              fontWeight: 'bold',
              letterSpacing: '[0.05em]',
              textTransform: 'uppercase',
              transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              backgroundColor: billingPeriod === 'annually' ? 'gray.900' : 'white',
              color: billingPeriod === 'annually' ? 'white' : 'gray.900',
              cursor: 'pointer',
              border: 'none',
              position: 'relative',
              zIndex: billingPeriod === 'annually' ? '2' : '1',
              transform: billingPeriod === 'annually' ? 'scale(1.05)' : 'scale(1)',
              _hover: {
                backgroundColor: billingPeriod === 'annually' ? 'gray.800' : 'gray.100',
              },
            })}
            onclick={() => (billingPeriod = 'annually')}
            type="button"
          >
            연간 결제
            <span
              class={css({
                marginLeft: '8px',
                paddingX: '10px',
                paddingY: '4px',
                fontSize: '12px',
                fontWeight: 'black',
                color: 'gray.900',
                backgroundColor: 'amber.400',
                transform: 'rotate(-2deg)',
                display: 'inline-block',
                border: '2px solid',
                borderColor: 'gray.900',
              })}
            >
              2개월 무료
            </span>
          </button>
        </div>
      </div>

      <div
        bind:this={elements[2]}
        class={css({
          display: 'grid',
          gridTemplateColumns: { base: '1fr', lg: '1fr 1fr' },
          gap: '32px',
          opacity: '0',
          transform: 'translateY(20px)',
          transition: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s]',
          marginTop: '20px',
          '&.in-view': {
            opacity: '100',
            transform: 'translateY(0)',
          },
        })}
      >
        <div
          class={css({
            position: 'relative',
            backgroundColor: 'white',
            padding: { base: '32px', md: '48px' },
            border: '4px solid',
            borderColor: 'gray.900',
            boxShadow: '[8px 8px 0 0 #000]',
            transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
            transform: 'rotate(-0.5deg)',
            display: 'flex',
            flexDirection: 'column',
            _hover: {
              transform: 'translate(-4px, -4px) rotate(0deg)',
              boxShadow: '[12px 12px 0 0 #000]',
            },
          })}
        >
          <h3
            class={css({
              fontSize: '[28px]',
              fontWeight: 'black',
              color: 'gray.900',
              fontFamily: 'Paperlogy',
              marginBottom: '12px',
              textTransform: 'uppercase',
              letterSpacing: '[0.05em]',
            })}
          >
            {plans.basic.name}
          </h3>
          <p
            class={css({
              fontSize: '16px',
              color: 'gray.700',
              fontFamily: 'Pretendard',
              marginBottom: '32px',
              lineHeight: '[1.6]',
              fontWeight: 'medium',
            })}
          >
            {plans.basic.description}
          </p>

          <div class={css({ marginBottom: '40px' })}>
            <div class={flex({ alignItems: 'center', gap: '8px', height: '[72px]' })}>
              <span class={css({ fontSize: '[56px]', fontWeight: 'black', color: 'gray.900', lineHeight: '[1]', fontFamily: 'Paperlogy' })}>
                무료
              </span>
            </div>
          </div>

          <a
            class={cx(
              'group',
              css({
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: 'full',
                paddingY: '14px',
                fontSize: '16px',
                fontWeight: 'bold',
                backgroundColor: 'gray.100',
                color: 'gray.900',
                border: '3px solid',
                borderColor: 'gray.900',
                transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                marginBottom: '32px',
                textTransform: 'uppercase',
                letterSpacing: '[0.05em]',
                position: 'relative',
                overflow: 'hidden',
                _hover: {
                  backgroundColor: 'gray.900',
                  color: 'white',
                  transform: 'translateY(-2px)',
                },
              }),
            )}
            href={env.PUBLIC_AUTH_URL}
          >
            무료로 시작하기
          </a>

          <div class={flex({ flexDirection: 'column', gap: '16px', flex: '1' })}>
            <p
              class={css({
                fontSize: '14px',
                fontWeight: 'bold',
                color: 'gray.900',
                textTransform: 'uppercase',
                letterSpacing: '[0.05em]',
              })}
            >
              포함 사항:
            </p>
            <ul class={flex({ flexDirection: 'column', gap: '16px' })}>
              {#each plans.basic.features as feature, index (index)}
                <li class={flex({ alignItems: 'flex-start', gap: '12px' })}>
                  <div
                    class={css({
                      width: '20px',
                      height: '20px',
                      backgroundColor: 'gray.200',
                      border: '3px solid',
                      borderColor: 'gray.900',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: 0,
                      marginTop: '2px',
                    })}
                  >
                    <Icon style={css.raw({ color: 'gray.900' })} icon={CheckIcon} size={12} />
                  </div>
                  <span class={css({ fontSize: '15px', color: 'gray.700', lineHeight: '[1.6]', fontWeight: 'medium' })}>{feature}</span>
                </li>
              {/each}
            </ul>
          </div>
        </div>

        <div
          class={css({
            position: 'relative',
            backgroundColor: 'gray.900',
            padding: { base: '32px', md: '48px' },
            border: '4px solid',
            borderColor: 'gray.900',
            boxShadow: '[8px 8px 0 0 #fbbf24]',
            transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
            transform: 'rotate(0.5deg)',
            display: 'flex',
            flexDirection: 'column',
            _hover: {
              transform: 'translate(-4px, -4px) rotate(0deg)',
              boxShadow: '[12px 12px 0 0 #fbbf24]',
            },
          })}
        >
          {#if plans.full.badge}
            <div
              class={css({
                position: 'absolute',
                top: '-20px',
                right: '20px',
                paddingX: '20px',
                paddingY: '8px',
                fontSize: '12px',
                fontWeight: 'black',
                color: 'gray.900',
                backgroundColor: 'amber.400',
                letterSpacing: '[0.1em]',
                textTransform: 'uppercase',
                border: '4px solid',
                borderColor: 'gray.900',
                transform: 'rotate(2deg)',
                boxShadow: '[4px 4px 0 0 #000]',
                zIndex: '10',
              })}
            >
              {plans.full.badge}
            </div>
          {/if}

          <h3
            class={css({
              fontSize: '[28px]',
              fontWeight: 'black',
              color: 'white',
              fontFamily: 'Paperlogy',
              marginBottom: '12px',
              textTransform: 'uppercase',
              letterSpacing: '[0.05em]',
            })}
          >
            {plans.full.name}
          </h3>
          <p
            class={css({
              fontSize: '16px',
              color: 'gray.300',
              fontFamily: 'Pretendard',
              marginBottom: '32px',
              lineHeight: '[1.6]',
              fontWeight: 'medium',
            })}
          >
            {plans.full.description}
          </p>

          <div class={css({ marginBottom: '40px' })}>
            <div class={flex({ alignItems: 'baseline', gap: '8px', height: '[72px]' })}>
              <NumberFlow
                class={css({
                  fontSize: '[56px]',
                  fontWeight: 'black',
                  color: 'amber.400',
                  lineHeight: '[1]',
                  fontVariantNumeric: 'tabular-nums',
                  letterSpacing: '[0.05em]',
                  fontFamily: 'Paperlogy',
                })}
                value={currentPrice}
              />
              <div>
                <span class={css({ fontSize: '18px', color: 'gray.400', fontWeight: 'bold' })}>원 / 월</span>
                <span
                  class={css({
                    fontSize: '14px',
                    color: 'gray.500',
                    opacity: billingPeriod === 'annually' ? '100' : '0',
                    transition: '[opacity 0.2s ease]',
                    marginLeft: '4px',
                    fontWeight: 'medium',
                  })}
                >
                  (연 {comma(plans.full.yearlyPrice)}원)
                </span>
              </div>
            </div>
          </div>

          <a
            class={cx(
              'group',
              css({
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '8px',
                width: 'full',
                paddingY: '14px',
                fontSize: '16px',
                fontWeight: 'black',
                backgroundColor: 'amber.400',
                color: 'gray.900',
                border: '3px solid',
                borderColor: 'gray.900',
                transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                marginBottom: '32px',
                textTransform: 'uppercase',
                letterSpacing: '[0.05em]',
                position: 'relative',
                boxShadow: '[4px 4px 0 0 #000]',
                _hover: {
                  transform: 'translate(-2px, -2px)',
                  boxShadow: '[6px 6px 0 0 #000]',
                },
                _active: {
                  transform: 'translate(2px, 2px)',
                  boxShadow: '[2px 2px 0 0 #000]',
                },
              }),
            )}
            href={env.PUBLIC_AUTH_URL}
          >
            <Icon icon={ZapIcon} size={18} />
            지금 시작하기
            <Icon
              style={css.raw({
                transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                _groupHover: {
                  transform: 'translateX(4px) rotate(-15deg)',
                },
              })}
              icon={ArrowRightIcon}
              size={18}
            />
          </a>

          <div class={flex({ flexDirection: 'column', gap: '16px', flex: '1' })}>
            <p class={css({ fontSize: '14px', fontWeight: 'bold', color: 'white', textTransform: 'uppercase', letterSpacing: '[0.05em]' })}>
              제한 없이 모든 기능 사용:
            </p>
            <ul class={flex({ flexDirection: 'column', gap: '16px' })}>
              {#each plans.full.features as feature, index (index)}
                <li class={flex({ alignItems: 'flex-start', gap: '12px' })}>
                  <div
                    class={css({
                      width: '20px',
                      height: '20px',
                      backgroundColor: 'amber.400',
                      border: '3px solid',
                      borderColor: 'gray.900',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: 0,
                      marginTop: '2px',
                      transform: 'rotate(45deg)',
                    })}
                  >
                    <Icon style={css.raw({ color: 'gray.900', transform: 'rotate(-45deg)' })} icon={CheckIcon} size={12} />
                  </div>
                  <span class={css({ fontSize: '15px', color: 'gray.300', lineHeight: '[1.6]', fontWeight: 'medium' })}>{feature}</span>
                </li>
              {/each}
            </ul>
          </div>
        </div>
      </div>
    </div>
  </section>

  <section
    class={css({
      position: 'relative',
      paddingY: '120px',
      paddingX: '24px',
      backgroundColor: 'white',
      zIndex: '2',
    })}
  >
    <div class={css({ maxWidth: '[1024px]', marginX: 'auto' })}>
      <div
        bind:this={elements[3]}
        class={center({
          flexDirection: 'column',
          marginBottom: '80px',
          opacity: '0',
          transform: 'translateY(20px) rotate(-1deg)',
          transition:
            '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.15s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.15s]',
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
          FAQ
        </div>

        <h2
          class={css({
            fontSize: { base: '[48px]', md: '[64px]' },
            fontWeight: 'black',
            color: 'gray.900',
            textAlign: 'center',
            fontFamily: 'Paperlogy',
            lineHeight: '[1.1]',
            textTransform: 'uppercase',
          })}
        >
          자주 묻는
          <br />
          <span
            class={css({
              backgroundColor: 'amber.400',
              paddingX: '20px',
              display: 'inline-block',
              transform: 'rotate(1deg)',
            })}
          >
            질문
          </span>
        </h2>
      </div>

      <div
        bind:this={elements[4]}
        class={css({
          maxWidth: '[800px]',
          marginX: 'auto',
          opacity: '0',
          transform: 'translateY(20px)',
          transition: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s]',
          '&.in-view': {
            opacity: '100',
            transform: 'translateY(0)',
          },
        })}
      >
        <div
          class={flex({
            flexDirection: 'column',
            gap: '24px',
          })}
        >
          {#each faqs as faq, index (index)}
            <div
              class={css({
                backgroundColor: 'white',
                border: '4px solid',
                borderColor: 'gray.900',
                transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                overflow: 'hidden',
                boxShadow: expandedFaqIndex === index ? '[8px 8px 0 0 #000]' : '[4px 4px 0 0 #000]',
                transform: expandedFaqIndex === index ? 'translate(-2px, -2px)' : 'translate(0, 0)',
                _hover: {
                  transform: 'translate(-2px, -2px)',
                  boxShadow: '[8px 8px 0 0 #000]',
                },
              })}
            >
              <button
                class={css({
                  width: 'full',
                  paddingX: '32px',
                  paddingY: '24px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: '16px',
                  textAlign: 'left',
                  cursor: 'pointer',
                  backgroundColor: expandedFaqIndex === index ? 'amber.50' : 'transparent',
                  border: 'none',
                  transition: '[background-color 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                  _hover: {
                    backgroundColor: expandedFaqIndex === index ? 'amber.50' : 'gray.50',
                  },
                })}
                onclick={() => toggleFaq(index)}
                type="button"
              >
                <h3
                  class={css({
                    fontSize: '20px',
                    fontWeight: 'bold',
                    color: 'gray.900',
                    fontFamily: 'Pretendard',
                    lineHeight: '[1.5]',
                  })}
                >
                  {faq.question}
                </h3>
                <div
                  class={css({
                    width: '32px',
                    height: '32px',
                    backgroundColor: expandedFaqIndex === index ? 'gray.900' : 'amber.400',
                    border: '3px solid',
                    borderColor: 'gray.900',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                    transform: expandedFaqIndex === index ? 'rotate(180deg)' : 'rotate(0deg)',
                    transition: '[transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)]',
                  })}
                >
                  <Icon
                    style={css.raw({
                      color: expandedFaqIndex === index ? 'white' : 'gray.900',
                    })}
                    icon={ChevronDownIcon}
                    size={18}
                  />
                </div>
              </button>

              <div
                class={css({
                  display: 'grid',
                  gridTemplateRows: expandedFaqIndex === index ? '1fr' : '0fr',
                  transition: '[grid-template-rows 0.15s cubic-bezier(0.4, 0, 0.2, 1)]',
                })}
              >
                <div
                  class={css({
                    overflow: 'hidden',
                  })}
                >
                  <div
                    class={css({
                      backgroundColor: 'amber.50',
                      borderTop: '4px solid',
                      borderColor: 'gray.900',
                    })}
                  >
                    <p
                      class={css({
                        fontSize: '17px',
                        color: 'gray.800',
                        fontFamily: 'Pretendard',
                        lineHeight: '[1.7]',
                        paddingX: '32px',
                        paddingY: '24px',
                        fontWeight: 'medium',
                        opacity: expandedFaqIndex === index ? '100' : '0',
                        transform: expandedFaqIndex === index ? 'translateY(0)' : 'translateY(-10px)',
                        transition: '[opacity 0.2s ease-out, transform 0.2s ease-out]',
                        transitionDelay: expandedFaqIndex === index ? '0.05s' : '0s',
                      })}
                    >
                      {faq.answer}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </section>

  <section
    class={css({
      position: 'relative',
      paddingY: '160px',
      paddingX: '24px',
      backgroundColor: 'gray.900',
      overflow: 'hidden',
      zIndex: '2',
      borderTop: '8px solid',
      borderColor: 'gray.900',
    })}
  >
    <div
      class={css({
        position: 'absolute',
        inset: '0',
        backgroundImage: `
          repeating-linear-gradient(
            45deg,
            transparent,
            transparent 20px,
            rgba(251, 191, 36, 0.1) 20px,
            rgba(251, 191, 36, 0.1) 40px
          )
        `,
        pointerEvents: 'none',
      })}
    ></div>

    <div
      class={css({
        position: 'absolute',
        top: '[10%]',
        left: '[5%]',
        width: '100px',
        height: '100px',
        backgroundColor: 'amber.400',
        transform: 'rotate(45deg)',
        opacity: '[0.2]',
      })}
    ></div>
    <div
      class={css({
        position: 'absolute',
        bottom: '[15%]',
        right: '[10%]',
        width: '80px',
        height: '80px',
        backgroundColor: 'amber.400',
        transform: 'rotate(15deg)',
        opacity: '[0.15]',
      })}
    ></div>

    <div class={css({ maxWidth: '[1024px]', marginX: 'auto', position: 'relative' })}>
      <div
        bind:this={elements[5]}
        class={center({
          flexDirection: 'column',
          textAlign: 'center',
          opacity: '0',
          transform: 'translateY(40px) rotate(-1deg) scale(0.95)',
          transition:
            '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.15s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.15s]',
          '&.in-view': {
            opacity: '100',
            transform: 'translateY(0) rotate(0) scale(1)',
          },
        })}
      >
        <div
          class={css({
            display: 'inline-flex',
            alignItems: 'center',
            gap: '8px',
            backgroundColor: 'amber.400',
            color: 'gray.900',
            paddingX: '24px',
            paddingY: '10px',
            fontSize: '14px',
            fontWeight: 'black',
            marginBottom: '48px',
            border: '4px solid',
            borderColor: 'gray.900',
            letterSpacing: '[0.1em]',
            textTransform: 'uppercase',
            transform: 'rotate(-2deg)',
            boxShadow: '[4px 4px 0 0 #000]',
          })}
        >
          <Icon icon={SparklesIcon} size={16} />
          START NOW
        </div>

        <h2
          class={css({
            fontSize: { base: '[64px]', md: '[80px]', lg: '[96px]' },
            fontWeight: 'black',
            color: 'white',
            fontFamily: 'Paperlogy',
            marginBottom: '32px',
            lineHeight: '[1]',
            textTransform: 'uppercase',
          })}
        >
          준비되셨나요
          <span
            class={css({
              display: 'inline-block',
              backgroundColor: 'amber.400',
              color: 'gray.900',
              paddingX: '24px',
              marginX: '8px',
              transform: 'rotate(2deg)',
            })}
          >
            ?
          </span>
        </h2>
        <p
          class={css({
            fontSize: { base: '20px', md: '24px' },
            fontWeight: 'medium',
            color: 'gray.300',
            fontFamily: 'Pretendard',
            marginBottom: '56px',
            lineHeight: '[1.6]',
            maxWidth: '[700px]',
            marginX: 'auto',
          })}
        >
          지금 바로 타이피와 함께 더 나은 글쓰기를 시작하세요.
          <br />
          무료로 시작하고, 필요할 때 업그레이드하세요.
        </p>

        <a
          class={cx(
            'group',
            css({
              display: 'inline-flex',
              alignItems: 'center',
              gap: '10px',
              paddingX: '40px',
              paddingY: '20px',
              fontSize: '20px',
              fontWeight: 'black',
              color: 'gray.900',
              backgroundColor: 'amber.400',
              border: '4px solid',
              borderColor: 'gray.900',
              transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              transform: 'rotate(-1deg)',
              textTransform: 'uppercase',
              letterSpacing: '[0.05em]',
              boxShadow: '[8px 8px 0 0 #000]',
              _hover: {
                transform: 'translate(-4px, -4px) rotate(0)',
                boxShadow: '[12px 12px 0 0 #000]',
              },
              _active: {
                transform: 'translate(4px, 4px)',
                boxShadow: '[4px 4px 0 0 #000]',
              },
            }),
          )}
          href={env.PUBLIC_AUTH_URL}
        >
          무료로 시작하기
          <Icon
            style={css.raw({
              transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              _groupHover: {
                transform: 'translateX(4px) rotate(-15deg)',
              },
            })}
            icon={ArrowRightIcon}
            size={24}
          />
        </a>
      </div>
    </div>
  </section>
</div>
