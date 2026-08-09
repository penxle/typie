<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import IconCircleCheck from '~icons/lucide/circle-check';
  import { minutesBetween, questionPausedMs } from '$lib/feedback/live.ts';
  import { stagesFor } from '$lib/feedback/stages.ts';
  import StageTimeline from './StageTimeline.svelte';
  import type { TurnLive } from '$lib/feedback/delta.ts';
  import type { AskAnswer, LiveState } from '$lib/feedback/live.ts';
  import type { TierName } from '$lib/feedback/tiers.ts';

  type Props = {
    live: LiveState;
    status: 'running' | 'failed' | 'canceled';
    startedAt: number;
    tier: TierName;
    error: string | null;
    turnLive: TurnLive | null;
    askAnswers: Record<string, AskAnswer[]> | null;
  };
  const { live, status, startedAt, tier, error, turnLive, askAnswers }: Props = $props();

  const ORDINALS = ['첫', '두', '세', '네', '다섯', '여섯'];
  const COUNTS = ['한', '두', '세', '네', '다섯', '여섯'];

  const stages = $derived(stagesFor(tier));

  let now = $state(Date.now());

  $effect(() => {
    if (status !== 'running') return;
    const timer = setInterval(() => (now = Date.now()), 60_000);
    return () => clearInterval(timer);
  });

  const position = $derived(
    Math.max(
      0,
      stages.findIndex((stage) => stage.key === live.currentStage),
    ),
  );
  // 질문 대기는 진행이 아니다 — 파킹 구간을 시점 이동으로 빼면 대기 중에는 카운터가 멈춘 것처럼 선다
  const minutes = $derived(minutesBetween(startedAt + questionPausedMs(live.questions, now), now));

  // 진행 바 채움 — 완료 스테이지 1칸, 진행 중 스테이지는 반 칸으로 셈한다.
  const fill = $derived.by(() => {
    const done = stages.filter((stage) => live.stages[stage.key] === 'done').length;
    const active = stages.some((stage) => live.stages[stage.key] === 'running') ? 0.5 : 0;
    return Math.min(100, Math.round(((done + active) / stages.length) * 100));
  });

  // 답을 기다리는 동안은 리뷰가 도는 시간이 아니라 사람의 차례다 — 흐르는 연출(경과 시간·shimmer)을 멈추고
  // 기다림의 말로 갈아탄다. 종결한 리뷰에는 답할 수 있는 질문이 없으므로 진행 중에만 선다.
  const waiting = $derived(status === 'running' && live.questions.some((question) => question.status === 'pending'));

  // 브라우저 알림 권한 — 요청은 이 버튼의 클릭 제스처 안에서만 한다(로드 시 자동 요청은 Firefox·Safari가
  // 무시하고, 네이티브 프롬프트의 거부는 영구라 진입점 없이 태우면 안 된다). default일 때만 행이 선다:
  // granted는 이미 켜졌고, denied는 브라우저 설정 밖에서 복구할 수 없어 조용히 접는 것이 관례다.
  let notificationPermission = $state<NotificationPermission | null>(null);
  $effect(() => {
    notificationPermission = typeof Notification === 'undefined' ? null : Notification.permission;
  });

  const enableNotifications = async () => {
    try {
      notificationPermission = await Notification.requestPermission();
    } catch {
      notificationPermission = Notification.permission;
    }
  };

  const metaRecipe = cva({
    base: { marginLeft: 'auto', flex: 'none', fontSize: '11px' },
    variants: {
      waiting: {
        true: { fontWeight: 'semibold', color: 'text.brand' },
        false: { color: 'text.disabled' },
      },
    },
  });

  // 대기 중 채움은 정지한 틴트다 — 같은 자리에 같은 너비로 남되, 움직이는 것이 없어 화면이 사람을 기다린다.
  const barRecipe = cva({
    base: { height: 'full', transition: '[width 0.6s ease]' },
    variants: {
      waiting: {
        true: { backgroundColor: 'brand.300', _dark: { backgroundColor: 'dark.brand.700' } },
        false: {
          background:
            '[linear-gradient(90deg, token(colors.accent.brand.default), token(colors.brand.300), token(colors.accent.brand.default))]',
          backgroundSize: '[200% 100%]',
          animation: 'shimmer 2.4s linear infinite',
          _dark: {
            background:
              '[linear-gradient(90deg, token(colors.dark.brand.300), token(colors.dark.brand.100), token(colors.dark.brand.300))]',
          },
        },
      },
    },
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
          {COUNTS[stages.length - 1]} 단계 중 {ORDINALS[position]} 번째 · {stages[position].label}
        </span>
        <span class={css(metaRecipe.raw({ waiting }))}>{waiting ? '답변 대기 중' : `${minutes}분 · 보통 60~90분`}</span>
      </div>

      <div class={css({ height: '3px', marginTop: '9px', borderRadius: '2px', backgroundColor: 'surface.muted', overflow: 'hidden' })}>
        <div style:width={`${fill}%`} class={css(barRecipe.raw({ waiting }))}></div>
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
    <StageTimeline {askAnswers} {error} {live} {now} {status} {tier} {turnLive} />
  </div>

  {#if status === 'running'}
    {#if notificationPermission === 'default'}
      <div
        class={flex({
          align: 'center',
          gap: '10px',
          flex: 'none',
          paddingX: '16px',
          paddingY: '10px',
          borderTopWidth: '1px',
          borderColor: 'border.subtle',
        })}
      >
        <span class={css({ flexGrow: '1', minWidth: '0', fontSize: '12px', lineHeight: '[1.6]', color: 'text.faint' })}>
          질문이 생기거나 리뷰가 끝나면 브라우저 알림으로 알려드릴 수 있어요
        </span>
        <Button style={{ flex: 'none' }} onclick={enableNotifications} size="sm" type="button">알림 켜기</Button>
      </div>
    {:else if notificationPermission === 'granted'}
      <!-- 켜짐 상태도 자리에 남는다 — 행이 통째로 사라지면 방금 누른 사람이 성공을 확인할 길이 없다. -->
      <div
        class={flex({
          align: 'center',
          gap: '6px',
          flex: 'none',
          paddingX: '16px',
          paddingTop: '11px',
          paddingBottom: '13px',
          borderTopWidth: '1px',
          borderColor: 'border.subtle',
        })}
      >
        <span class={css({ display: 'inline-flex', flex: 'none', color: 'text.success' })}>
          <Icon icon={IconCircleCheck} size={12} />
        </span>
        <span class={css({ minWidth: '0', fontSize: '12px', lineHeight: '[1.6]', color: 'text.faint' })}>
          질문이 생기거나 리뷰가 끝나면 브라우저 알림으로 알려드려요
        </span>
      </div>
    {/if}
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
      {waiting
        ? '답하시기 전까지 리뷰가 기다려요 — 창을 닫아도 질문은 남아 있어요.'
        : '이 창을 닫아도 리뷰는 계속되고, 끝난 뒤 들어오시면 결과가 정리돼 있어요.'}
    </p>
  {/if}
</aside>
