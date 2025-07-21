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
  <!-- Background gradient -->
  <div
    class={css({
      position: 'absolute',
      inset: '0',
      backgroundImage:
        'linear-gradient(to bottom, token(colors.white), token(colors.gray.50) 25%, token(colors.gray.50) 75%, token(colors.white))',
      zIndex: '0',
    })}
  ></div>

  <!-- Dot pattern -->
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

  <!-- Gradient orbs -->
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
      paddingTop: '160px',
      paddingBottom: '80px',
      paddingX: '24px',
      zIndex: '2',
      opacity: '0',
      transform: 'translateY(20px)',
      transition: '[opacity 0.4s ease-out, transform 0.4s ease-out]',
    })}
  >
    <div class={center({ flexDirection: 'column', maxWidth: '[1024px]', marginX: 'auto' })}>
      <!-- Badge -->
      <div
        class={css({
          display: 'inline-flex',
          alignItems: 'center',
          gap: '8px',
          backgroundColor: 'amber.50',
          color: 'amber.900',
          paddingX: '20px',
          paddingY: '8px',
          borderRadius: 'full',
          fontSize: '14px',
          fontWeight: 'semibold',
          marginBottom: '40px',
          border: '1px solid',
          borderColor: 'amber.200',
          letterSpacing: 'wide',
          animation: 'none',
        })}
      >
        <Icon icon={SparklesIcon} size={16} />
        요금제
      </div>

      <h1
        class={css({
          fontSize: { base: '[48px]', md: '[72px]' },
          fontWeight: 'extrabold',
          color: 'gray.900',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          lineHeight: '[1.1]',
          marginBottom: '32px',
          letterSpacing: 'tight',
        })}
      >
        심플하고 투명한 요금제
      </h1>
      <p
        class={css({
          fontSize: { base: '18px', md: '21px' },
          fontWeight: 'normal',
          color: 'gray.600',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          lineHeight: '[1.7]',
          maxWidth: '[700px]',
          marginX: 'auto',
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
          transform: 'translateY(20px)',
          transition: '[opacity 0.4s ease-out 0.1s, transform 0.4s ease-out 0.1s]',
        })}
      >
        <div
          class={flex({
            alignItems: 'center',
            gap: '0',
            padding: '4px',
            backgroundColor: 'white',
            borderRadius: 'full',
            border: '1px solid',
            borderColor: 'gray.200',
            boxShadow: '[0 4px 12px rgba(0, 0, 0, 0.04)]',
          })}
        >
          <button
            class={css({
              paddingX: '24px',
              paddingY: '10px',
              fontSize: '14px',
              fontWeight: 'semibold',
              borderRadius: 'full',
              transition: '[all 0.2s ease]',
              backgroundColor: billingPeriod === 'monthly' ? 'gray.900' : 'transparent',
              color: billingPeriod === 'monthly' ? 'white' : 'gray.600',
              boxShadow: billingPeriod === 'monthly' ? '[0 4px 12px rgba(0, 0, 0, 0.1)]' : undefined,
              cursor: 'pointer',
            })}
            onclick={() => (billingPeriod = 'monthly')}
            type="button"
          >
            월간 결제
          </button>
          <button
            class={css({
              paddingX: '24px',
              paddingY: '10px',
              fontSize: '14px',
              fontWeight: 'semibold',
              borderRadius: 'full',
              transition: '[all 0.2s ease]',
              backgroundColor: billingPeriod === 'annually' ? 'gray.900' : 'transparent',
              color: billingPeriod === 'annually' ? 'white' : 'gray.600',
              boxShadow: billingPeriod === 'annually' ? '[0 4px 12px rgba(0, 0, 0, 0.1)]' : undefined,
              cursor: 'pointer',
            })}
            onclick={() => (billingPeriod = 'annually')}
            type="button"
          >
            연간 결제
            <span
              class={css({
                marginLeft: '8px',
                paddingX: '8px',
                paddingY: '2px',
                fontSize: '12px',
                fontWeight: 'semibold',
                color: 'amber.800',
                backgroundColor: 'amber.100',
                borderRadius: 'full',
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
          transition: '[opacity 0.4s ease-out 0.2s, transform 0.4s ease-out 0.2s]',
          marginTop: '20px',
        })}
      >
        <!-- Basic Plan -->
        <div
          class={css({
            position: 'relative',
            backgroundColor: 'white',
            borderRadius: '[32px]',
            padding: { base: '32px', md: '48px' },
            border: '2px solid',
            borderColor: 'gray.100',
            boxShadow: '[0 1px 3px rgba(0, 0, 0, 0.02)]',
            transition: '[all 0.3s ease]',
            transform: 'translateZ(0)',
            display: 'flex',
            flexDirection: 'column',
            _hover: {
              transform: 'translateY(-8px)',
              boxShadow: '[0 20px 40px rgba(0, 0, 0, 0.08)]',
              borderColor: 'gray.200',
            },
          })}
        >
          <h3
            class={css({
              fontSize: '24px',
              fontWeight: 'bold',
              color: 'gray.900',
              fontFamily: 'Paperlogy',
              marginBottom: '8px',
            })}
          >
            {plans.basic.name}
          </h3>
          <p
            class={css({
              fontSize: '15px',
              color: 'gray.600',
              fontFamily: 'Pretendard',
              marginBottom: '32px',
              lineHeight: '[1.6]',
            })}
          >
            {plans.basic.description}
          </p>

          <div class={css({ marginBottom: '40px' })}>
            <div class={flex({ alignItems: 'center', gap: '8px', height: '[72px]' })}>
              <span class={css({ fontSize: '[48px]', fontWeight: 'extrabold', color: 'gray.900', lineHeight: '[1]' })}>무료</span>
            </div>
          </div>

          <a
            class={css({
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: 'full',
              paddingY: '12px',
              borderRadius: 'full',
              fontSize: '16px',
              fontWeight: 'semibold',
              backgroundColor: 'gray.100',
              color: 'gray.900',
              transition: 'all',
              marginBottom: '32px',
              _hover: {
                backgroundColor: 'gray.200',
              },
            })}
            href={env.PUBLIC_AUTH_URL}
          >
            무료로 시작하기
          </a>

          <div class={flex({ flexDirection: 'column', gap: '16px', flex: '1' })}>
            <p class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'gray.700' })}>포함 사항:</p>
            <ul class={flex({ flexDirection: 'column', gap: '12px' })}>
              {#each plans.basic.features as feature, index (index)}
                <li class={flex({ alignItems: 'flex-start', gap: '12px' })}>
                  <Icon style={css.raw({ color: 'gray.400', flexShrink: 0, marginTop: '2px' })} icon={CheckIcon} size={16} />
                  <span class={css({ fontSize: '14px', color: 'gray.600', lineHeight: '[1.5]' })}>{feature}</span>
                </li>
              {/each}
            </ul>
          </div>
        </div>

        <!-- Full Access Plan -->
        <div
          class={css({
            position: 'relative',
            backgroundColor: 'gray.900',
            borderRadius: '[32px]',
            padding: { base: '32px', md: '48px' },
            border: '2px solid',
            borderColor: 'gray.800',
            boxShadow: '[0 20px 40px rgba(0, 0, 0, 0.15)]',
            transition: '[all 0.3s ease]',
            transform: 'translateZ(0)',
            display: 'flex',
            flexDirection: 'column',
            _hover: {
              transform: 'translateY(-8px)',
              boxShadow: '[0 30px 60px rgba(0, 0, 0, 0.2)]',
            },
            '&::before': {
              content: '""',
              position: 'absolute',
              top: '0',
              left: '0',
              right: '0',
              bottom: '0',
              backgroundImage: 'linear-gradient(to bottom, token(colors.amber.400), transparent 50%)',
              opacity: '[0.08]',
              pointerEvents: 'none',
              borderRadius: '[32px]',
              overflow: 'hidden',
            },
          })}
        >
          {#if plans.full.badge}
            <div
              class={css({
                position: 'absolute',
                top: '-14px',
                left: '[50%]',
                transform: 'translateX(-50%)',
                paddingX: '20px',
                paddingY: '6px',
                fontSize: '12px',
                fontWeight: 'bold',
                color: 'gray.900',
                backgroundColor: 'amber.400',
                borderRadius: 'full',
                letterSpacing: '[0.05em]',
                boxShadow: '[0 4px 12px rgba(251, 191, 36, 0.3)]',
                zIndex: '10',
              })}
            >
              {plans.full.badge}
            </div>
          {/if}

          <h3
            class={css({
              fontSize: '24px',
              fontWeight: 'bold',
              color: 'white',
              fontFamily: 'Paperlogy',
              marginBottom: '8px',
            })}
          >
            {plans.full.name}
          </h3>
          <p
            class={css({
              fontSize: '15px',
              color: 'gray.300',
              fontFamily: 'Pretendard',
              marginBottom: '32px',
              lineHeight: '[1.6]',
            })}
          >
            {plans.full.description}
          </p>

          <div class={css({ marginBottom: '40px' })}>
            <div class={flex({ alignItems: 'baseline', gap: '8px', height: '[72px]' })}>
              <NumberFlow
                class={css({
                  fontSize: '[48px]',
                  fontWeight: 'extrabold',
                  color: 'white',
                  lineHeight: '[1]',
                  fontVariantNumeric: 'tabular-nums',
                })}
                value={currentPrice}
              />
              <div>
                <span class={css({ fontSize: '16px', color: 'gray.400' })}>원 / 월</span>
                <span
                  class={css({
                    fontSize: '12px',
                    color: 'gray.500',
                    opacity: billingPeriod === 'annually' ? '100' : '0',
                    transition: '[opacity 0.2s ease]',
                    marginLeft: '4px',
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
                paddingY: '12px',
                borderRadius: 'full',
                fontSize: '16px',
                fontWeight: 'semibold',
                backgroundColor: 'amber.400',
                color: 'gray.900',
                transition: 'all',
                marginBottom: '32px',
                _hover: {
                  backgroundColor: 'amber.300',
                },
              }),
            )}
            href={env.PUBLIC_AUTH_URL}
          >
            <Icon icon={ZapIcon} size={16} />
            지금 시작하기
            <Icon
              style={css.raw({ _groupHover: { transform: 'translateX(4px)' }, transition: '[transform 0.2s ease]' })}
              icon={ArrowRightIcon}
              size={16}
            />
          </a>

          <div class={flex({ flexDirection: 'column', gap: '16px', flex: '1' })}>
            <p class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'gray.300' })}>제한 없이 모든 기능 사용:</p>
            <ul class={flex({ flexDirection: 'column', gap: '12px' })}>
              {#each plans.full.features as feature, index (index)}
                <li class={flex({ alignItems: 'flex-start', gap: '12px' })}>
                  <Icon style={css.raw({ color: 'amber.400', flexShrink: 0, marginTop: '2px' })} icon={CheckIcon} size={16} />
                  <span class={css({ fontSize: '14px', color: 'gray.400', lineHeight: '[1.5]' })}>{feature}</span>
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
          transform: 'translateY(20px)',
          transition: '[opacity 0.3s ease-out 0.15s, transform 0.3s ease-out 0.15s]',
        })}
      >
        <div
          class={css({
            display: 'inline-flex',
            alignItems: 'center',
            gap: '8px',
            backgroundColor: 'gray.100',
            color: 'gray.700',
            paddingX: '16px',
            paddingY: '6px',
            borderRadius: 'full',
            fontSize: '13px',
            fontWeight: 'semibold',
            marginBottom: '32px',
            letterSpacing: 'wide',
            textTransform: 'uppercase',
          })}
        >
          FAQ
        </div>

        <h2
          class={css({
            fontSize: { base: '[40px]', md: '[56px]' },
            fontWeight: 'extrabold',
            color: 'gray.900',
            textAlign: 'center',
            fontFamily: 'Paperlogy',
            lineHeight: '[1.2]',
            letterSpacing: 'tight',
          })}
        >
          자주 묻는 질문
        </h2>
      </div>

      <div
        bind:this={elements[4]}
        class={css({
          maxWidth: '[800px]',
          marginX: 'auto',
          opacity: '0',
          transform: 'translateY(20px)',
          transition: '[opacity 0.3s ease-out 0.2s, transform 0.3s ease-out 0.2s]',
        })}
      >
        <div
          class={flex({
            flexDirection: 'column',
            gap: '16px',
          })}
        >
          {#each faqs as faq, index (index)}
            <div
              class={css({
                backgroundColor: 'white',
                borderRadius: '[16px]',
                border: '1px solid',
                borderColor: expandedFaqIndex === index ? 'gray.300' : 'gray.200',
                transition: '[border-color 0.15s ease, box-shadow 0.15s ease]',
                overflow: 'hidden',
                boxShadow: expandedFaqIndex === index ? '[0 4px 12px rgba(0, 0, 0, 0.05)]' : undefined,
                _hover: {
                  borderColor: 'gray.300',
                  boxShadow: '[0 4px 12px rgba(0, 0, 0, 0.05)]',
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
                  backgroundColor: 'transparent',
                  border: 'none',
                  transition: '[background-color 0.2s ease]',
                  _hover: {
                    backgroundColor: expandedFaqIndex === index ? 'transparent' : 'gray.50',
                  },
                })}
                onclick={() => toggleFaq(index)}
                type="button"
              >
                <h3
                  class={css({
                    fontSize: '18px',
                    fontWeight: 'semibold',
                    color: 'gray.900',
                    fontFamily: 'Pretendard',
                    lineHeight: '[1.5]',
                  })}
                >
                  {faq.question}
                </h3>
                <Icon
                  style={css.raw({
                    color: 'gray.500',
                    flexShrink: 0,
                    transform: expandedFaqIndex === index ? 'rotate(180deg)' : 'rotate(0deg)',
                    transition: '[transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)]',
                  })}
                  icon={ChevronDownIcon}
                  size={20}
                />
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
                  <p
                    class={css({
                      fontSize: '16px',
                      color: 'gray.600',
                      fontFamily: 'Pretendard',
                      lineHeight: '[1.7]',
                      paddingX: '32px',
                      paddingBottom: '24px',
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
    })}
  >
    <!-- Background pattern -->
    <div
      class={css({
        position: 'absolute',
        inset: '0',
        backgroundImage: 'radial-gradient(circle at 1px 1px, token(colors.gray.800) 1px, transparent 1px)',
        backgroundSize: '[40px 40px]',
        opacity: '[0.5]',
        pointerEvents: 'none',
      })}
    ></div>

    <!-- Gradient glow -->
    <div
      class={css({
        position: 'absolute',
        top: '[50%]',
        left: '[50%]',
        transform: 'translate(-50%, -50%)',
        width: '[800px]',
        height: '[400px]',
        backgroundImage: 'radial-gradient(ellipse, token(colors.amber.500), transparent)',
        opacity: '[0.15]',
        filter: '[blur(100px)]',
        pointerEvents: 'none',
      })}
    ></div>

    <div class={css({ maxWidth: '[1024px]', marginX: 'auto', position: 'relative' })}>
      <div
        bind:this={elements[5]}
        class={center({
          flexDirection: 'column',
          textAlign: 'center',
          opacity: '0',
          transform: 'translateY(20px)',
          transition: '[opacity 0.3s ease-out 0.15s, transform 0.3s ease-out 0.15s]',
        })}
      >
        <div
          class={css({
            display: 'inline-flex',
            alignItems: 'center',
            gap: '8px',
            backgroundColor: 'gray.800',
            color: 'amber.400',
            paddingX: '20px',
            paddingY: '8px',
            borderRadius: 'full',
            fontSize: '14px',
            fontWeight: 'semibold',
            marginBottom: '40px',
            border: '1px solid',
            borderColor: 'gray.700',
            animation: '[shimmer 3s linear infinite]',
            backgroundImage: 'linear-gradient(90deg, transparent, rgba(251, 191, 36, 0.1) 50%, transparent)',
            backgroundSize: '[200% 100%]',
          })}
        >
          <Icon icon={SparklesIcon} size={16} />
          지금 시작하세요
        </div>

        <h2
          class={css({
            fontSize: { base: '[48px]', md: '[64px]' },
            fontWeight: 'extrabold',
            color: 'white',
            fontFamily: 'Paperlogy',
            marginBottom: '24px',
            lineHeight: '[1.1]',
            letterSpacing: 'tight',
          })}
        >
          준비되셨나요?
        </h2>
        <p
          class={css({
            fontSize: { base: '20px', md: '24px' },
            fontWeight: 'normal',
            color: 'gray.300',
            fontFamily: 'Pretendard',
            marginBottom: '48px',
            lineHeight: '[1.6]',
            maxWidth: '[600px]',
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
              gap: '8px',
              paddingX: '32px',
              paddingY: '16px',
              fontSize: '18px',
              fontWeight: 'semibold',
              color: 'gray.900',
              backgroundColor: 'amber.400',
              borderRadius: 'full',
              transition: '[all 0.3s ease]',
              transform: 'scale(1)',
              boxShadow: '[0 8px 24px rgba(251, 191, 36, 0.3)]',
              _hover: {
                backgroundColor: 'amber.300',
                transform: 'scale(1.05)',
                boxShadow: '[0 12px 32px rgba(251, 191, 36, 0.4)]',
              },
            }),
          )}
          href={env.PUBLIC_AUTH_URL}
        >
          무료로 시작하기
          <Icon
            style={css.raw({ _groupHover: { transform: 'translateX(4px)' }, transition: '[transform 0.2s ease]' })}
            icon={ArrowRightIcon}
            size={20}
          />
        </a>
      </div>
    </div>
  </section>
</div>

<style>
  @keyframes -global-float {
    0%,
    100% {
      transform: translateY(0px);
    }
    50% {
      transform: translateY(-20px);
    }
  }

  @keyframes -global-shimmer {
    0% {
      background-position: -200% center;
    }
    100% {
      background-position: 200% center;
    }
  }

  :global(.in-view) {
    opacity: 1 !important;
    transform: translateY(0) !important;
  }
</style>
