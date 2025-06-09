<script lang="ts">
  import Logo from '$assets/logos/logo.svg?component';
  import { graphql } from '$graphql';
  import { Helmet } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex, grid } from '$styled-system/patterns';
  import SmallStatCard from './SmallStatCard.svelte';
  import StatCard from './StatCard.svelte';

  const query = graphql(`
    query OpenStartupPage_Query {
      stats
    }
  `);

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

  function formatFileSize(bytes: number): string {
    if (bytes >= 1024 * 1024 * 1024) {
      const value = (bytes / 1024 / 1024 / 1024).toFixed(1);
      return value.replaceAll(/(\d)(?=(\d{3})+(?!\d))/g, '$1,') + 'GB';
    }
    const value = (bytes / 1024 / 1024).toFixed(1);
    return value.replaceAll(/(\d)(?=(\d{3})+(?!\d))/g, '$1,') + 'MB';
  }
</script>

<Helmet
  description="타이피는 오픈 스타트업으로 운영됩니다. 사용자 수, 매출, 성장률 등 주요 운영 지표를 공개합니다."
  title="타이피 데이터 대시보드"
/>

<div
  class={css({
    height: '[100dvh]',
    backgroundColor: 'gray.50',
    overflowY: 'auto',
    wordBreak: 'keep-all',
  })}
