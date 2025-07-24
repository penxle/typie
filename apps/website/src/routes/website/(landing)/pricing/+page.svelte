<script lang="ts">
  import NumberFlow from '@number-flow/svelte';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import SparklesIcon from '~icons/lucide/sparkles';
  import ZapIcon from '~icons/lucide/zap';
  import { browser } from '$app/environment';
  import { env } from '$env/dynamic/public';
  import { Helmet, Icon } from '$lib/components';
  import { PLAN_FEATURES } from '$lib/constants';
  import { comma } from '$lib/utils/number';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  let selectedInterval = $state<'monthly' | 'yearly'>('monthly');
  let expandedIndex = $state<number | null>(null);

  const features = {
    basic: PLAN_FEATURES.basic.map((feature) => feature.label),
    full: PLAN_FEATURES.full.map((feature) => feature.label),
  };

  const faqs = [
    {
      question: '유료 플랜에서 무료 플랜으로 다운그레이드하면 기존 데이터는 어떻게 되나요?',
      answer:
        '기존 데이터는 모두 안전하게 보존돼요. 다만 무료 플랜의 제한을 초과한 콘텐츠는 읽기 전용으로 전환되며, 제한 내로 조정하면 다시 편집이 가능해요.',
    },
    {
      question: '언제든지 플랜을 변경할 수 있나요?',
      answer: '네, 언제든지 플랜을 변경할 수 있어요. 변경된 플랜은 다음 결제 주기부터 자동으로 적용돼요.',
    },
    {
      question: '결제 수단은 무엇을 지원하나요?',
      answer: '지금은 국내 신용카드, 체크카드를 지원하고 있어요.',
    },
    {
      question: '환불 정책은 어떻게 되나요?',
      answer: '결제 후 7일 이내에는 전액 환불이 가능해요. 이후에는 남은 기간에 대해 일할 계산해 환불해드려요.',
    },
  ];

  const toggleFaq = (index: number) => {
    expandedIndex = expandedIndex === index ? null : index;
  };
</script>

