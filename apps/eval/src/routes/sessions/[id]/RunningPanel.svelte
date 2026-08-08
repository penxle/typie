<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { minutesBetween } from '$lib/feedback/live.ts';
  import { STAGES } from '$lib/feedback/stages.ts';
  import StageTimeline from './StageTimeline.svelte';
  import type { TurnLive } from '$lib/feedback/delta.ts';
  import type { LiveState } from '$lib/feedback/live.ts';

  type Props = {
    live: LiveState;
    status: 'running' | 'failed' | 'canceled';
    startedAt: number;
    error: string | null;
    turnLive: TurnLive | null;
  };
  const { live, status, startedAt, error, turnLive }: Props = $props();

  const ORDINALS = ['첫', '두', '세', '네', '다섯', '여섯'];

  let now = $state(Date.now());

  $effect(() => {
    if (status !== 'running') return;
    const timer = setInterval(() => (now = Date.now()), 60_000);
    return () => clearInterval(timer);
  });

  const position = $derived(
    Math.max(
      0,
      STAGES.findIndex((stage) => stage.key === live.currentStage),
    ),
  );
  const minutes = $derived(minutesBetween(startedAt, now));

  // 진행 바 채움 — 완료 스테이지 1칸, 진행 중 스테이지는 반 칸으로 셈한다.
  const fill = $derived.by(() => {
    const done = STAGES.filter((stage) => live.stages[stage.key] === 'done').length;
    const active = STAGES.some((stage) => live.stages[stage.key] === 'running') ? 0.5 : 0;
    return Math.min(100, Math.round(((done + active) / STAGES.length) * 100));
  });
</script>

<aside
  class={flex({
    direction: 'column',
    flex: 'none',
    width: '380px',
    borderLeftWidth: '1px',
    borderColor: 'border.subtle',
    backgroundColor: 'surface.default',
  })}
>
  {#if status === 'running'}
    <div class={css({ flex: 'none', paddingX: '16px', paddingTop: '14px' })}>
      <div class={flex({ align: 'baseline', gap: '6px' })}>
        <span class={css({ fontSize: '13px', fontWeight: 'bold' })}>
          여섯 단계 중 {ORDINALS[position]} 번째 · {STAGES[position].label}
        </span>
        <span class={css({ marginLeft: 'auto', flex: 'none', fontSize: '11px', color: 'text.disabled' })}>{minutes}분 · 보통 40~60분</span>
      </div>

      <div class={css({ height: '3px', marginTop: '9px', borderRadius: '2px', backgroundColor: 'surface.muted', overflow: 'hidden' })}>
        <div
          style:width={`${fill}%`}
          class={css({
            height: 'full',
            background:
              '[linear-gradient(90deg, token(colors.accent.brand.default), token(colors.brand.300), token(colors.accent.brand.default))]',
            backgroundSize: '[200% 100%]',
            animation: 'shimmer 2.4s linear infinite',
            transition: '[width 0.6s ease]',
            _dark: {
              background:
                '[linear-gradient(90deg, token(colors.dark.brand.300), token(colors.dark.brand.100), token(colors.dark.brand.300))]',
            },
          })}
        ></div>
      </div>
    </div>
  {:else}
    <div class={css({ flex: 'none', paddingX: '16px', paddingTop: '14px' })}>
      <p class={css({ fontSize: '13px', fontWeight: 'bold', color: status === 'failed' ? 'text.danger' : 'text.default' })}>
        {status === 'failed' ? '리뷰가 실패했어요' : '리뷰를 중단했어요'}
      </p>
      {#if status === 'failed' && error && live.currentStage === null}
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
          {error}
        </p>
      {/if}
      <p class={css({ marginTop: '8px', fontSize: '12px', lineHeight: '[1.6]', color: 'text.faint' })}>
        홈에서 새 세션으로 다시 시도할 수 있어요
        <br />
        다시 시도하면 새 리뷰가 처음부터 실행돼요
      </p>
    </div>
  {/if}

  <div
    class={flex({
      direction: 'column',
      gap: '6px',
      flexGrow: '1',
      minHeight: '0',
      overflowY: 'auto',
      paddingX: '16px',
      paddingTop: '14px',
      paddingBottom: '14px',
    })}
  >
    <StageTimeline {error} {live} {now} {status} {turnLive} />
  </div>

  {#if status === 'running'}
    <p
      class={css({
        flex: 'none',
        paddingX: '16px',
        paddingTop: '11px',
        paddingBottom: '13px',
        borderTopWidth: '1px',
        borderColor: 'border.subtle',
        fontSize: '12px',
        lineHeight: '[1.6]',
        color: 'text.faint',
      })}
    >
      이 창을 닫아도 리뷰는 계속되고, 끝난 뒤 들어오시면 결과가 정리돼 있어요.
    </p>
  {/if}
</aside>
