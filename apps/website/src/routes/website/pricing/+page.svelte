<script lang="ts">
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CheckIcon from '~icons/lucide/check';
  import { env } from '$env/dynamic/public';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import Footer from '../landing2/Footer.svelte';
  import Header from '../landing2/Header.svelte';

  let billingPeriod: 'monthly' | 'annually' = $state('monthly');

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
</script>

<div
  class={css({
    width: '[100dvw]',
    minHeight: '[100dvh]',
    color: 'gray.900',
    backgroundColor: 'gray.50',
    wordBreak: 'keep-all',
  })}
>
  <Header />

  <!-- Hero Section -->
  <section class={css({ position: 'relative', paddingY: '120px', paddingX: '24px' })}>
    <div class={center({ flexDirection: 'column', maxWidth: '[1024px]', marginX: 'auto' })}>
      <h1
        class={css({
          fontSize: '[56px]',
          fontWeight: 'extrabold',
          color: 'gray.950',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          lineHeight: '[1.2]',
          marginBottom: '32px',
        })}
      >
        심플하고 투명한 요금제
      </h1>
      <p
        class={css({
          fontSize: '20px',
          fontWeight: 'medium',
          color: 'gray.700',
          textAlign: 'center',
          fontFamily: 'Pretendard',
          lineHeight: '[1.6]',
          maxWidth: '[600px]',
          marginX: 'auto',
        })}
      >
        복잡한 옵션 없이, 필요에 맞는 플랜을 선택하세요.
        <br />
        언제든지 업그레이드하거나 취소할 수 있습니다.
      </p>
    </div>
  </section>

  <!-- Pricing Cards -->
  <section class={css({ position: 'relative', paddingTop: '40px', paddingBottom: '120px', paddingX: '24px', backgroundColor: 'white' })}>
    <div class={css({ maxWidth: '[1024px]', marginX: 'auto' })}>
      <!-- Billing Period Toggle -->
      <div class={center({ marginBottom: '48px' })}>
        <div
          class={flex({
            alignItems: 'center',
            gap: '8px',
            padding: '4px',
            backgroundColor: 'gray.100',
            borderRadius: 'full',
          })}
        >
          <button
            class={css({
              paddingX: '20px',
              paddingY: '8px',
              fontSize: '14px',
              fontWeight: 'medium',
              borderRadius: 'full',
              transition: 'all',
              backgroundColor: billingPeriod === 'monthly' ? 'white' : 'transparent',
              color: billingPeriod === 'monthly' ? 'gray.900' : 'gray.600',
              boxShadow: billingPeriod === 'monthly' ? 'small' : undefined,
              cursor: 'pointer',
            })}
            onclick={() => (billingPeriod = 'monthly')}
            type="button"
          >
            월간 결제
          </button>
          <button
            class={css({
              paddingX: '20px',
              paddingY: '8px',
              fontSize: '14px',
              fontWeight: 'medium',
              borderRadius: 'full',
              transition: 'all',
              backgroundColor: billingPeriod === 'annually' ? 'white' : 'transparent',
              color: billingPeriod === 'annually' ? 'gray.900' : 'gray.600',
              boxShadow: billingPeriod === 'annually' ? 'small' : undefined,
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
                color: 'green.700',
                backgroundColor: 'green.100',
                borderRadius: 'full',
              })}
            >
              2개월 무료
            </span>
          </button>
        </div>
      </div>

      <!-- Plans Grid -->
      <div class={css({ display: 'grid', gridTemplateColumns: { base: '1fr', md: '1fr 1fr' }, gap: '32px' })}>
        <!-- Basic Plan -->
        <div
          class={css({
            position: 'relative',
            backgroundColor: 'white',
            borderRadius: '[24px]',
            padding: '40px',
            borderWidth: '1px',
            borderColor: 'gray.200',
            transition: 'all',
            _hover: {
              borderColor: 'gray.300',
              boxShadow: 'medium',
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

          <div class={flex({ alignItems: 'baseline', gap: '8px', marginBottom: '32px' })}>
            <span class={css({ fontSize: '[40px]', fontWeight: 'bold', color: 'gray.900' })}>무료</span>
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

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
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

        <!-- Full Plan -->
        <div
          class={css({
            position: 'relative',
            backgroundColor: 'dark.gray.950',
            borderRadius: '[24px]',
            padding: '40px',
            borderWidth: '2px',
            borderColor: 'dark.gray.950',
            transition: 'all',
            _hover: {
              boxShadow: 'medium',
            },
          })}
        >
          {#if plans.full.badge}
            <div
              class={css({
                position: 'absolute',
                top: '-12px',
                left: '1/2',
                transform: 'translateX(-50%)',
                paddingX: '16px',
                paddingY: '4px',
                fontSize: '12px',
                fontWeight: 'bold',
                color: 'dark.gray.950',
                backgroundColor: 'amber.400',
                borderRadius: 'full',
                letterSpacing: '[0.05em]',
              })}
            >
              {plans.full.badge}
            </div>
          {/if}

          <h3
            class={css({
              fontSize: '24px',
              fontWeight: 'bold',
              color: 'dark.gray.50',
              fontFamily: 'Paperlogy',
              marginBottom: '8px',
            })}
          >
            {plans.full.name}
          </h3>
          <p
            class={css({
              fontSize: '15px',
              color: 'dark.gray.100',
              fontFamily: 'Pretendard',
              marginBottom: '32px',
              lineHeight: '[1.6]',
            })}
          >
            {plans.full.description}
          </p>

          <div class={flex({ alignItems: 'baseline', gap: '8px', marginBottom: '32px' })}>
            <span class={css({ fontSize: '[40px]', fontWeight: 'bold', color: 'dark.gray.50' })}>
              {billingPeriod === 'monthly' ? plans.full.price.toLocaleString() : Math.floor(plans.full.yearlyPrice / 12).toLocaleString()}
            </span>
            <span class={css({ fontSize: '16px', color: 'dark.gray.100' })}>원 / 월</span>
            {#if billingPeriod === 'annually'}
              <span class={css({ fontSize: '14px', color: 'dark.gray.200' })}>
                (연 {plans.full.yearlyPrice.toLocaleString()}원)
              </span>
            {/if}
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
                color: 'dark.gray.950',
                transition: 'all',
                marginBottom: '32px',
                _hover: {
                  backgroundColor: 'amber.300',
                },
              }),
            )}
            href={env.PUBLIC_AUTH_URL}
          >
            지금 시작하기
            <Icon style={css.raw({ _groupHover: { transform: 'translateX(2px)' } })} icon={ArrowRightIcon} size={16} />
          </a>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <p class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'dark.gray.100' })}>제한 없이 모든 기능 사용:</p>
            <ul class={flex({ flexDirection: 'column', gap: '12px' })}>
              {#each plans.full.features as feature, index (index)}
                <li class={flex({ alignItems: 'flex-start', gap: '12px' })}>
                  <Icon style={css.raw({ color: 'amber.400', flexShrink: 0, marginTop: '2px' })} icon={CheckIcon} size={16} />
                  <span class={css({ fontSize: '14px', color: 'dark.gray.100', lineHeight: '[1.5]' })}>{feature}</span>
                </li>
              {/each}
            </ul>
          </div>
        </div>
      </div>
    </div>
  </section>

  <!-- FAQ Section -->
  <section class={css({ position: 'relative', paddingY: '120px', paddingX: '24px', backgroundColor: 'gray.50' })}>
    <div class={css({ maxWidth: '[1024px]', marginX: 'auto' })}>
      <h2
        class={css({
          fontSize: '[44px]',
          fontWeight: 'extrabold',
          color: 'gray.950',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          marginBottom: '80px',
          lineHeight: '[1.2]',
        })}
      >
        자주 묻는 질문
      </h2>

      <div class={css({ maxWidth: '[800px]', marginX: 'auto' })}>
        <div class={flex({ flexDirection: 'column' })}>
          {#each faqs as faq, index (index)}
            <div
              class={css({
                paddingY: '32px',
                borderBottomWidth: index < faqs.length - 1 ? '1px' : '0',
                borderBottomColor: 'gray.200',
              })}
            >
              <h3
                class={css({
                  fontSize: '18px',
                  fontWeight: 'semibold',
                  color: 'gray.900',
                  fontFamily: 'Pretendard',
                  marginBottom: '12px',
                })}
              >
                {faq.question}
              </h3>
              <p
                class={css({
                  fontSize: '16px',
                  color: 'gray.600',
                  fontFamily: 'Pretendard',
                  lineHeight: '[1.6]',
                })}
              >
                {faq.answer}
              </p>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </section>

  <!-- CTA Section -->
  <section class={css({ position: 'relative', paddingY: '160px', paddingX: '24px', backgroundColor: 'white' })}>
    <div class={css({ maxWidth: '[1024px]', marginX: 'auto' })}>
      <div class={center({ flexDirection: 'column', textAlign: 'center' })}>
        <h2
          class={css({
            fontSize: '[44px]',
            fontWeight: 'extrabold',
            color: 'gray.950',
            fontFamily: 'Paperlogy',
            marginBottom: '24px',
            lineHeight: '[1.2]',
          })}
        >
          준비되셨나요?
        </h2>
        <p
          class={css({
            fontSize: '20px',
            fontWeight: 'medium',
            color: 'gray.600',
            fontFamily: 'Pretendard',
            marginBottom: '40px',
            lineHeight: '[1.6]',
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
              paddingX: '24px',
              paddingY: '12px',
              fontSize: '16px',
              fontWeight: 'semibold',
              color: 'white',
              backgroundColor: 'gray.950',
              borderRadius: 'full',
              transition: 'all',
              _hover: {
                backgroundColor: 'gray.800',
              },
            }),
          )}
          href={env.PUBLIC_AUTH_URL}
        >
          무료로 시작하기
          <Icon style={css.raw({ _groupHover: { transform: 'translateX(2px)' } })} icon={ArrowRightIcon} size={16} />
        </a>
      </div>
    </div>
  </section>

  <Footer />
</div>