>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      flexShrink: '0',
      borderBottomWidth: '1px',
      borderBottomColor: 'gray.200',
      paddingX: '20px',
      height: '52px',
      backgroundColor: 'white',
      position: 'sticky',
      top: '0',
      zIndex: '50',
    })}
  >
    <a class={css({ display: 'flex', alignItems: 'center' })} href="/">
      <Logo class={css({ flexShrink: '0', height: '20px' })} />
    </a>
  </div>

  <div
    class={css({
      maxWidth: '[1200px]',
      marginX: 'auto',
      paddingY: { base: '[80px]', md: '[120px]' },
      paddingX: { base: '20px', md: '32px' },
    })}
  >
    <header class={css({ marginBottom: '[100px]' })}>
      <div class={css({ maxWidth: '[900px]' })}>
        <div
          class={css({
            display: 'inline-flex',
            alignItems: 'center',
            backgroundColor: 'gray.100',
            borderRadius: '[20px]',
            paddingX: '16px',
            paddingY: '8px',
            marginBottom: '24px',
            fontSize: '14px',
            fontWeight: 'medium',
            color: 'gray.700',
          })}
        >
          <span
            class={css({
              width: '8px',
              height: '8px',
              backgroundColor: 'green.500',
              borderRadius: 'full',
              marginRight: '8px',
            })}
          ></span>
          타이피 데이터 대시보드
        </div>

        <h1
          class={css({
            fontSize: { base: '[40px]', md: '[52px]', lg: '[64px]' },
            fontWeight: 'extrabold',
            marginBottom: '24px',
            color: 'gray.950',
            letterSpacing: '[-.04em]',
            lineHeight: '[1.2]',
          })}
        >
          가장 투명한
          <br />
          글쓰기 플랫폼
        </h1>

        <p
          class={css({
            fontSize: { base: '18px', md: '20px' },
            fontWeight: 'normal',
            color: 'gray.600',
            marginBottom: '20px',
            lineHeight: '[1.6]',
            maxWidth: '[560px]',
          })}
        >
          타이피는 오픈 스타트업으로 운영됩니다.
          <br />
          사용자 수, 매출, 성장률 등 주요 운영 지표를 공개합니다.
        </p>

        <p
          class={css({
            fontSize: { base: '16px', md: '17px' },
            color: 'gray.500',
            lineHeight: '[1.6]',
            maxWidth: '[520px]',
          })}
        >
          운영 데이터 공개는 정보 비대칭을 줄이기 위한 경영 원칙입니다. 모든 데이터는 자동 수집되며 1시간 단위로 갱신, 원본 상태로
          제공됩니다.
        </p>
      </div>
    </header>

    <section class={css({ marginBottom: '[120px]' })}>
      <div class={css({ marginBottom: '64px' })}>
        <h2
          class={css({
            fontSize: { base: '[28px]', md: '[32px]' },
            fontWeight: 'bold',
            marginBottom: '12px',
            color: 'gray.950',
            letterSpacing: '[-.01em]',
          })}
        >
          핵심 지표
        </h2>
        <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.5]' })}>
          타이피 성장의 핵심을 보여주는 지표들입니다. 사용자 증가, 창작 활동, 구독 수익의 30일 변화 추이를 함께 확인할 수 있습니다.
        </p>
      </div>

      <div class={grid({ columns: { base: 1, md: 2, lg: 3 }, gap: '20px' })}>
        <StatCard
          data={$query.stats.totalUsers.data}
          description="누적 가입자 수"
          title="전체 사용자"
          type="daily"
          value={formatWithUnit($query.stats.totalUsers.current, '명')}
        />

        <StatCard
          data={$query.stats.newSignups.data}
          description="지난 24시간동안 가입한 사용자 수"
          title="신규 사용자"
          type="daily"
          value={formatWithUnit($query.stats.newSignups.current, '명')}
        />

        <StatCard
          data={$query.stats.dailyCharacters.data}
          description="지난 24시간동안 입력된 글자 수"
          title="24시간 입력량"
          type="daily"
          value={formatWithUnit($query.stats.dailyCharacters.current, '자')}
        />

        <StatCard
          data={$query.stats.activeWriters.data}
          description="지난 24시간동안 활동한 사용자 수"
          title="일일 활성 사용자 (DAU)"
          type="daily"
          value={formatWithUnit($query.stats.activeWriters.current, '명')}
        />

        <StatCard
          data={$query.stats.monthlyRecurringRevenue.data}
          description="지난 30일 구독 수익"
          title="월간 반복 수익 (MRR)"
          type="accumulative"
          value={formatWithUnit($query.stats.monthlyRecurringRevenue.current, '원')}
        />

        <StatCard
          data={$query.stats.activeSubscriptions.data}
          description="현재 활성 구독자 수"
          title="활성 구독자"
          type="accumulative"
          value={formatWithUnit($query.stats.activeSubscriptions.current, '명')}
        />
      </div>
    </section>

    <section class={css({ marginBottom: '[120px]' })}>
      <div class={css({ marginBottom: '64px' })}>
        <h2
          class={css({
            fontSize: { base: '[28px]', md: '[32px]' },
            fontWeight: 'bold',
            marginBottom: '12px',
            color: 'gray.950',
            letterSpacing: '[-.01em]',
          })}
        >
          세부 활동 지표
        </h2>
        <p class={css({ fontSize: '16px', color: 'gray.600', lineHeight: '[1.5]' })}>
          사용자들의 글쓰기 활동 패턴을 보여주는 지표들입니다. 글자 수, 포스트 수, 반응 수 등 작성 활동의 양적 측면을 측정합니다.
        </p>
      </div>

      <div class={css({ marginBottom: '48px' })}>
        <h3
          class={css({
            fontSize: '18px',
            fontWeight: 'semibold',
            marginBottom: '20px',
            color: 'gray.900',
          })}
        >
          작성
        </h3>
        <div class={grid({ columns: { base: 1, md: 2, lg: 4 }, gap: '16px' })}>
          <SmallStatCard
            data={$query.stats.totalCharacters.data}
            description="전체 포스트의 총 글자 수"
            title="전체 글자 수"
            value={formatWithUnit($query.stats.totalCharacters.current, '자')}
          />

          <SmallStatCard
            data={$query.stats.totalInputCharacters.data}
            description="누적 입력 글자 수"
            title="누적 입력 글자 수"
            value={formatWithUnit($query.stats.totalInputCharacters.current, '자')}
          />

          <SmallStatCard
            data={$query.stats.totalPosts.data}
            description="누적 작성 포스트 수"
            title="전체 작성 포스트"
            value={formatWithUnit($query.stats.totalPosts.current, '개')}
          />

          <SmallStatCard
            data={$query.stats.newPosts.data}
            description="지난 24시간동안 작성된 포스트 수"
            title="신규 작성 포스트"
            value={formatWithUnit($query.stats.newPosts.current, '개')}
          />

          <SmallStatCard
            data={$query.stats.averagePostLength.data}
            description="포스트당 평균 글자 수"
            title="평균 포스트 길이"
            value={formatWithUnit($query.stats.averagePostLength.current, '자')}
          />
        </div>
      </div>

      <div class={css({ marginBottom: '48px' })}>
        <h3
          class={css({
            fontSize: '18px',
            fontWeight: 'semibold',
            marginBottom: '20px',
            color: 'gray.900',
          })}
        >
          미디어
        </h3>
        <div class={grid({ columns: { base: 1, md: 2, lg: 4 }, gap: '16px' })}>
          <SmallStatCard
            data={$query.stats.totalMedia.data}
            description="누적 업로드 이미지 및 파일 수"
            title="전체 이미지 및 파일"
            value={formatWithUnit($query.stats.totalMedia.current, '개')}
          />

          <SmallStatCard
            data={$query.stats.newMedia.data}
            description="지난 24시간동안 업로드된 이미지 및 파일 수"
            title="신규 이미지 및 파일"
            value={formatWithUnit($query.stats.newMedia.current, '개')}
          />

          <SmallStatCard
            data={$query.stats.totalMediaSize.data}
            description="전체 저장 용량"
            title="전체 이미지 및 파일 용량"
            value={formatFileSize($query.stats.totalMediaSize.current)}
          />
        </div>
      </div>

      <div>
        <h3
          class={css({
            fontSize: '18px',
            fontWeight: 'semibold',
            marginBottom: '20px',
            color: 'gray.900',
          })}
        >
          공유
        </h3>
        <div class={grid({ columns: { base: 1, md: 2, lg: 4 }, gap: '16px' })}>
          <SmallStatCard
            data={$query.stats.unlistedPrivatePostRatio.data}
            description="비공개로 저장된 포스트의 비율"
            title="비공개 비율"
            value={$query.stats.unlistedPrivatePostRatio.current + '%'}
          />

          <SmallStatCard
            data={$query.stats.totalReactions.data}
            description="누적 이모지 반응 수"
            title="전체 이모지 반응"
            value={formatWithUnit($query.stats.totalReactions.current, '개')}
          />

          <SmallStatCard
            data={$query.stats.newReactions.data}
            description="지난 24시간동안 달린 이모지 반응 수"
            title="신규 이모지 반응"
            value={formatWithUnit($query.stats.newReactions.current, '개')}
          />
        </div>
      </div>
    </section>

    <section
      class={css({
        marginBottom: '[120px]',
        paddingY: { base: '80px', md: '120px' },
        backgroundColor: 'gray.900',
        borderRadius: '[24px]',
        color: 'white',
        position: 'relative',
        overflow: 'hidden',
      })}
    >
      <div
        class={css({
          position: 'absolute',
          top: '0',
          left: '0',
          right: '0',
          bottom: '0',
          background: '[radial-gradient(circle at 30% 20%, rgba(255, 255, 255, 0.06) 0%, transparent 50%)]',
          pointerEvents: 'none',
        })}
      ></div>

      <div class={css({ maxWidth: '[1000px]', marginX: 'auto', paddingX: { base: '20px', md: '32px' }, position: 'relative' })}>
        <div class={css({ marginBottom: '80px' })}>
          <h2
            class={css({
              fontSize: { base: '[32px]', md: '[40px]' },
              fontWeight: 'bold',
              marginBottom: '16px',
              letterSpacing: '[-.01em]',
            })}
          >
            누적 성과
          </h2>
          <p class={css({ fontSize: { base: '18px', md: '20px' }, color: 'gray.400', lineHeight: '[1.5]' })}>
            서비스 출시 이후 전체 기간의 주요 성과를 요약한 지표입니다.
          </p>
        </div>

        <div class={grid({ columns: { base: 1, md: 2 }, gap: { base: '24px', md: '32px' }, marginBottom: '80px' })}>
          <div
            class={css({
              padding: { base: '24px', md: '32px' },
              backgroundColor: 'white/5',
              borderRadius: '[16px]',
              border: '1px solid',
              borderColor: 'white/10',
            })}
          >
            <p
              class={css({
                fontSize: { base: '[48px]', md: '[56px]' },
                fontWeight: 'extrabold',
                marginBottom: '12px',
                lineHeight: '[1]',
              })}
            >
              {formatWithUnit($query.stats.serviceDays.current, '일')}
            </p>
            <p class={css({ fontSize: '16px', color: 'gray.300' })}>서비스 운영 일수</p>
            <p class={css({ fontSize: '13px', color: 'gray.500', marginTop: '4px' })}>서비스 출시 이후 현재까지의 운영 기간</p>
          </div>

          <div
            class={css({
              padding: { base: '24px', md: '32px' },
              backgroundColor: 'white/5',
              borderRadius: '[16px]',
              border: '1px solid',
              borderColor: 'white/10',
            })}
          >
            <p
              class={css({
                fontSize: { base: '[48px]', md: '[56px]' },
                fontWeight: 'extrabold',
                marginBottom: '12px',
                lineHeight: '[1]',
              })}
            >
              {formatWithUnit($query.stats.totalPosts.current, '개')}
            </p>
            <p class={css({ fontSize: '16px', color: 'gray.300' })}>누적 포스트 수</p>
            <p class={css({ fontSize: '13px', color: 'gray.500', marginTop: '4px' })}>서비스 전체 기간 동안 작성된 모든 포스트</p>
          </div>

          <div
            class={css({
              padding: { base: '24px', md: '32px' },
              backgroundColor: 'white/5',
              borderRadius: '[16px]',
              border: '1px solid',
              borderColor: 'white/10',
            })}
          >
            <p
              class={css({
                fontSize: { base: '[48px]', md: '[56px]' },
                fontWeight: 'extrabold',
                marginBottom: '12px',
                lineHeight: '[1]',
              })}
            >
              {formatWithUnit($query.stats.totalUsers.current, '명')}
            </p>
            <p class={css({ fontSize: '16px', color: 'gray.300' })}>누적 가입자 수</p>
            <p class={css({ fontSize: '13px', color: 'gray.500', marginTop: '4px' })}>서비스 전체 기간 동안 가입한 모든 사용자</p>
          </div>

          <div
            class={css({
              padding: { base: '24px', md: '32px' },
              backgroundColor: 'white/5',
              borderRadius: '[16px]',
              border: '1px solid',
              borderColor: 'white/10',
            })}
          >
            <p
              class={css({
                fontSize: { base: '[48px]', md: '[56px]' },
                fontWeight: 'extrabold',
                marginBottom: '12px',
                lineHeight: '[1]',
              })}
            >
              {formatWithUnit($query.stats.totalInputCharacters.current, '자')}
            </p>
            <p class={css({ fontSize: '16px', color: 'gray.300' })}>누적 입력 글자 수</p>
            <p class={css({ fontSize: '13px', color: 'gray.500', marginTop: '4px' })}>서비스 전체 기간 동안 입력된 모든 글자</p>
          </div>
        </div>

        <div
          class={css({
            maxWidth: '[700px]',
            marginX: 'auto',
            textAlign: 'center',
            fontSize: '18px',
            lineHeight: '[1.7]',
            color: 'gray.300',
          })}
        >
          <p class={css({ marginBottom: '24px', fontSize: '20px', fontWeight: 'medium', color: 'white' })}>
            운영 데이터 공개는 단순한 정보 전달이 아닌,
            <br />
            신뢰 기반의 서비스 운영을 위한 구조적 원칙입니다.
          </p>
          <p class={css({ marginBottom: '20px' })}>
            모든 지표는 자동화된 시스템을 통해 매일 업데이트됩니다.
            <br />
            내부 의사결정에 사용되는 원본 데이터를 외부에도 동일하게 공개합니다.
          </p>
          <p>
            투명한 정보 공유는 사용자와 서비스 간의 정보 격차를 줄이고
            <br />
            예측 가능한 서비스 환경을 제공합니다.
          </p>
        </div>
      </div>
    </section>

    <section
      class={css({
        maxWidth: '[800px]',
        marginX: 'auto',
        paddingX: { base: '20px', md: '32px' },
        marginBottom: '[120px]',
      })}
    >
      <h2
        class={css({
          fontSize: { base: '[28px]', md: '[32px]' },
          fontWeight: 'bold',
          marginBottom: '48px',
          color: 'gray.950',
          letterSpacing: '[-.01em]',
        })}
      >
        투명성 원칙
      </h2>

      <div
        class={css({
          fontSize: '16px',
          lineHeight: '[1.7]',
          color: 'gray.700',
        })}
      >
        <h3
          class={css({
            marginBottom: '16px',
            fontSize: '20px',
            fontWeight: 'semibold',
            color: 'gray.900',
          })}
        >
          오픈 스타트업이란
        </h3>

        <p class={css({ marginBottom: '24px' })}>
          오픈 스타트업은 기업의 주요 운영 지표를 외부에 공개하는 방식입니다.
          <br />
          매출, 사용자 수, 성장률 등 일반적으로 내부에서만 공유하는 데이터를 외부에도 제공합니다.
        </p>

        <p class={css({ marginBottom: '48px' })}>
          Buffer, Ghost, Cal.com 등의 기존 사례처럼,
          <br />
          정보 비대칭을 줄이고 신뢰를 확보하는 운영 원칙입니다.
        </p>

        <h3
          class={css({
            marginBottom: '16px',
            fontSize: '20px',
            fontWeight: 'semibold',
            color: 'gray.900',
          })}
        >
          마케팅이 아닌 경영 원칙
        </h3>

        <p class={css({ marginBottom: '24px' })}>
          운영 데이터 공개는 마케팅 수단이 아니라,
          <br />
          책임 있는 운영 체계 구축을 위한 방식입니다.
        </p>

        <p class={css({ marginBottom: '64px' })}>
          공개된 지표는 주관이 개입되지 않은 정보로,
          <br />
          서비스 신뢰도와 예측 가능성을 높이는 데 기여합니다.
        </p>

        <div
          class={css({
            backgroundColor: 'gray.100',
            borderRadius: '[16px]',
            padding: '32px',
            textAlign: 'center',
          })}
        >
          <p class={css({ fontSize: '14px', color: 'gray.600', marginBottom: '8px' })}>문의하기</p>
          <a
            class={css({
              fontSize: '18px',
              color: 'gray.900',
              fontWeight: 'semibold',
              textDecoration: 'none',
              _hover: {
                textDecoration: 'underline',
              },
            })}
            href="mailto:hello@penxle.io"
          >
            hello@penxle.io
          </a>
        </div>
      </div>
    </section>
  </div>
</div>
