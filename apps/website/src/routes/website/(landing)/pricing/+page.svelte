<script lang="ts">
  import NumberFlow from '@number-flow/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import { PLAN_FEATURES } from '@typie/ui/constants';
  import { comma } from '@typie/ui/utils';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import { browser } from '$app/environment';
  import { page } from '$app/state';
  import { inview } from '../(index)/inview';

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

<Helmet description="무료로 시작하고, 필요할 때 업그레이드하세요. 월 4,900원으로 모든 기능을 제한 없이 쓸 수 있어요." title="구독 안내" />

<div
  class={css({
    position: 'relative',
    minHeight: '[100vh]',
    backgroundColor: 'dark.gray.950',
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

  <section
    class={css({
      position: 'relative',
      paddingTop: { sm: '100px', lg: '140px' },
      paddingBottom: { sm: '60px', lg: '80px' },
      paddingX: { sm: '24px', lg: '80px' },
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
    <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
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
        Pricing
      </span>

      <h1
        class={css({
          fontSize: { sm: '[36px]', lg: '[56px]' },
          fontWeight: 'medium',
          color: 'dark.gray.100',
          lineHeight: '[1.2]',
          letterSpacing: '[-0.02em]',
          fontFamily: 'Paperlogy',
          marginBottom: '20px',
        })}
      >
        일단 써보세요.
        <br />
        <span class={css({ color: 'dark.gray.400' })}>결제는 나중에.</span>
      </h1>

      <p
        class={css({
          fontSize: { sm: '16px', lg: '18px' },
          color: 'dark.gray.400',
          lineHeight: '[1.65]',
          maxWidth: '[400px]',
        })}
      >
        무료로 충분히 써보고, 마음에 들면 업그레이드하세요.
      </p>
    </div>
  </section>

  <section
    class={css({
      position: 'relative',
      paddingBottom: { sm: '80px', lg: '120px' },
      paddingX: { sm: '24px', lg: '80px' },
    })}
  >
    <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
      <div
        class={css({
          display: 'flex',
          justifyContent: 'flex-end',
          opacity: '0',
          transform: 'translate3d(0, 20px, 0)',
          transition: '[opacity 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s, transform 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s]',
          '&.in-view': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
        {@attach inview}
      >
        <div
          class={css({
            display: 'inline-flex',
            alignItems: 'center',
            gap: '24px',
          })}
        >
          <button
            class={css({
              position: 'relative',
              fontSize: '15px',
              fontWeight: 'medium',
              transition: '[all 0.2s ease-out]',
              backgroundColor: 'transparent',
              color: 'dark.gray.500',
              cursor: 'pointer',
              border: 'none',
              padding: '0',
              paddingBottom: '16px',
              _hover: {
                color: 'dark.gray.300',
              },
              _pressed: {
                color: 'dark.gray.100',
                _after: {
                  content: '""',
                  position: 'absolute',
                  left: '0',
                  right: '0',
                  bottom: '-1px',
                  height: '2px',
                  backgroundColor: 'dark.gray.100',
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
              position: 'relative',
              fontSize: '15px',
              fontWeight: 'medium',
              transition: '[all 0.2s ease-out]',
              backgroundColor: 'transparent',
              color: 'dark.gray.500',
              cursor: 'pointer',
              border: 'none',
              padding: '0',
              paddingBottom: '16px',
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              _hover: {
                color: 'dark.gray.300',
              },
              _pressed: {
                color: 'dark.gray.100',
                _after: {
                  content: '""',
                  position: 'absolute',
                  left: '0',
                  right: '0',
                  bottom: '-1px',
                  height: '2px',
                  backgroundColor: 'dark.gray.100',
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
                fontSize: '11px',
                fontFamily: 'mono',
                fontWeight: 'medium',
                color: 'dark.brand.300',
                letterSpacing: '[0.02em]',
              })}
            >
              −17%
            </span>
          </button>
        </div>
      </div>

      <div
        class={css({
          display: 'grid',
          gridTemplateColumns: { sm: '1fr', lg: '1fr 1fr' },
          gap: '0',
          borderTopWidth: '1px',
          borderTopColor: 'dark.gray.900',
          opacity: '0',
          transform: 'translate3d(0, 20px, 0)',
          transition: '[opacity 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.2s, transform 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.2s]',
          '&.in-view': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
        {@attach inview}
      >
        <div
          class={css({
            paddingY: { sm: '32px', lg: '48px' },
            paddingX: { sm: '0', lg: '48px' },
            paddingLeft: { lg: '0' },
            borderBottomWidth: { sm: '1px', lg: '0' },
            borderBottomColor: 'dark.gray.900',
            borderRightWidth: { sm: '0', lg: '1px' },
            borderRightColor: 'dark.gray.900',
            display: 'flex',
            flexDirection: 'column',
            order: { sm: '2', lg: '1' },
          })}
        >
          <div class={css({ marginBottom: '20px' })}>
            <span
              class={css({
                display: 'block',
                fontSize: '11px',
                fontFamily: 'mono',
                color: 'transparent',
                letterSpacing: '[0.1em]',
                textTransform: 'uppercase',
                marginBottom: '6px',
                visibility: 'hidden',
              })}
            >
              Placeholder
            </span>
            <span
              class={css({
                fontSize: '14px',
                fontFamily: 'mono',
                fontWeight: 'medium',
                color: 'dark.gray.400',
                letterSpacing: '[0.1em]',
                textTransform: 'uppercase',
              })}
            >
              Basic
            </span>
          </div>

          <div class={flex({ alignItems: 'baseline', gap: '8px', marginBottom: '8px', height: { sm: '[56px]', lg: '[64px]' } })}>
            <span
              class={css({
                fontSize: { sm: '[40px]', lg: '[48px]' },
                fontWeight: 'medium',
                color: 'dark.gray.100',
                lineHeight: '[1]',
                fontFamily: 'Paperlogy',
              })}
            >
              무료
            </span>
            <span class={css({ fontSize: '15px', color: 'transparent', visibility: 'hidden' })}>원 / 월</span>
          </div>

          <p
            class={css({
              fontSize: '14px',
              color: 'dark.gray.500',
              marginBottom: '24px',
              height: '20px',
            })}
          ></p>

          <p
            class={css({
              fontSize: '15px',
              color: 'dark.gray.400',
              marginBottom: '32px',
              lineHeight: '[1.65]',
            })}
          >
            핵심 기능만으로 가볍게 시작하세요
          </p>

          <a
            class={css({
              display: 'inline-flex',
              alignItems: 'center',
              justifyContent: 'center',
              paddingX: '24px',
              paddingY: '14px',
              fontSize: '15px',
              fontWeight: 'medium',
              borderWidth: '1px',
              borderColor: 'dark.gray.700',
              color: 'dark.gray.200',
              transition: '[all 0.2s ease-out]',
              marginBottom: '32px',
              _hover: {
                borderColor: 'dark.gray.600',
                backgroundColor: 'dark.gray.900',
              },
            })}
            href={page.data.startUrl}
          >
            무료로 시작하기
          </a>

          <div class={flex({ flexDirection: 'column', gap: '16px', flex: '1' })}>
            <p
              class={css({
                fontSize: '12px',
                fontFamily: 'mono',
                color: 'dark.gray.500',
                letterSpacing: '[0.05em]',
                textTransform: 'uppercase',
              })}
            >
              Includes
            </p>
            <ul class={flex({ flexDirection: 'column', gap: '12px' })}>
              {#each features.basic as feature, index (index)}
                <li class={flex({ alignItems: 'flex-start', gap: '12px' })}>
                  <Icon style={css.raw({ color: 'dark.gray.600', marginTop: '4px' })} icon={CheckIcon} size={14} />
                  <span class={css({ fontSize: '15px', color: 'dark.gray.300', lineHeight: '[1.65]' })}>{feature}</span>
                </li>
              {/each}
            </ul>
          </div>
        </div>

        <div
          class={css({
            paddingY: { sm: '32px', lg: '48px' },
            paddingX: { sm: '0', lg: '48px' },
            paddingRight: { lg: '0' },
            display: 'flex',
            flexDirection: 'column',
            order: { sm: '1', lg: '2' },
          })}
        >
          <div class={css({ marginBottom: '20px' })}>
            <span
              class={css({
                display: 'block',
                fontSize: '11px',
                fontFamily: 'mono',
                color: 'dark.brand.400',
                letterSpacing: '[0.1em]',
                textTransform: 'uppercase',
                marginBottom: '6px',
              })}
            >
              Recommended
            </span>
            <span
              class={css({
                fontSize: '14px',
                fontFamily: 'mono',
                fontWeight: 'medium',
                color: 'dark.gray.400',
                letterSpacing: '[0.1em]',
                textTransform: 'uppercase',
              })}
            >
              Full Access
            </span>
          </div>

          <div class={flex({ alignItems: 'baseline', gap: '8px', marginBottom: '8px', height: { sm: '[56px]', lg: '[64px]' } })}>
            {#if browser}
              <NumberFlow
                class={css({
                  fontSize: { sm: '[40px]', lg: '[48px]' },
                  fontWeight: 'medium',
                  color: 'dark.gray.100',
                  lineHeight: '[1]',
                  fontVariantNumeric: 'tabular-nums',
                  fontFamily: 'Paperlogy',
                })}
                value={selectedInterval === 'monthly' ? 4900 : Math.floor(49_000 / 12)}
              />
            {:else}
              <span
                class={css({
                  fontSize: { sm: '[40px]', lg: '[48px]' },
                  fontWeight: 'medium',
                  color: 'dark.gray.100',
                  lineHeight: '[1]',
                  fontVariantNumeric: 'tabular-nums',
                  fontFamily: 'Paperlogy',
                })}
              >
                {selectedInterval === 'monthly' ? 4900 : Math.floor(49_000 / 12)}
              </span>
            {/if}
            <span class={css({ fontSize: '15px', color: 'dark.gray.500' })}>원 / 월</span>
          </div>

          <p
            class={css({
              fontSize: '14px',
              color: 'dark.gray.500',
              marginBottom: '24px',
              height: '20px',
            })}
          >
            {#if selectedInterval === 'yearly'}
              연 {comma(49_000)}원 결제
            {/if}
          </p>

          <p
            class={css({
              fontSize: '15px',
              color: 'dark.gray.400',
              marginBottom: '32px',
              lineHeight: '[1.65]',
            })}
          >
            제한 없이 모든 기능을 사용하세요
          </p>

          <a
            class={cx(
              'group',
              css({
                display: 'inline-flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '10px',
                paddingX: '24px',
                paddingY: '14px',
                fontSize: '15px',
                fontWeight: 'medium',
                backgroundColor: 'dark.brand.300',
                borderWidth: '1px',
                borderColor: 'dark.brand.300',
                color: 'dark.gray.950',
                transition: '[all 0.2s ease-out]',
                marginBottom: '32px',
                _hover: {
                  backgroundColor: 'dark.brand.200',
                },
              }),
            )}
            href={page.data.startUrl}
          >
            지금 시작하기
            <Icon
              style={css.raw({
                transition: '[transform 0.2s ease-out]',
                _groupHover: {
                  transform: 'translateX(4px)',
                },
              })}
              icon={ArrowRightIcon}
              size={16}
            />
          </a>

          <div class={flex({ flexDirection: 'column', gap: '16px', flex: '1' })}>
            <p
              class={css({
                fontSize: '12px',
                fontFamily: 'mono',
                color: 'dark.gray.500',
                letterSpacing: '[0.05em]',
                textTransform: 'uppercase',
              })}
            >
              Everything in Basic, plus
            </p>
            <ul class={flex({ flexDirection: 'column', gap: '12px' })}>
              {#each features.full as feature, index (index)}
                <li class={flex({ alignItems: 'flex-start', gap: '12px' })}>
                  <Icon style={css.raw({ color: 'dark.brand.400', marginTop: '4px' })} icon={CheckIcon} size={14} />
                  <span class={css({ fontSize: '15px', color: 'dark.gray.300', lineHeight: '[1.65]' })}>{feature}</span>
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
      paddingX: { sm: '24px', lg: '80px' },
      borderTopWidth: '1px',
      borderTopColor: 'dark.gray.900',
      borderBottomWidth: '1px',
      borderBottomColor: 'dark.gray.900',
    })}
  >
    <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
      <div
        class={css({
          marginBottom: { sm: '48px', lg: '64px' },
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
          FAQ
        </span>

        <h2
          class={css({
            fontSize: { sm: '[32px]', lg: '[48px]' },
            fontWeight: 'medium',
            color: 'dark.gray.100',
            fontFamily: 'Paperlogy',
            lineHeight: '[1.2]',
            letterSpacing: '[-0.02em]',
          })}
        >
          자주 묻는 질문
        </h2>
      </div>

      <div
        class={css({
          opacity: '0',
          transform: 'translate3d(0, 20px, 0)',
          transition: '[opacity 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s, transform 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s]',
          '&.in-view': {
            opacity: '100',
            transform: 'translate3d(0, 0, 0)',
          },
        })}
        {@attach inview}
      >
        <div class={flex({ flexDirection: 'column', gap: '0' })}>
          {#each faqs as faq, index (index)}
            <div
              class={cx(
                'group',
                css({
                  borderBottomWidth: '1px',
                  borderBottomColor: 'dark.gray.900',
                }),
              )}
              aria-expanded={expandedIndex === index}
            >
              <button
                class={css({
                  width: 'full',
                  paddingY: { sm: '20px', lg: '24px' },
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: '16px',
                  textAlign: 'left',
                  cursor: 'pointer',
                  backgroundColor: 'transparent',
                  border: 'none',
                  fontSize: { sm: '16px', lg: '17px' },
                  fontWeight: 'medium',
                  color: 'dark.gray.200',
                  lineHeight: '[1.5]',
                  transition: '[color 0.2s ease-out]',
                  _hover: {
                    color: 'dark.gray.100',
                  },
                })}
                onclick={() => toggleFaq(index)}
                type="button"
              >
                {faq.question}
                <Icon
                  style={css.raw({
                    color: 'dark.gray.500',
                    flexShrink: 0,
                    transition: '[transform 0.2s ease-out]',
                    _groupExpanded: {
                      transform: 'rotate(180deg)',
                    },
                  })}
                  icon={ChevronDownIcon}
                  size={18}
                />
              </button>

              <div
                class={css({
                  display: 'grid',
                  gridTemplateRows: '0fr',
                  transition: '[grid-template-rows 0.2s ease-out]',
                  _groupExpanded: {
                    gridTemplateRows: '1fr',
                  },
                })}
              >
                <div class={css({ overflow: 'hidden' })}>
                  <p
                    class={css({
                      paddingBottom: { sm: '20px', lg: '24px' },
                      fontSize: '15px',
                      color: 'dark.gray.400',
                      lineHeight: '[1.65]',
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
      paddingY: { sm: '80px', lg: '120px' },
      paddingX: { sm: '24px', lg: '80px' },
    })}
  >
    <div class={css({ maxWidth: '[1200px]', marginX: 'auto' })}>
      <div
        class={css({
          display: 'grid',
          gridTemplateColumns: { sm: '1fr', lg: '[1fr auto]' },
          gap: { sm: '32px', lg: '80px' },
          alignItems: 'end',
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
        <div>
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
            Get Started
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
            오늘부터 시작하세요.
          </h2>

          <p
            class={css({
              fontSize: { sm: '16px', lg: '18px' },
              color: 'dark.gray.400',
              lineHeight: '[1.65]',
              maxWidth: '[400px]',
            })}
          >
            무료 플랜으로 먼저 경험해보세요.
          </p>
        </div>

        <a
          class={cx(
            'group',
            css({
              display: 'inline-flex',
              alignItems: 'center',
              gap: '12px',
              paddingX: '28px',
              paddingY: '16px',
              fontSize: '15px',
              fontWeight: 'semibold',
              color: 'dark.gray.950',
              backgroundColor: 'dark.brand.300',
              transition: '[all 0.2s ease-out]',
              _hover: {
                backgroundColor: 'dark.brand.200',
              },
            }),
          )}
          href={page.data.startUrl}
        >
          무료로 시작하기
          <Icon
            style={css.raw({
              transition: '[transform 0.2s ease-out]',
              _groupHover: {
                transform: 'translateX(4px)',
              },
            })}
            icon={ArrowRightIcon}
            size={16}
          />
        </a>
      </div>
    </div>
  </section>
</div>
