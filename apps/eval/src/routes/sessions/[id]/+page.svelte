<script lang="ts">
  // cspell:ignore WHATWG

  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Helmet, Icon } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { tick } from 'svelte';
  import IconChevronLeft from '~icons/lucide/chevron-left';
  import { enhance } from '$app/forms';
  import { invalidateAll } from '$app/navigation';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { applyDelta, sealTurn, startTurn } from '$lib/feedback/delta.ts';
  import { applyEvent, initialLive } from '$lib/feedback/live.ts';
  import { TERMINAL_EVENTS } from '$lib/feedback/stages.ts';
  import ManuscriptView from './ManuscriptView.svelte';
  import OverviewBand from './OverviewBand.svelte';
  import RunningPanel from './RunningPanel.svelte';
  import ThreadColumn from './ThreadColumn.svelte';
  import type { SubmitFunction } from '@sveltejs/kit';
  import type { TurnLive } from '$lib/feedback/delta.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  // EventSource는 named event만 리스너에 흘린다 — 이 9종이 로그 이벤트 어휘 전부다. 휘발 프레임 turn.delta는
  // 로그에 남지 않아(id 없음) 커서·리듀서 밖 경로로 따로 받는다.
  const EVENT_NAMES = [
    'run.started',
    'step.started',
    'step.completed',
    'turn.started',
    'turn.completed',
    'tool.called',
    'run.completed',
    'run.failed',
    'run.canceled',
  ];
  const RETRY_MS = 10_000;

  const STATUS_LABELS = { running: '진행 중', completed: '리뷰 완료', failed: '실패', canceled: '중단됨' };

  // 커서는 릴레이 resolveCursor와 같은 잣대로 본다 — 음수·비정수·과대값은 커서로 쓰지 않고 없는 것으로 다룬다.
  const eventId = (raw: string): number | null => {
    if (!raw) return null;
    const id = Number(raw);
    return Number.isSafeInteger(id) && id >= 0 ? id : null;
  };

  const title = $derived(data.session.title || '제목 없음');
  const paragraphs = $derived(
    data.version.content
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line.length > 0),
  );

  const conclusion = $derived(data.review.result?.conclusion ?? null);

  let live = $state(initialLive([]));
  // 흐르는 턴의 조각은 리듀서 밖에 둔다 — 로그가 아니라 휘발 프레임이라 재생도 커서도 없고, 턴이 확정되면 사라진다.
  let turnLive = $state<TurnLive | null>(null);
  let cancelForm = $state<HTMLFormElement>();
  let canceling = $state(false);

  let bandOpen = $state(false);
  let activeId = $state<string | null>(null);

  // 활성 전환은 반대편만 스크롤한다 — 원고와 카드가 같은 스크롤 통에 있어 둘 다 옮기면 서로를 밀어낸다.
  const activate = async (threadId: string | null, from: 'manuscript' | 'thread' | 'jump') => {
    activeId = threadId;
    if (threadId === null) return;
    await tick();
    // 스레드 id에는 점이 들어간다 — id 선택자는 이스케이프가 필요하므로 양쪽 다 속성 선택자로 집는다.
    const target =
      from === 'thread'
        ? document.querySelector(`[data-thread-range~="${threadId}"]`)
        : document.querySelector(`[data-thread-card="${threadId}"]`);
    target?.scrollIntoView({ behavior: 'smooth', block: from === 'jump' ? 'center' : 'nearest' });
  };

  const jump = async (threadId: string) => {
    bandOpen = false;
    await activate(threadId, 'jump');
  };

  // 경과·소요의 기준 시각은 run.started의 봉투 시각이다. 스냅샷도 라이브도 같은 축을 쓰고, 없으면 DB 시작 시각으로 떨어진다.
  const originAt = $derived(live.startedAt ?? data.review.startedAt);

  // 종결 리뷰의 타임라인 원천은 사영된 이벤트 스냅샷이다. 실행 중이면 아래 SSE 재생이 같은 상태를 채운다.
  $effect(() => {
    if (data.review.status === 'running') return;
    live = initialLive(data.review.events ?? []);
  });

  $effect(() => {
    if (data.review.status !== 'running') return;

    const url = `/sessions/${data.session.id}/events`;
    let source: EventSource | null = null;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let stopped = false;

    // data 라인은 항상 한 줄 JSON이다 — 델타는 그 자체가 프레임이고, 로그 이벤트는 {seq,kind,data,createdAt} 봉투다.
    const parseFrame = (raw: string): unknown => {
      try {
        return JSON.parse(raw);
      } catch {
        return null;
      }
    };

    // 봉인 판정은 리듀서 밖 경로라 봉투를 여기서 한 겹 벗긴다. 못 벗기면 빈 본문으로 본다 — 어느 턴의
    // 확정인지 모르는 채 조각을 남겨 두는 것보다 지우는 편이 안전하다(확정 텍스트가 곧 뒤따른다).
    const payloadOf = (raw: string): { turn?: unknown; agent?: unknown } => {
      const envelope = parseFrame(raw);
      const data = envelope && typeof envelope === 'object' ? (envelope as Record<string, unknown>).data : null;
      return data && typeof data === 'object' ? (data as Record<string, unknown>) : {};
    };

    const handle = (event: MessageEvent, name: string) => {
      // 판정은 sticky 상태가 아니라 전이다 — 사영이 실패해 running으로 재로드되면 재접속 재생의 첫 프레임이
      // 다시 terminal을 보게 되고, 상태로 판정하면 invalidateAll이 무유계로 재발화한다. 재발화가 없어도
      // 다음 자연 로드가 사영을 재시도한다.
      const wasTerminal = live.terminal;
      const wasCursor = live.cursor;
      live = applyEvent(live, { id: eventId(event.lastEventId), event: name, data: event.data });
      // 조각은 확정된 턴을 넘어 살아남지 않는다 — 확정 텍스트 위에 옛 조각이 겹쳐 보이면 그것이 곧 거짓말이다.
      if (name === 'turn.completed' || TERMINAL_EVENTS.has(name)) turnLive = sealTurn(turnLive, payloadOf(event.data));
      // 턴의 시작도 조각의 유통기한이다. 재생분은 제외한다 — 직접 다시 여는 재접속은 Last-Event-ID 없이 붙어
      // 로그를 처음부터 다시 받으므로(events/+server.ts:24), 그 옛 turn.started가 지금 흐르는 턴을 지우면 안
      // 된다. 판별은 리듀서와 같은 잣대인 커서 전진으로 한다.
      if (name === 'turn.started' && live.cursor > wasCursor) turnLive = startTurn(turnLive, payloadOf(event.data));
      if (wasTerminal || !live.terminal) return;
      stopped = true;
      clearTimeout(timer);
      source?.close();
      void invalidateAll(); // 이 재로드가 지연 사영을 트리거해 완료 화면으로 전환된다
    };

    const connect = () => {
      if (stopped) return;
      const opened = new EventSource(url);
      source = opened;

      // 확립된 스트림의 종료는 EventSource가 Last-Event-ID를 들고 스스로 다시 잇는다. 하지만 연결 시점의
      // 비200 응답은 재접속 없이 영구 CLOSED다(WHATWG HTML §9.2) — 그 경우만 지연 후 직접 다시 연다.
      opened.addEventListener('error', () => {
        if (stopped || opened.readyState !== EventSource.CLOSED) return;
        opened.close();
        timer = setTimeout(connect, RETRY_MS);
      });

      for (const name of EVENT_NAMES) {
        opened.addEventListener(name, (event) => handle(event as MessageEvent, name));
      }

      // 델타는 id가 없어 커서를 건드리지 않는다 — 유실이 계약이라 놓친 조각은 뒤따르는 확정 텍스트가 바로잡는다.
      opened.addEventListener('turn.delta', (event) => {
        turnLive = applyDelta(turnLive, parseFrame((event as MessageEvent).data));
      });
    };

    connect();
    return () => {
      stopped = true;
      clearTimeout(timer);
      source?.close();
    };
  });

  const confirmCancel = () => {
    Dialog.confirm({
      title: '리뷰를 중단할까요?',
      message: '지금까지 읽은 내용은 남지 않고, 다시 받으려면 새 세션으로 처음부터 시작해야 해요.',
      action: 'danger',
      actionLabel: '리뷰 중단',
      actionHandler: () => cancelForm?.requestSubmit(),
    });
  };

  const submitCancel: SubmitFunction = () => {
    canceling = true;
    return async ({ result, update }) => {
      canceling = false;
      if (result.type === 'success') Toast.success('중단을 요청했어요');
      else if (result.type === 'failure') Toast.error(String(result.data?.error ?? '중단 요청을 보내지 못했어요'));
      // 스트림을 끊지 않는다 — 취소의 확정은 터미널 이벤트와 사영이 한다.
      await update({ invalidateAll: false, reset: false });
    };
  };

  const badgeRecipe = cva({
    base: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '5px',
      flexShrink: '0',
      paddingX: '9px',
      paddingY: '3px',
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
</script>

<Helmet {title} />

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
    <a class={css({ flex: 'none', color: 'text.faint', _hover: { color: 'text.default' } })} aria-label="목록으로" href="/">
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
      AI 리뷰 · {data.review.round}회차
    </span>

    <span class={css(badgeRecipe.raw({ status: data.review.status }))}>
      {#if data.review.status === 'running'}
        <span
          class={css({
            size: '5px',
            borderRadius: 'full',
            backgroundColor: 'accent.brand.default',
            animation: 'pulse 1.6s ease-in-out infinite',
          })}
        ></span>
      {/if}
      {STATUS_LABELS[data.review.status]}
    </span>

    <div class={flex({ align: 'center', gap: '8px', marginLeft: 'auto' })}>
      {#if data.review.status === 'completed'}
        <a
          class={css({
            paddingX: '14px',
            paddingY: '7px',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '6px',
            backgroundColor: 'surface.default',
            fontSize: '12px',
            fontWeight: 'medium',
            color: 'text.subtle',
            boxShadow: 'small',
            _hover: { borderColor: 'border.strong' },
          })}
          href={`/sessions/${data.session.id}/process`}
        >
          과정 보기
        </a>
      {/if}
      {#if data.review.status === 'running'}
        <form bind:this={cancelForm} action="?/cancel" method="post" use:enhance={submitCancel}>
          <Button disabled={canceling} loading={canceling} onclick={confirmCancel} size="sm" type="button" variant="secondary">
            리뷰 중단
          </Button>
        </form>
      {/if}
      <ThemeToggle />
    </div>
  </header>

  {#if data.review.status === 'completed'}
    <div class={css({ flexGrow: '1', minHeight: '0', overflowY: 'auto' })}>
      {#if conclusion}
        <OverviewBand
          {conclusion}
          onJump={jump}
          onToggle={() => (bandOpen = !bandOpen)}
          open={bandOpen}
          reaction={data.reaction}
          threads={data.threads}
        />
      {/if}

      <div
        class={flex({
          gap: '28px',
          width: 'full',
          maxWidth: '1184px',
          marginX: 'auto',
          paddingX: '24px',
          paddingTop: '44px',
          paddingBottom: '48px',
        })}
      >
        <ManuscriptView
          {activeId}
          content={data.version.content}
          onActivate={(threadId) => activate(threadId, 'manuscript')}
          threads={data.threads}
          {title}
        />

        <ThreadColumn
          {activeId}
          comments={data.comments}
          content={data.version.content}
          onActivate={(threadId) => activate(threadId, 'thread')}
          patterns={conclusion?.patterns ?? []}
          threads={data.threads}
        />
      </div>
    </div>
  {:else}
    <div class={flex({ flexGrow: '1', minHeight: '0' })}>
      <div class={css({ flexGrow: '1', minWidth: '0', overflowY: 'auto' })}>
        <div class={css({ width: 'full', maxWidth: '620px', marginX: 'auto', paddingX: '32px', paddingY: '52px' })}>
          <h1 class={css({ marginBottom: '40px', fontFamily: 'RIDIBatang', fontSize: '24px' })}>{title}</h1>
          <div class={flex({ direction: 'column', gap: '16px' })}>
            {#each paragraphs as paragraph, index (index)}
              <p
                class={css({
                  fontFamily: 'RIDIBatang',
                  fontSize: '16px',
                  lineHeight: '[1.95]',
                  color: 'text.subtle',
                  textIndent: '[1em]',
                })}
              >
                {paragraph}
              </p>
            {/each}
          </div>
        </div>
      </div>

      <RunningPanel error={data.review.error} {live} startedAt={originAt} status={data.review.status} {turnLive} />
    </div>
  {/if}
</div>
