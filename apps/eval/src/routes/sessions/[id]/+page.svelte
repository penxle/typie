<script lang="ts">
  // cspell:ignore WHATWG

  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Helmet, Icon, Modal } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { tick } from 'svelte';
  import IconChevronLeft from '~icons/lucide/chevron-left';
  import { enhance } from '$app/forms';
  import { invalidateAll } from '$app/navigation';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { applyDelta, sealTurn, startTurn } from '$lib/feedback/delta.ts';
  import { applyEvent, initialLive } from '$lib/feedback/live.ts';
  import { TERMINAL_EVENTS } from '$lib/feedback/stages.ts';
  import { AGENTS } from '$lib/feedback/tiers.ts';
  import ConclusionDrawer from './ConclusionDrawer.svelte';
  import ConclusionPanel from './ConclusionPanel.svelte';
  import ManuscriptView from './ManuscriptView.svelte';
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

  // 전 섹션이 비면 패널·드로어 자체를 두지 않는다 — 빈 편지를 세우는 것보다 없는 편이 낫다.
  const conclusionEmpty = $derived(
    conclusion === null ||
      (conclusion.strengths.length === 0 &&
        conclusion.clearances.length === 0 &&
        conclusion.patterns.length === 0 &&
        conclusion.priorities.length === 0 &&
        (conclusion.understanding ?? '').trim().length === 0),
  );

  // 첫 페인트가 곧 현재 상태다 — 로드가 걷어 온 이벤트 스냅샷으로 시드하고, SSE는 그 커서부터 잇는다.
  // 빈 시드로 시작해 재생을 화면에서 재연하면 새로고침마다 과거 기록이 빨리감기로 보인다.
  // 초기값 캡처는 의도다: 시드는 마운트 1회의 몫이고, 이후는 SSE(실행 중)·아래 효과(종결)가 최신을 소유한다.
  // svelte-ignore state_referenced_locally
  let live = $state(initialLive(data.review.events ?? []));
  // 흐르는 턴의 조각은 리듀서 밖에 둔다 — 로그가 아니라 휘발 프레임이라 재생도 커서도 없고, 턴이 확정되면 사라진다.
  let turnLive = $state<TurnLive | null>(null);
  let cancelForm = $state<HTMLFormElement>();
  let canceling = $state(false);

  let drawerOpen = $state(false);
  let activeId = $state<string | null>(null);
  let modelConfigOpen = $state(false);

  // 회차 첫 방문만 드로어가 열린 채 진입한다(오너 결정) — 정독은 첫 만남의 의식이고, 이후 방문은 상주 패널이
  // 잇는다. 3컬럼이 먼저 그려진 뒤 400ms 뒤에 열어 어디서 열렸는지가 보인다(모션 명세). 기억은 열어 본
  // 사실이 아니라 자동 확장을 소모했다는 표식이라 열자마자 적는다.
  $effect(() => {
    if (conclusionEmpty || data.review.status !== 'completed') return;
    const key = `conclusion-read:${data.session.id}:${data.review.round}`;
    let seen = true;
    try {
      seen = localStorage.getItem(key) !== null;
      localStorage.setItem(key, '1');
    } catch {
      // 접근이 차단된 환경(테마 토글과 같은 처지)은 자동 확장 없이 지나간다.
    }
    if (seen) return;
    const timer = setTimeout(() => (drawerOpen = true), 400);
    return () => clearTimeout(timer);
  });

  // 활성 전환은 반대편만 스크롤한다 — 원고와 카드가 같은 스크롤 통에 있어 둘 다 옮기면 서로를 밀어낸다.
  // 강점(strength.N)은 카드가 없다 — 반대편은 원고의 초록 하이라이트이고, 원고에서 출발한 활성은
  // 패널이 제 몸을 스크롤한다(ConclusionPanel의 칩 연동).
  const activate = async (id: string | null, from: 'manuscript' | 'thread' | 'jump') => {
    activeId = id;
    if (id === null) return;
    await tick();
    // 스레드 id에는 점이 들어간다 — id 선택자는 이스케이프가 필요하므로 전부 속성 선택자로 집는다.
    const target = id.startsWith('strength.')
      ? from === 'manuscript'
        ? null
        : document.querySelector(`[data-thread-range~="${id}"]`)
      : from === 'thread'
        ? document.querySelector(`[data-thread-range~="${id}"]`)
        : document.querySelector(`[data-thread-card="${id}"]`);
    target?.scrollIntoView({ behavior: 'smooth', block: from === 'jump' ? 'center' : 'nearest' });
  };

  const jump = async (id: string) => {
    drawerOpen = false;
    await activate(id, 'jump');
  };

  const closeDrawer = () => {
    drawerOpen = false;
    // 닫힘 직후 포커스는 패널의 확대 버튼으로 복귀한다(모션 명세) — 좁은 폭에서는 스트립이 그 자리다.
    for (const el of document.querySelectorAll<HTMLElement>('[data-drawer-return]')) {
      if (el.offsetParent !== null) {
        el.focus();
        break;
      }
    }
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
    let lastSeenAt = Date.now();

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

    // 이벤트는 프레임 단위로 묶어 한 번에 접는다(fold) — 리듀서는 싸지만 렌더(6개 스테이지 groupFeed 재계산·
    // 스크롤 연출)는 비싸서, 건당 한 번씩 그리면 시드 없는 재생·긴 재접속에서 로그 길이만큼 화면 재연이 늘어진다.
    // 배치 안에서도 적용 순서는 도착 순서 그대로다.
    let batch: { name: string; event: MessageEvent }[] = [];
    let flushScheduled = false;

    const flushBatch = () => {
      flushScheduled = false;
      const events = batch;
      batch = [];
      if (stopped || events.length === 0) return;
      // 판정은 sticky 상태가 아니라 전이다 — 사영이 실패해 running으로 재로드되면 재접속 재생의 첫 프레임이
      // 다시 terminal을 보게 되고, 상태로 판정하면 invalidateAll이 무유계로 재발화한다. 재발화가 없어도
      // 다음 자연 로드가 사영을 재시도한다.
      const wasTerminal = live.terminal;
      let next = live;
      for (const { name, event } of events) {
        const wasCursor = next.cursor;
        next = applyEvent(next, { id: eventId(event.lastEventId), event: name, data: event.data });
        // 조각은 확정된 턴을 넘어 살아남지 않는다 — 확정 텍스트 위에 옛 조각이 겹쳐 보이면 그것이 곧 거짓말이다.
        if (name === 'turn.completed' || TERMINAL_EVENTS.has(name)) turnLive = sealTurn(turnLive, payloadOf(event.data));
        // 턴의 시작도 조각의 유통기한이다. 재생분은 제외한다 — 재생은 커서 이전 id로 도착하므로, 판별은
        // 리듀서와 같은 잣대인 커서 전진으로 한다.
        if (name === 'turn.started' && next.cursor > wasCursor) turnLive = startTurn(turnLive, payloadOf(event.data));
      }
      live = next;
      if (wasTerminal || !live.terminal) return;
      stopped = true;
      clearTimeout(timer);
      source?.close();
      void invalidateAll(); // 이 재로드가 지연 사영을 트리거해 완료 화면으로 전환된다
    };

    const handle = (event: MessageEvent, name: string) => {
      lastSeenAt = Date.now();
      batch.push({ name, event });
      if (flushScheduled) return;
      flushScheduled = true;
      requestAnimationFrame(flushBatch);
    };

    const connect = () => {
      if (stopped) return;
      // 시드된 커서부터 잇는다 — 자동 재접속은 Last-Event-ID 헤더가 우선하므로(relay.resolveCursor) 무해하다.
      const opened = new EventSource(`${url}?lastEventId=${live.cursor}`);
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
        lastSeenAt = Date.now();
        turnLive = applyDelta(turnLive, parseFrame((event as MessageEvent).data));
      });

      // 하트비트(15초 주기, prism sse.ts)는 생존 신호다 — 워치독의 기준 시각만 갱신한다.
      opened.addEventListener('heartbeat', () => {
        lastSeenAt = Date.now();
      });
    };

    // 절반 열림(half-open) 워치독 — 연결만 살고 데이터가 끊긴 스트림은 EventSource가 스스로 알아채지 못해
    // 화면이 문장 중간에서 조용히 얼어붙는다(오류 이벤트 없음). 하트비트가 두 번 넘게 유실되면 끊긴 것으로
    // 보고 닫은 뒤 즉시 다시 연다 — 재생이 놓친 구간을 이어붙인다.
    const STALL_MS = 40_000;
    const watchdog = setInterval(() => {
      if (stopped || Date.now() - lastSeenAt < STALL_MS) return;
      lastSeenAt = Date.now();
      source?.close();
      connect();
    }, 10_000);

    connect();
    return () => {
      stopped = true;
      clearTimeout(timer);
      clearInterval(watchdog);
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
      {#if data.isAdmin}
        <Button onclick={() => (modelConfigOpen = true)} size="sm" type="button" variant="secondary">모델 구성</Button>
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
    <div class={flex({ flexGrow: '1', minHeight: '0' })}>
      {#if conclusion && !conclusionEmpty}
        <ConclusionPanel
          {activeId}
          {conclusion}
          content={data.version.content}
          onActivate={(id) => activate(id, 'jump')}
          onExpand={() => (drawerOpen = true)}
          reaction={data.reaction}
          threads={data.threads}
        />
      {/if}

      <div class={css({ flexGrow: '1', minWidth: '0', overflowY: 'auto' })} data-completed-scroll>
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
            onActivate={(id) => activate(id, 'manuscript')}
            strengths={conclusion?.strengths ?? []}
            threads={data.threads}
            {title}
          />

          <ThreadColumn
            {activeId}
            comments={data.comments}
            content={data.version.content}
            onActivate={(threadId) => activate(threadId, 'thread')}
            patterns={conclusion?.patterns ?? []}
            priorities={conclusion?.priorities ?? []}
            threads={data.threads}
          />
        </div>
      </div>
    </div>

    {#if conclusion && !conclusionEmpty}
      <ConclusionDrawer
        {activeId}
        {conclusion}
        content={data.version.content}
        finishedAt={data.review.finishedAt}
        onActivate={jump}
        onClose={closeDrawer}
        open={drawerOpen}
        reaction={data.reaction}
        round={data.review.round}
        threads={data.threads}
        {title}
      />
    {/if}
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

{#if data.isAdmin}
  <Modal style={css.raw({ padding: '20px', width: '420px' })} bind:open={modelConfigOpen}>
    <h2 class={css({ fontSize: '14px', fontWeight: 'semibold', marginBottom: '10px' })}>이 리뷰의 모델 구성</h2>
    {#if data.modelConfig}
      <div class={css({ display: 'flex', flexDirection: 'column', gap: '6px' })}>
        {#each AGENTS as agent (agent)}
          {@const entry = data.modelConfig[agent]}
          <div class={flex({ align: 'center', gap: '8px', fontSize: '12px' })}>
            <span class={css({ width: '80px', fontFamily: 'mono', color: 'text.subtle' })}>{agent}</span>
            <span class={css({ fontFamily: 'mono', fontWeight: entry.overridden ? 'semibold' : 'normal' })}>
              {entry.model} · {entry.effort}
            </span>
            {#if entry.overridden}
              <span class={css({ fontSize: '11px', color: 'text.brand' })}>변경됨</span>
            {/if}
          </div>
        {/each}
      </div>
      <p class={css({ marginTop: '10px', fontSize: '11px', color: 'text.faint' })}>기본값 표시는 리뷰 시작 시점 기준이에요.</p>
    {:else}
      <p class={css({ fontSize: '12px', color: 'text.faint' })}>구성 기록이 없어요 — 이 기능이 생기기 전에 시작된 리뷰예요.</p>
    {/if}
  </Modal>
{/if}
