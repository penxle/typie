<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { grid } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import { onMount } from 'svelte';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { inview } from '../(index)/inview';
  import StatCard from './StatCard.svelte';
  import StatCardSkeleton from './StatCardSkeleton.svelte';

  const query = graphql(`
    query OpenStartupPage_Query @client {
      stats
    }
  `);

  onMount(() => {
    query.load();
  });

  function formatNumber(num: number): string {
    if (num >= 100_000_000) {
      const value = (num / 100_000_000).toFixed(1);
      const formatted = value.endsWith('.0') ? value.slice(0, -2) + '억' : value + '억';
      return formatted.replaceAll(/(\d)(?=(\d{3})+(?!\d))/g, '$1,');
    } else if (num >= 10_000) {
      const value = (num / 10_000).toFixed(1);
      const formatted = value.endsWith('.0') ? value.slice(0, -2) + '만' : value + '만';
      return formatted.replaceAll(/(\d)(?=(\d{3})+(?!\d))/g, '$1,');
    }
    return num.toLocaleString();
  }

  function formatWithUnit(num: number, unit: string): string {
    return formatNumber(num) + unit;
  }
</script>

<Helmet description="타이피의 사용자 수, 매출, 성장률을 실시간으로 확인하세요." title="오픈 대시보드" />

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
        Open Startup
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
        숨기는 건 없습니다.
        <br />
        <span class={css({ color: 'dark.gray.400' })}>숫자로 증명합니다.</span>
      </h1>

      <p
        class={css({
          fontSize: { sm: '16px', lg: '18px' },
          color: 'dark.gray.400',
          lineHeight: '[1.65]',
          maxWidth: '[480px]',
        })}
      >
        매출, 사용자 수, 성장률.
        <br />
        타이피의 모든 운영 지표를 여기서 확인하세요.
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
          Core Metrics
        </span>

        <h2
          class={css({
            fontSize: { sm: '[32px]', lg: '[48px]' },
            fontWeight: 'medium',
            color: 'dark.gray.100',
            lineHeight: '[1.2]',
            letterSpacing: '[-0.02em]',
            fontFamily: 'Paperlogy',
          })}
        >
          지금 이 순간
        </h2>
      </div>

      <div
        class={cx(
          grid({ columns: { sm: 1, md: 2, lg: 3 }, gap: { sm: '16px', lg: '20px' } }),
          css({
            opacity: '0',
            transform: 'translate3d(0, 20px, 0)',
            transition: '[opacity 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s, transform 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s]',
            '&.in-view': {
              opacity: '100',
              transform: 'translate3d(0, 0, 0)',
            },
          }),
        )}
        {@attach inview}
      >
        {#if $query}
          <StatCard
            data={$query.stats.usersTotal.data}
            description="가입한 전체 사용자 수"
            title="전체 사용자"
            type="accumulative"
            value={formatWithUnit($query.stats.usersTotal.current, '명')}
          />

          <StatCard
            data={$query.stats.usersNew.data}
            description="오늘 새로 합류한 사용자"
            title="신규 가입"
            type="daily"
            value={formatWithUnit($query.stats.usersNew.current, '명')}
          />

          <StatCard
            data={$query.stats.charactersDaily.data}
            description="오늘 하루 동안 쓰인 글자"
            title="오늘의 글자 수"
            type="daily"
            value={formatWithUnit($query.stats.charactersDaily.current, '자')}
          />

          <StatCard
            data={$query.stats.usersActive.data}
            description="오늘 글을 쓴 사용자"
            title="일일 활성 사용자"
            type="daily"
            value={formatWithUnit($query.stats.usersActive.current, '명')}
          />

          <StatCard
            data={$query.stats.subscriptionsRevenue.data}
            description="이번 달 구독 매출"
            title="월 매출 (MRR)"
            type="accumulative"
            value={formatWithUnit($query.stats.subscriptionsRevenue.current, '원')}
          />

          <StatCard
            data={$query.stats.subscriptionsActive.data}
            description="유료 플랜 사용 중"
            title="유료 구독자"
            type="accumulative"
            value={formatWithUnit($query.stats.subscriptionsActive.current, '명')}
          />
        {:else}
          {#each Array.from({ length: 6 }, (_, i) => i) as i (i)}
            <StatCardSkeleton />
          {/each}
        {/if}
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
          Lifetime Stats
        </span>

        <h2
          class={css({
            fontSize: { sm: '[32px]', lg: '[48px]' },
            fontWeight: 'medium',
            color: 'dark.gray.100',
            lineHeight: '[1.2]',
            letterSpacing: '[-0.02em]',
            fontFamily: 'Paperlogy',
          })}
        >
          지금까지 쌓인 것들
        </h2>
      </div>

      <div
        class={cx(
          grid({ columns: { sm: 1, md: 2 }, gap: '0' }),
          css({
            borderTopWidth: '1px',
            borderTopColor: 'dark.gray.900',
            opacity: '0',
            transform: 'translate3d(0, 20px, 0)',
            transition: '[opacity 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s, transform 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.15s]',
            '&.in-view': {
              opacity: '100',
              transform: 'translate3d(0, 0, 0)',
            },
          }),
        )}
        {@attach inview}
      >
        {#if $query}
          <div
            class={css({
              paddingY: { sm: '32px', lg: '48px' },
              paddingRight: { sm: '0', lg: '48px' },
              borderBottomWidth: '1px',
              borderBottomColor: 'dark.gray.900',
              borderRightWidth: { sm: '0', lg: '1px' },
              borderRightColor: 'dark.gray.900',
            })}
          >
            <span
              class={css({
                display: 'block',
                fontSize: '[11px]',
                fontFamily: 'mono',
                color: 'dark.gray.600',
                letterSpacing: '[0.1em]',
                textTransform: 'uppercase',
                marginBottom: '16px',
              })}
            >
              Days Running
            </span>
            <p
              class={css({
                fontSize: { sm: '[48px]', lg: '[64px]' },
                fontWeight: 'medium',
                color: 'dark.gray.100',
                lineHeight: '[1]',
                fontFamily: 'Paperlogy',
                marginBottom: '12px',
              })}
            >
              {formatWithUnit($query.stats.systemServiceDays.current, '일')}
            </p>
            <p class={css({ fontSize: '14px', color: 'dark.gray.500' })}>첫 번째 사용자가 가입한 날부터</p>
          </div>

          <div
            class={css({
              paddingY: { sm: '32px', lg: '48px' },
              paddingLeft: { sm: '0', lg: '48px' },
              borderBottomWidth: '1px',
              borderBottomColor: 'dark.gray.900',
            })}
          >
            <span
              class={css({
                display: 'block',
                fontSize: '[11px]',
                fontFamily: 'mono',
                color: 'dark.gray.600',
                letterSpacing: '[0.1em]',
                textTransform: 'uppercase',
                marginBottom: '16px',
              })}
            >
              Total Posts
            </span>
            <p
              class={css({
                fontSize: { sm: '[48px]', lg: '[64px]' },
                fontWeight: 'medium',
                color: 'dark.gray.100',
                lineHeight: '[1]',
                fontFamily: 'Paperlogy',
                marginBottom: '12px',
              })}
            >
              {formatWithUnit($query.stats.postsTotal.current, '개')}
            </p>
            <p class={css({ fontSize: '14px', color: 'dark.gray.500' })}>타이피에서 작성된 글</p>
          </div>

          <div
            class={css({
              paddingY: { sm: '32px', lg: '48px' },
              paddingRight: { sm: '0', lg: '48px' },
              borderBottomWidth: { sm: '1px', md: '0' },
              borderBottomColor: 'dark.gray.900',
              borderRightWidth: { sm: '0', lg: '1px' },
              borderRightColor: 'dark.gray.900',
            })}
          >
            <span
              class={css({
                display: 'block',
                fontSize: '[11px]',
                fontFamily: 'mono',
                color: 'dark.gray.600',
                letterSpacing: '[0.1em]',
                textTransform: 'uppercase',
                marginBottom: '16px',
              })}
            >
              Total Users
            </span>
            <p
              class={css({
                fontSize: { sm: '[48px]', lg: '[64px]' },
                fontWeight: 'medium',
                color: 'dark.gray.100',
                lineHeight: '[1]',
                fontFamily: 'Paperlogy',
                marginBottom: '12px',
              })}
            >
              {formatWithUnit($query.stats.usersTotal.current, '명')}
            </p>
            <p class={css({ fontSize: '14px', color: 'dark.gray.500' })}>타이피와 함께하는 사용자</p>
          </div>

          <div
            class={css({
              paddingY: { sm: '32px', lg: '48px' },
              paddingLeft: { sm: '0', lg: '48px' },
            })}
          >
            <span
              class={css({
                display: 'block',
                fontSize: '[11px]',
                fontFamily: 'mono',
                color: 'dark.gray.600',
                letterSpacing: '[0.1em]',
                textTransform: 'uppercase',
                marginBottom: '16px',
              })}
            >
              Total Characters
            </span>
            <p
              class={css({
                fontSize: { sm: '[48px]', lg: '[64px]' },
                fontWeight: 'medium',
                color: 'dark.gray.100',
                lineHeight: '[1]',
                fontFamily: 'Paperlogy',
                marginBottom: '12px',
              })}
            >
              {formatWithUnit($query.stats.charactersInput.current, '자')}
            </p>
            <p class={css({ fontSize: '14px', color: 'dark.gray.500' })}>타이피에서 입력된 모든 글자</p>
          </div>
        {:else}
          {#each Array.from({ length: 4 }, (_, i) => i) as i (i)}
            <div
              class={css({
                paddingY: { sm: '32px', lg: '48px' },
                paddingRight: { sm: '0', lg: i % 2 === 0 ? '48px' : '0' },
                paddingLeft: { sm: '0', lg: i % 2 === 1 ? '48px' : '0' },
                borderBottomWidth: i < 2 ? '1px' : { sm: i === 2 ? '1px' : '0', md: '0' },
                borderBottomColor: 'dark.gray.900',
                borderRightWidth: { sm: '0', lg: i % 2 === 0 ? '1px' : '0' },
                borderRightColor: 'dark.gray.900',
              })}
            >
              <div
                class={css({
                  width: '[80px]',
                  height: '11px',
                  backgroundColor: 'dark.gray.800',
                  borderRadius: '2px',
                  marginBottom: '16px',
                  animation: 'pulse 1.5s ease-in-out infinite',
                })}
              ></div>
              <div
                class={css({
                  width: '[160px]',
                  height: { sm: '[48px]', lg: '[64px]' },
                  backgroundColor: 'dark.gray.900',
                  borderRadius: '4px',
                  marginBottom: '12px',
                  animation: 'pulse 1.5s ease-in-out infinite',
                })}
              ></div>
              <div
                class={css({
                  width: '[200px]',
                  height: '14px',
                  backgroundColor: 'dark.gray.800',
                  borderRadius: '2px',
                  animation: 'pulse 1.5s ease-in-out infinite',
                })}
              ></div>
            </div>
          {/each}
        {/if}
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
          gridTemplateColumns: { sm: '1fr', lg: '[1fr 1fr]' },
          gap: { sm: '48px', lg: '80px' },
          alignItems: 'start',
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
            Why Open
          </span>

          <h2
            class={css({
              fontSize: { sm: '[32px]', lg: '[48px]' },
              fontWeight: 'medium',
              color: 'dark.gray.100',
              lineHeight: '[1.2]',
              letterSpacing: '[-0.02em]',
              fontFamily: 'Paperlogy',
              marginBottom: '24px',
            })}
          >
            왜 공개하나요?
          </h2>

          <p
            class={css({
              fontSize: { sm: '16px', lg: '18px' },
              color: 'dark.gray.400',
              lineHeight: '[1.65]',
            })}
          >
            좋은 서비스는 믿을 수 있어야 합니다.
            <br />
            믿음은 투명함에서 시작됩니다.
          </p>
        </div>

        <div
          class={css({
            display: 'flex',
            flexDirection: 'column',
            gap: '32px',
          })}
        >
          <div>
            <h3
              class={css({
                fontSize: { sm: '18px', lg: '20px' },
                fontWeight: 'medium',
                color: 'dark.gray.100',
                marginBottom: '12px',
                fontFamily: 'Paperlogy',
              })}
            >
              같은 숫자를 봅니다
            </h3>
            <p
              class={css({
                fontSize: '15px',
                color: 'dark.gray.400',
                lineHeight: '[1.65]',
              })}
            >
              이 페이지의 모든 지표는 내부 대시보드와 동일합니다. 경영진이 보는 숫자, 사용자가 보는 숫자. 다르지 않습니다.
            </p>
          </div>

          <div>
            <h3
              class={css({
                fontSize: { sm: '18px', lg: '20px' },
                fontWeight: 'medium',
                color: 'dark.gray.100',
                marginBottom: '12px',
                fontFamily: 'Paperlogy',
              })}
            >
              가공하지 않습니다
            </h3>
            <p
              class={css({
                fontSize: '15px',
                color: 'dark.gray.400',
                lineHeight: '[1.65]',
              })}
            >
              좋아 보이는 숫자만 골라 보여주지 않습니다. 성장이 멈춘 날도, 사용자가 떠난 날도 그대로 기록됩니다.
            </p>
          </div>

          <div>
            <h3
              class={css({
                fontSize: { sm: '18px', lg: '20px' },
                fontWeight: 'medium',
                color: 'dark.gray.100',
                marginBottom: '12px',
                fontFamily: 'Paperlogy',
              })}
            >
              1시간마다 갱신됩니다
            </h3>
            <p
              class={css({
                fontSize: '15px',
                color: 'dark.gray.400',
                lineHeight: '[1.65]',
              })}
            >
              모든 지표는 자동으로 수집되고 갱신됩니다. 사람 손을 거치지 않아 항상 정확합니다.
            </p>
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
            글쓰기, 시작해볼까요?
          </h2>

          <p
            class={css({
              fontSize: { sm: '16px', lg: '18px' },
              color: 'dark.gray.400',
              lineHeight: '[1.65]',
              maxWidth: '[400px]',
            })}
          >
            숫자로 신뢰를 증명하는 플랫폼.
            <br />
            무료로 시작하세요.
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
          타이피 시작하기
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
