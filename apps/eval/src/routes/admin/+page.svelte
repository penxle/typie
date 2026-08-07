<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import IconChevronLeft from '~icons/lucide/chevron-left';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { formatKrw } from '$lib/feedback/pricing.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const STATUS_LABELS = { running: '진행 중', completed: '완료', failed: '실패', canceled: '중단됨' };

  const num = (value: number) => value.toLocaleString('ko-KR');

  const badgeRecipe = cva({
    base: {
      display: 'inline-flex',
      alignItems: 'center',
      flexShrink: '0',
      paddingX: '8px',
      paddingY: '2px',
      borderRadius: 'full',
      fontSize: '11px',
      fontWeight: 'semibold',
    },
    variants: {
      status: {
        running: { backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
        completed: { backgroundColor: 'accent.success.subtle', color: 'text.success' },
        failed: { backgroundColor: 'accent.danger.subtle', color: 'text.danger' },
        canceled: { backgroundColor: 'surface.muted', color: 'text.faint' },
      },
    },
  });

  const headStyle = css.raw({ fontSize: '11px', fontWeight: 'medium', color: 'text.faint' });
  const cellStyle = css.raw({ fontSize: '12px', color: 'text.subtle', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' });
  const stampStyle = css.raw({ fontFamily: 'mono', fontSize: '11px', letterSpacing: '0', color: 'text.faint', whiteSpace: 'nowrap' });

  const rightHeadClass = css(headStyle, { textAlign: 'right' });
  const headClass = css(headStyle);
  const cellClass = css(cellStyle);
  const titleClass = css(cellStyle, { fontWeight: 'semibold', color: 'text.default' });
  const stampClass = css(stampStyle);
  const refClass = css(stampStyle, { overflow: 'hidden', textOverflow: 'ellipsis' });

  const numberClass = css({
    fontFamily: 'mono',
    fontSize: '11px',
    letterSpacing: '0',
    fontVariantNumeric: 'tabular-nums',
    color: 'text.subtle',
    textAlign: 'right',
  });

  const costClass = css({
    fontSize: '11px',
    fontVariantNumeric: 'tabular-nums',
    color: 'text.subtle',
    textAlign: 'right',
    whiteSpace: 'nowrap',
  });

  type Review = PageData['reviews'][number];

  // usage가 없으면 낼 수치 자체가 없고, usage는 있는데 원가가 없으면 단가표에 없는 모델이 섞인 것이다(부분합은 내지 않는다).
  // complete=false면 아직 접히지 않은 fold가 빠져 있으므로 이 금액은 하한이다 — 옆 칸의 '미확정 포함'과 같은 사유다.
  const costLabel = (review: Review): string => {
    if (!review.usage) return '—';
    if (!review.cost) return '단가 미설정';
    return `${review.usage.complete ? '' : '≥ '}${formatKrw(review.cost.krw)}`;
  };
</script>

<Helmet title="관리자" />

<main class={flex({ direction: 'column', minHeight: '[100dvh]', backgroundColor: 'surface.subtle' })}>
  <header
    class={flex({
      align: 'center',
      gap: '10px',
      flex: 'none',
      height: '48px',
      paddingX: '20px',
      borderBottomWidth: '1px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
    })}
  >
    <a class={css({ flex: 'none', color: 'text.faint', _hover: { color: 'text.default' } })} aria-label="홈으로" href="/">
      <Icon icon={IconChevronLeft} size={16} />
    </a>
    <h1 class={css({ fontSize: '14px', fontWeight: 'semibold' })}>관리자</h1>
    <span class={css({ fontSize: '12px', color: 'text.faint' })}>세션 {data.reviews.length}개</span>
    <div class={css({ marginLeft: 'auto' })}>
      <ThemeToggle />
    </div>
  </header>

  <div class={css({ width: 'full', maxWidth: '[1420px]', marginX: 'auto', paddingX: '20px', paddingY: '24px' })}>
    <section
      class={css({
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '10px',
        backgroundColor: 'surface.default',
        boxShadow: 'card',
        overflowX: 'auto',
      })}
    >
      {#if data.reviews.length === 0}
        <p class={css({ paddingY: '48px', textAlign: 'center', fontSize: '13px', color: 'text.faint' })}>아직 세션이 없어요.</p>
      {:else}
        <div class={css({ minWidth: 'fit' })}>
          <div
            class={css({
              display: 'grid',
              // Panda는 정적 추출기다 — 이 문자열을 변수로 빼면 선언이 만들어지지 않는다.
              gridTemplateColumns: '[minmax(140px, 1fr) minmax(160px, 1.4fr) 108px 68px 124px 124px 80px 80px 88px 88px 88px 82px]',
              alignItems: 'center',
              columnGap: '10px',
              paddingX: '16px',
              paddingY: '9px',
              borderBottomWidth: '1px',
              borderColor: 'border.default',
            })}
          >
            <span class={headClass}>테스터</span>
            <span class={headClass}>제목</span>
            <span class={headClass}>문서 ID</span>
            <span class={headClass}>상태</span>
            <span class={headClass}>시작</span>
            <span class={headClass}>종료</span>
            <span class={rightHeadClass}>입력</span>
            <span class={rightHeadClass}>출력</span>
            <span class={rightHeadClass}>캐시 읽기</span>
            <span class={rightHeadClass}>캐시 쓰기</span>
            <span class={rightHeadClass}>비용</span>
            <span class={rightHeadClass}>합산</span>
          </div>

          {#each data.reviews as review (review.id)}
            <a
              class={css({
                display: 'grid',
                gridTemplateColumns: '[minmax(140px, 1fr) minmax(160px, 1.4fr) 108px 68px 124px 124px 80px 80px 88px 88px 88px 82px]',
                alignItems: 'center',
                columnGap: '10px',
                paddingX: '16px',
                paddingY: '10px',
                borderBottomWidth: '1px',
                borderColor: 'border.subtle',
                transition: '[background-color 0.15s ease]',
                _hover: { backgroundColor: 'surface.subtle' },
                _last: { borderBottomWidth: '0' },
              })}
              href={`/sessions/${review.id}`}
            >
              <span class={cellClass}>{review.testerEmail}</span>
              <span class={titleClass}>{review.title || '제목 없음'}</span>
              <span class={refClass}>{review.refId}</span>
              <span>
                <span class={css(badgeRecipe.raw({ status: review.status }))}>{STATUS_LABELS[review.status]}</span>
              </span>
              <span class={stampClass}>{review.startedAt}</span>
              <span class={stampClass}>{review.finishedAt ?? '—'}</span>
              <span class={numberClass}>{review.usage ? num(review.usage.inputTokens) : '—'}</span>
              <span class={numberClass}>{review.usage ? num(review.usage.outputTokens) : '—'}</span>
              <span class={numberClass}>{review.usage ? num(review.usage.cacheReadTokens) : '—'}</span>
              <span class={numberClass}>{review.usage ? num(review.usage.cacheWriteTokens) : '—'}</span>
              <span class={costClass}>{costLabel(review)}</span>
              <span
                class={css({
                  fontSize: '11px',
                  textAlign: 'right',
                  whiteSpace: 'nowrap',
                  color: review.usage && !review.usage.complete ? 'accent.warning.default' : 'text.faint',
                })}
              >
                {review.usage ? (review.usage.complete ? '확정' : '미확정 포함') : '—'}
              </span>
            </a>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</main>
