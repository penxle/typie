<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import IconChevronLeft from '~icons/lucide/chevron-left';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { initialLive, minutesBetween } from '$lib/feedback/live.ts';
  import { askAnswerIndex } from '$lib/feedback/questions.ts';
  import StageTimeline from '../StageTimeline.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const title = $derived(data.session.title || '제목 없음');

  // 스냅샷은 이미 종결된 리뷰의 전량 재생분이다 — 한 번 접으면 더 들어올 이벤트가 없다.
  const live = $derived(initialLive(data.review.events));

  // 답변 문면은 재생에 없다 — 사영이 원장에서 걷어 굳힌 기록에서 되짚어 카드에 싣는다.
  const askAnswers = $derived(askAnswerIndex(data.review.questions));

  const endedAt = $derived(data.review.finishedAt ?? data.review.startedAt);

  const duration = $derived(
    data.review.finishedAt === null
      ? null
      : data.review.finishedAt - data.review.startedAt < 60_000
        ? '1분 미만'
        : `${minutesBetween(data.review.startedAt, data.review.finishedAt)}분`,
  );

  const failed = $derived(data.review.status === 'failed');
</script>

<Helmet title={`${title} 리뷰 과정`} />

<div class={flex({ direction: 'column', height: '[100dvh]', backgroundColor: 'surface.default' })}>
  <header
    class={flex({
      align: 'center',
      gap: '10px',
      flex: 'none',
      height: '48px',
      paddingX: '14px',
      borderBottomWidth: '1px',
      borderColor: 'border.default',
    })}
  >
    <a
      class={css({ flex: 'none', color: 'text.faint', _hover: { color: 'text.default' } })}
      aria-label="세션으로"
      href={`/sessions/${data.session.id}`}
    >
      <Icon icon={IconChevronLeft} size={16} />
    </a>

    <span
      class={css({
        minWidth: '0',
        fontSize: '14px',
        fontWeight: 'semibold',
        whiteSpace: 'nowrap',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
      })}
    >
      {title}
    </span>
    <span class={css({ flex: 'none', fontSize: '12px', color: 'text.faint' })}>/</span>
    <span class={css({ flex: 'none', fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>
      리뷰 {data.review.roundNumber}회차 과정
    </span>

    <div class={flex({ align: 'center', gap: '8px', marginLeft: 'auto' })}>
      {#if duration}
        <span class={css({ flex: 'none', fontSize: '12px', color: 'text.faint' })}>소요 {duration}</span>
      {/if}
      <ThemeToggle />
    </div>
  </header>

  <div class={css({ flexGrow: '1', minHeight: '0', overflowY: 'auto' })}>
    <div class={css({ width: 'full', maxWidth: '720px', marginX: 'auto', paddingX: '24px', paddingY: '32px' })}>
      {#if data.review.status !== 'completed'}
        <div
          class={css({
            marginBottom: '18px',
            paddingX: '14px',
            paddingY: '12px',
            borderWidth: '1px',
            borderColor: failed ? 'border.danger' : 'border.default',
            borderRadius: '8px',
            backgroundColor: failed ? 'accent.danger.subtle' : 'surface.muted',
          })}
        >
          <p class={css({ fontSize: '13px', fontWeight: 'bold', color: failed ? 'text.danger' : 'text.subtle' })}>
            {failed ? '리뷰가 실패했어요' : '리뷰를 중단했어요'}
          </p>
          {#if failed && data.review.error}
            <p
              class={css({
                marginTop: '8px',
                paddingX: '10px',
                paddingY: '8px',
                borderRadius: '6px',
                backgroundColor: 'accent.danger.subtle',
                fontFamily: 'mono',
                fontSize: '11px',
                letterSpacing: '0',
                lineHeight: '[1.6]',
                color: 'text.danger',
                wordBreak: 'break-all',
              })}
            >
              {data.review.error}
            </p>
          {/if}
        </div>
      {/if}

      <StageTimeline {askAnswers} {live} now={endedAt} status={data.review.status} tier={data.review.tier} />
    </div>
  </div>
</div>