<Helmet description="언제든 무료로 시작하세요. 필요시 월 4,900원으로 타이피의 모든 기능을 사용할 수 있어요." title="구독 안내" />

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
        'linear-gradient(to bottom, token(colors.white), token(colors.gray.50) 25%, token(colors.gray.50) 75%, token(colors.white)), radial-gradient(circle at 1px 1px, token(colors.gray.200) 1px, transparent 1px)',
      backgroundSize: '[1px, 50px 50px]',
      opacity: '[0.3]',
      pointerEvents: 'none',
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
    })}
  ></div>

  <section
    class={css({
      position: 'relative',
      paddingTop: { sm: '80px', lg: '100px' },
      paddingBottom: { sm: '60px', lg: '80px' },
      paddingX: { sm: '16px', lg: '24px' },
      zIndex: '2',
      opacity: '0',
      transform: { sm: 'translate3d(0, 40px, 0) scale(0.95)', lg: 'translate3d(0, 40px, 0) rotate(-1deg) scale(0.95)' },
      transition: {
        sm: '[opacity 0.4s ease-out, transform 0.4s ease-out]',
        lg: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
      },
      willChange: 'transform, opacity',
      '&.in-view': {
        opacity: '100',
        transform: { sm: 'translate3d(0, 0, 0) scale(1)', lg: 'translate3d(0, 0, 0) rotate(0) scale(1)' },
      },
    })}
    data-observe
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
          transform: { sm: 'scale(0)', lg: 'rotate(-2deg) scale(0)' },
          transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.3s]',
          '.in-view &': {
            transform: { sm: 'scale(1)', lg: 'rotate(-2deg) scale(1)' },
          },
        })}
      >
        <Icon icon={SparklesIcon} size={16} />
        SUBSCRIPTION
      </div>

      <h1
        class={css({
          fontSize: { sm: '[40px]', lg: '[80px]' },
          fontWeight: 'black',
          color: 'gray.900',
          textAlign: 'center',
          fontFamily: 'Paperlogy',
          lineHeight: '[1]',
          marginBottom: '32px',
          textTransform: 'uppercase',
          opacity: '0',
          transform: 'translate3d(0, 20px, 0)',
          transition: {
            sm: '[opacity 0.4s ease-out 0.2s, transform 0.4s ease-out 0.2s]',
            lg: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.4s]',
          },
          willChange: 'transform, opacity',
          '.in-view &': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
      >
        <span
          class={css({
            backgroundColor: 'amber.400',
            paddingX: '20px',
            display: 'inline-block',
            transform: { sm: 'scale(0)', lg: 'rotate(1deg) scale(0)' },
            transition: '[transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) 0.6s]',
            '.in-view &': {
              transform: { sm: 'scale(1)', lg: 'rotate(1deg) scale(1)' },
            },
          })}
        >
          구독
        </span>
        안내
      </h1>
      <p
        class={css({
          fontSize: { sm: '18px', lg: '20px' },
          fontWeight: 'medium',
          color: 'gray.700',
          textAlign: 'center',
          lineHeight: '[1.7]',
          maxWidth: '[700px]',
          marginX: 'auto',
          opacity: '0',
          transform: 'translate3d(0, 10px, 0)',
          transition: {
            sm: '[opacity 0.4s ease-out 0.4s, transform 0.4s ease-out 0.4s]',
            lg: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.7s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.7s]',
          },
          willChange: 'transform, opacity',
          '.in-view &': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
      >
        처음엔 가볍게 시작하고,
        <br />
        필요할 땐 제한 없이 모든 기능을 사용할 수 있어요.
      </p>
    </div>
  </section>

  <section
    class={css({
      position: 'relative',
      paddingTop: '0',
      paddingBottom: { sm: '120px', lg: '160px' },
      paddingX: { sm: '16px', lg: '24px' },
      zIndex: '2',
    })}
  >
    <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
      <div
        class={css({
          marginBottom: '64px',
          display: 'flex',
          justifyContent: 'center',
          opacity: '0',
          transform: { sm: 'translate3d(0, 20px, 0)', lg: 'translate3d(0, 20px, 0) rotate(-1deg)' },
          transition: {
            sm: '[opacity 0.3s ease-out 0.05s, transform 0.3s ease-out 0.05s]',
            lg: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.1s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.1s]',
          },
          willChange: 'transform, opacity',
          '&.in-view': {
            opacity: '100',
            transform: { sm: 'translate3d(0, 0, 0)', lg: 'translate3d(0, 0, 0) rotate(0)' },
          },
        })}
        data-observe
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
              backgroundColor: 'white',
              color: 'gray.900',
              cursor: 'pointer',
              border: 'none',
              position: 'relative',
              zIndex: '1',
              transform: 'scale(1)',
              _hover: {
                backgroundColor: 'gray.100',
              },
              _pressed: {
                backgroundColor: 'gray.900',
                color: 'white',
                zIndex: '2',
                transform: 'scale(1.05)',
                _hover: {
                  backgroundColor: 'gray.800',
                },
              },
            })}
            aria-pressed={selectedInterval === 'monthly'}
            onclick={() => (selectedInterval = 'monthly')}
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
              backgroundColor: 'white',
              color: 'gray.900',
              cursor: 'pointer',
              border: 'none',
              position: 'relative',
              zIndex: '1',
              transform: 'scale(1)',
              _hover: {
                backgroundColor: 'gray.100',
              },
              _pressed: {
                backgroundColor: 'gray.900',
                color: 'white',
                zIndex: '2',
                transform: 'scale(1.05)',
                _hover: {
                  backgroundColor: 'gray.800',
                },
              },
            })}
            aria-pressed={selectedInterval === 'yearly'}
            onclick={() => (selectedInterval = 'yearly')}
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
                transform: { sm: 'rotate(0)', lg: 'rotate(-2deg)' },
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
        class={css({
          display: 'grid',
          gridTemplateColumns: { sm: '1fr', lg: '1fr 1fr' },
          gap: { sm: '24px', lg: '32px' },
          opacity: '0',
          transform: 'translate3d(0, 20px, 0)',
          transition: {
            sm: '[opacity 0.3s ease-out 0.1s, transform 0.3s ease-out 0.1s]',
            lg: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s]',
          },
          willChange: 'transform, opacity',
          marginTop: '20px',
          '&.in-view': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
        data-observe
      >
        <div
          class={css({
            position: 'relative',
            backgroundColor: 'white',
            padding: { sm: '24px', lg: '48px' },
            border: '4px solid',
            borderColor: 'gray.900',
            boxShadow: '[8px 8px 0 0 #000]',
            transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
            transform: { sm: 'rotate(0deg)', lg: 'rotate(-0.5deg)' },
            display: 'flex',
            flexDirection: 'column',
            order: { sm: '2', lg: '1' },
            _hover: {
              transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0deg)' },
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
            타이피 BASIC ACCESS
          </h3>
          <p
            class={css({
              fontSize: '16px',
              color: 'gray.700',
              marginBottom: '32px',
              lineHeight: '[1.6]',
              fontWeight: 'medium',
            })}
          >
            부담 없이, 필요한 만큼만 써보세요
          </p>

          <div
            class={css({
              fontSize: '[56px]',
              fontWeight: 'black',
              color: 'gray.900',
              lineHeight: '[1]',
              fontFamily: 'Paperlogy',
              marginBottom: '40px',
            })}
          >
            무료
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
                  transform: 'translate3d(0, -2px, 0)',
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
              언제든 다음 기능을 이용하세요
            </p>
            <ul class={flex({ flexDirection: 'column', gap: '16px' })}>
              {#each features.basic as feature, index (index)}
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
            padding: { sm: '24px', lg: '48px' },
            border: '4px solid',
            borderColor: 'gray.900',
            boxShadow: '[8px 8px 0 0 #fbbf24]',
            transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
            transform: { sm: 'rotate(0deg)', lg: 'rotate(0.5deg)' },
            display: 'flex',
            flexDirection: 'column',
            order: { sm: '1', lg: '2' },
            _hover: {
              transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0deg)' },
              boxShadow: '[12px 12px 0 0 #fbbf24]',
            },
          })}
        >
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
              transform: { sm: 'rotate(0)', lg: 'rotate(2deg)' },
              boxShadow: '[4px 4px 0 0 #000]',
              zIndex: '10',
            })}
          >
            RECOMMENDED
          </div>

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
            타이피 FULL ACCESS
          </h3>
          <p
            class={css({
              fontSize: '16px',
              color: 'gray.300',
              marginBottom: '32px',
              lineHeight: '[1.6]',
              fontWeight: 'medium',
            })}
          >
            더 많은 도구와 함께, 자유롭게 글을 시작해보세요
          </p>

          <div class={flex({ alignItems: 'baseline', gap: '8px', marginBottom: '12px' })}>
            {#if browser}
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
                value={selectedInterval === 'monthly' ? 4900 : Math.floor(49_000 / 12)}
              />
            {:else}
              <div
                class={css({
                  fontSize: '[56px]',
                  fontWeight: 'black',
                  color: 'amber.400',
                  lineHeight: '[1]',
                  fontVariantNumeric: 'tabular-nums',
                  letterSpacing: '[0.05em]',
                  fontFamily: 'Paperlogy',
                })}
              >
                {selectedInterval === 'monthly' ? 4900 : Math.floor(49_000 / 12)}
              </div>
            {/if}
            <div>
              <span class={css({ fontSize: '18px', color: 'gray.400', fontWeight: 'bold' })}>원 / 월</span>
              <span
                class={css({
                  fontSize: '14px',
                  color: 'gray.500',
                  opacity: selectedInterval === 'yearly' ? '100' : '0',
                  transition: '[opacity 0.2s ease]',
                  marginLeft: '4px',
                  fontWeight: 'medium',
                })}
              >
                (연 {comma(49_000)}원)
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
                  transform: 'translate3d(-2px, -2px, 0)',
                  boxShadow: '[6px 6px 0 0 #000]',
                },
                _active: {
                  transform: 'translate3d(2px, 2px, 0)',
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
                  transform: { sm: 'translate3d(4px, 0, 0)', lg: 'translate3d(4px, 0, 0) rotate(-15deg)' },
                },
              })}
              icon={ArrowRightIcon}
              size={18}
            />
          </a>

          <div class={flex({ flexDirection: 'column', gap: '16px', flex: '1' })}>
            <p class={css({ fontSize: '14px', fontWeight: 'bold', color: 'white', textTransform: 'uppercase', letterSpacing: '[0.05em]' })}>
              제한 없이 모든 기능을 사용하세요
            </p>
            <ul class={flex({ flexDirection: 'column', gap: '16px' })}>
              {#each features.full as feature, index (index)}
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
                      transform: { sm: 'rotate(0)', lg: 'rotate(45deg)' },
                    })}
                  >
                    <Icon
                      style={css.raw({ color: 'gray.900', transform: { sm: 'rotate(0)', lg: 'rotate(-45deg)' } })}
                      icon={CheckIcon}
                      size={12}
                    />
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
      paddingY: { sm: '80px', lg: '120px' },
      paddingX: { sm: '16px', lg: '24px' },
      backgroundColor: 'white',
      zIndex: '2',
    })}
  >
    <div class={css({ maxWidth: '[1024px]', marginX: 'auto' })}>
      <div
        class={center({
          flexDirection: 'column',
          marginBottom: '80px',
          opacity: '0',
          transform: { sm: 'translate3d(0, 20px, 0)', lg: 'translate3d(0, 20px, 0) rotate(-1deg)' },
          transition: {
            sm: '[opacity 0.3s ease-out 0.1s, transform 0.3s ease-out 0.1s]',
            lg: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.15s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.15s]',
          },
          willChange: 'transform, opacity',
          '&.in-view': {
            opacity: '100',
            transform: { sm: 'translate3d(0, 0, 0)', lg: 'translate3d(0, 0, 0) rotate(0)' },
          },
        })}
        data-observe
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
            transform: { sm: 'rotate(0)', lg: 'rotate(-2deg)' },
          })}
        >
          FAQ
        </div>

        <h2
          class={css({
            fontSize: { sm: '[36px]', lg: '[64px]' },
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
              transform: { sm: 'rotate(0)', lg: 'rotate(1deg)' },
            })}
          >
            질문
          </span>
        </h2>
      </div>

      <div
        class={css({
          maxWidth: '[800px]',
          marginX: 'auto',
          opacity: '0',
          transform: 'translate3d(0, 20px, 0)',
          transition: {
            sm: '[opacity 0.3s ease-out 0.1s, transform 0.3s ease-out 0.1s]',
            lg: '[opacity 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s, transform 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.2s]',
          },
          willChange: 'transform, opacity',
          '&.in-view': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
        data-observe
      >
        <div
          class={flex({
            flexDirection: 'column',
            gap: '24px',
          })}
        >
          {#each faqs as faq, index (index)}
            <div
              class={cx(
                'group',
                css({
                  backgroundColor: 'white',
                  border: '4px solid',
                  borderColor: 'gray.900',
                  transition: '[transform 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94), box-shadow 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                  overflow: 'hidden',
                  boxShadow: '[4px 4px 0 0 #000]',
                  transform: 'translate3d(0, 0, 0)',
                  _hover: {
                    transform: 'translate3d(-2px, -2px, 0)',
                    boxShadow: '[8px 8px 0 0 #000]',
                  },
                  _expanded: {
                    boxShadow: '[8px 8px 0 0 #000]',
                    transform: 'translate3d(-2px, -2px, 0)',
                  },
                }),
              )}
              aria-expanded={expandedIndex === index}
            >
              <button
                class={css({
                  width: 'full',
                  paddingX: { sm: '24px', lg: '32px' },
                  paddingY: { sm: '20px', lg: '24px' },
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: '16px',
                  textAlign: 'left',
                  cursor: 'pointer',
                  backgroundColor: 'transparent',
                  border: 'none',
                  transition: '[background-color 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
                  fontSize: { sm: '18px', lg: '20px' },
                  fontWeight: 'bold',
                  color: 'gray.900',
                  lineHeight: '[1.5]',
                  _hover: {
                    backgroundColor: 'gray.50',
                  },
                  _groupExpanded: {
                    backgroundColor: 'amber.50',
                    _hover: {
                      backgroundColor: 'amber.50',
                    },
                  },
                })}
                onclick={() => toggleFaq(index)}
                type="button"
              >
                {faq.question}
                <div
                  class={css({
                    width: '32px',
                    height: '32px',
                    backgroundColor: 'amber.400',
                    border: '3px solid',
                    borderColor: 'gray.900',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                    transform: 'rotate(0deg)',
                    transition: '[transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)]',
                    _groupExpanded: {
                      backgroundColor: 'gray.900',
                      transform: 'rotate(180deg)',
                    },
                  })}
                >
                  <Icon
                    style={css.raw({
                      color: 'gray.900',
                      _groupExpanded: {
                        color: 'white',
                      },
                    })}
                    icon={ChevronDownIcon}
                    size={18}
                  />
                </div>
              </button>

              <div
                class={css({
                  display: 'grid',
                  gridTemplateRows: '0fr',
                  transition: '[grid-template-rows 0.15s cubic-bezier(0.4, 0, 0.2, 1)]',
                  _groupExpanded: {
                    gridTemplateRows: '1fr',
                  },
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
                        lineHeight: '[1.7]',
                        paddingX: { sm: '24px', lg: '32px' },
                        paddingY: { sm: '20px', lg: '24px' },
                        fontWeight: 'medium',
                        opacity: '0',
                        transform: 'translate3d(0, -10px, 0)',
                        transition: '[opacity 0.2s ease-out, transform 0.2s ease-out]',
                        transitionDelay: '0s',
                        _groupExpanded: {
                          opacity: '100',
                          transform: 'translate3d(0, 0, 0)',
                          transitionDelay: '0.05s',
                        },
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
      paddingY: { sm: '120px', lg: '160px' },
      paddingX: { sm: '16px', lg: '24px' },
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
        backgroundImage: 'radial-gradient(circle at 1px 1px, rgba(251, 191, 36, 0.15) 1px, transparent 1px)',
        backgroundSize: '[60px 60px]',
        pointerEvents: 'none',
      })}
    ></div>

    <div
      class={css({
        position: 'absolute',
        top: '[50%]',
        left: '[50%]',
        width: '[800px]',
        height: '[800px]',
        backgroundImage: 'radial-gradient(circle, rgba(251, 191, 36, 0.2), transparent)',
        transform: 'translate3d(-50%, -50%, 0)',
        filter: '[blur(120px)]',
        pointerEvents: 'none',
      })}
    ></div>

    <div class={css({ maxWidth: '[1024px]', marginX: 'auto', position: 'relative' })}>
      <div
        class={center({
          flexDirection: 'column',
          textAlign: 'center',
          opacity: '0',
          transform: { sm: 'translate3d(0, 40px, 0) scale(0.95)', lg: 'translate3d(0, 40px, 0) rotate(-1deg) scale(0.95)' },
          transition: {
            sm: '[opacity 0.4s ease-out 0.1s, transform 0.4s ease-out 0.1s]',
            lg: '[opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.15s, transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94) 0.15s]',
          },
          willChange: 'transform, opacity',
          '&.in-view': {
            opacity: '100',
            transform: { sm: 'translate3d(0, 0, 0) scale(1)', lg: 'translate3d(0, 0, 0) rotate(0) scale(1)' },
          },
        })}
        data-observe
      >
        <div
          class={css({
            display: 'inline-flex',
            alignItems: 'center',
            gap: '8px',
            color: 'amber.400',
            fontSize: '14px',
            fontWeight: 'black',
            marginBottom: '48px',
            letterSpacing: '[0.1em]',
            textTransform: 'uppercase',
            paddingBottom: '4px',
            borderBottom: '4px solid',
            borderColor: 'amber.400',
            transform: { sm: 'rotate(0)', lg: 'rotate(-1deg)' },
          })}
        >
          <Icon icon={SparklesIcon} size={16} />
          TRY NOW
        </div>

        <h2
          class={css({
            fontSize: { sm: '[48px]', lg: '[80px]' },
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
              transform: { sm: 'rotate(0)', lg: 'rotate(2deg)' },
            })}
          >
            ?
          </span>
        </h2>
        <p
          class={css({
            fontSize: { sm: '18px', lg: '24px' },
            fontWeight: 'medium',
            color: 'gray.300',
            marginBottom: '56px',
            lineHeight: '[1.6]',
            maxWidth: '[700px]',
            marginX: 'auto',
          })}
        >
          지금 바로 타이피와 함께 더 나은 글쓰기를 시작하세요.
          <br />
          필요한 건 이미 다 준비되어 있어요.
        </p>

        <a
          class={cx(
            'group',
            css({
              display: 'inline-flex',
              alignItems: 'center',
              gap: '10px',
              paddingX: { sm: '32px', lg: '40px' },
              paddingY: { sm: '16px', lg: '20px' },
              fontSize: { sm: '18px', lg: '20px' },
              fontWeight: 'black',
              color: 'gray.900',
              backgroundColor: 'amber.400',
              border: '4px solid',
              borderColor: 'gray.900',
              transition: '[all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)]',
              transform: { sm: 'rotate(0deg)', lg: 'rotate(-1deg)' },
              textTransform: 'uppercase',
              letterSpacing: '[0.05em]',
              boxShadow: '[8px 8px 0 0 #000]',
              _hover: {
                transform: { sm: 'translate3d(-2px, -2px, 0)', lg: 'translate3d(-4px, -4px, 0) rotate(0)' },
                boxShadow: '[12px 12px 0 0 #000]',
              },
              _active: {
                transform: 'translate3d(4px, 4px, 0)',
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
                transform: { sm: 'translate3d(4px, 0, 0)', lg: 'translate3d(4px, 0, 0) rotate(-15deg)' },
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
