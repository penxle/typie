<script lang="ts">
  // cspell:ignore WHATWG

  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Helmet, Icon, Tooltip } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { tick, untrack } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import IconChevronLeft from '~icons/lucide/chevron-left';
  import { enhance } from '$app/forms';
  import { invalidateAll } from '$app/navigation';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { applyDelta, sealTurn, startTurn } from '$lib/feedback/delta.ts';
  import { applyEvent, initialLive } from '$lib/feedback/live.ts';
  import { askAnswerIndex } from '$lib/feedback/questions.ts';
  import { EVENT_NAMES } from '$lib/feedback/sse.ts';
  import { synthesizeFolds, usageLowerBound } from '$lib/feedback/stage-cost.ts';
  import { TERMINAL_EVENTS } from '$lib/feedback/stages.ts';
  import ArtifactsModal from './ArtifactsModal.svelte';
  import ConclusionDrawer from './ConclusionDrawer.svelte';
  import ConclusionPanel from './ConclusionPanel.svelte';
  import DiagnosticsModal from './DiagnosticsModal.svelte';
  import InfoModal from './InfoModal.svelte';
  import ManuscriptView from './ManuscriptView.svelte';
  import RunningPanel from './RunningPanel.svelte';
  import ThreadColumn from './ThreadColumn.svelte';
  import type { SubmitFunction } from '@sveltejs/kit';
  import type { TurnLive } from '$lib/feedback/delta.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

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
  // 강점은 의미 진술 티어(high)의 총평에만 선다 — 없는 티어(medium)는 빈 목록으로 접어 절이 서지 않게 한다.
  const strengths = $derived(conclusion?.strengths ?? []);
  // 판정은 결과의 최상위 필드다 — 기준표를 세우는 티어에만 실리므로 없으면 절 자체가 서지 않는다.
  const verdicts = $derived(data.review.result?.verdicts ?? []);
  // 격상도 결과의 최상위 필드다 — 결함이 아니라 다음 단계의 제안이라 지적 스레드를 만들지 않고 총평에만 선다.
  const elevations = $derived(data.review.result?.elevations ?? []);

  // 총평이 참조하는 스레드는 표시 회차가 만든 것뿐이다 — issues 번호가 그 회차 result.issues의 인덱스라,
  // 같은 번호를 가진 승계 스레드에 지난 회차의 칩·콜아웃이 잘못 붙는다.
  const conclusionThreads = $derived(data.threads.filter((thread) => thread.reviewRound === data.review.round));

  // 전 섹션이 비면 패널·드로어 자체를 두지 않는다 — 빈 편지를 세우는 것보다 없는 편이 낫다.
  const conclusionEmpty = $derived(
    conclusion === null ||
      (strengths.length === 0 &&
        verdicts.length === 0 &&
        elevations.length === 0 &&
        conclusion.patterns.length === 0 &&
        conclusion.priorities.length === 0 &&
        (conclusion.understanding ?? '').trim().length === 0 &&
        (conclusion.progress ?? '').trim().length === 0),
  );

  // low 티어는 마무리 글 단계 자체가 없다 — 빈 총평 폴백에 기대지 않고 여기서 명시로 걷는다.
  const conclusionShown = $derived(data.review.tier !== 'low' && !conclusionEmpty);

  // 첫 페인트가 곧 현재 상태다 — 로드가 걷어 온 이벤트 스냅샷으로 시드하고, SSE는 그 커서부터 잇는다.
  // 빈 시드로 시작해 재생을 화면에서 재연하면 새로고침마다 과거 기록이 빨리감기로 보인다.
  // 초기값 캡처는 의도다: 시드는 마운트 1회의 몫이고, 이후는 SSE(실행 중)·아래 효과(종결)가 최신을 소유한다.
  // svelte-ignore state_referenced_locally
  let live = $state(initialLive(data.review.events ?? []));
  // 흐르는 턴의 조각은 리듀서 밖에 둔다 — 로그가 아니라 휘발 프레임이라 재생도 커서도 없고, 턴이 확정되면 사라진다.
  let turnLive = $state<TurnLive | null>(null);
  // 연결 생사와 델타 신선도는 UI의 정직성 재료다 — 끊긴 스트림 위의 "생각하는 중"은 거짓말이라(90분 관측),
  // 두절이면 재연결 문면을, 델타가 마른 지 오래면 지연 문면을 대신 세운다(StageTimeline liveTail).
  let streamStale = $state(false);
  let lastDeltaAt = $state(Date.now());
  let cancelForm = $state<HTMLFormElement>();
  let canceling = $state(false);
  let rereviewForm = $state<HTMLFormElement>();
  let rereviewing = $state(false);
  let resumeForm = $state<HTMLFormElement>();
  let resuming = $state(false);

  // 질문 대기는 탭 제목으로도 알린다 — 다른 탭에 가 있는 작가가 리뷰가 멈춘 것을 알 수 있는 유일한 신호다.
  // 스피너는 백그라운드 탭에서 인터벌이 ~1s로 스로틀되어 느리게 돌지만, 제목이 움직인다는 사실만으로 족하다.
  const TAB_SPINNER = ['⠋', '⠙', '⠸', '⠴', '⠦', '⠇'];
  const hasPendingQuestion = $derived(live.questions.some((question) => question.status === 'pending'));
  let spin = $state(0);
  $effect(() => {
    if (!hasPendingQuestion) return;
    const timer = setInterval(() => (spin = (spin + 1) % TAB_SPINNER.length), 600);
    return () => clearInterval(timer);
  });
  const tabTitle = $derived(hasPendingQuestion ? `${TAB_SPINNER[spin]} 질문이 있어요` : data.session.title || '제목 없음');

  // 브라우저 알림 — 권한이 있고 시선이 떠나 있을 때만 낸다(보고 있는 화면엔 카드·탭 제목이 이미 있다).
  // 권한 요청은 RunningPanel의 [알림 켜기] 제스처 몫이고, 여기는 granted를 읽기만 한다.
  const canNotify = () =>
    typeof Notification !== 'undefined' && Notification.permission === 'granted' && (document.hidden || !document.hasFocus());

  // 알림 생성은 미지원 환경(Android Chrome 등)에서 던진다 — 알림은 부가 신호라 조용히 접는다.
  const showNotification = (text: string, body: string, tag: string): Notification | null => {
    try {
      const notification = new Notification(text, { body, tag });
      notification.addEventListener('click', () => {
        window.focus();
        notification.close();
      });
      return notification;
    } catch {
      return null;
    }
  };

  // 질문당 1회 — 재접속 재생이 같은 pending을 다시 보여도 재발화하지 않는다. 떠 있는 알림은 해소되면 닫는다.
  const askNotified = new SvelteSet<string>();
  const askShown = new SvelteMap<string, Notification>();
  $effect(() => {
    for (const question of live.questions) {
      if (question.status === 'pending' && !askNotified.has(question.toolCallId)) {
        askNotified.add(question.toolCallId);
        if (!canNotify()) continue;
        const first = question.questions[0]?.question ?? '';
        const body = question.questions.length > 1 ? `${first} 외 ${question.questions.length - 1}개` : first;
        const notification = showNotification(`질문이 있어요 — ${title}`, body, `ask:${data.session.id}:${question.toolCallId}`);
        if (notification) askShown.set(question.toolCallId, notification);
      }
      if (question.status !== 'pending') {
        askShown.get(question.toolCallId)?.close();
        askShown.delete(question.toolCallId);
      }
    }
  });

  // 완료 알림은 이 방문에서 실행 중인 리뷰를 본 적이 있을 때만 낸다 — 끝난 리뷰의 재방문은 알리지 않는다.
  // 관찰은 마운트 시점 하나로 굳히지 않는다: 완료 화면에서 재리뷰를 시작한 방문도 그 회차가 도는 것을 보게 되고,
  // 그 완료를 알림으로 받는다. 실패·중단은 알리지 않는다(오너 승인 범위 = 질문·완료).
  // 비반응 plain let은 의도다 — 방문 안의 관찰 기록이지 렌더 입력이 아니다.
  // svelte-ignore state_referenced_locally
  let sawRunning = data.review.status === 'running';
  let doneNotified = false;
  $effect(() => {
    if (data.review.status === 'running') {
      sawRunning = true;
      // 새 회차가 돌기 시작했다는 뜻이다 — 같은 방문에서 재리뷰를 잇달아 돌려도 회차마다 완료를 알린다.
      // 같은 회차가 완료에서 실행 중으로 되돌아오는 일은 없다(종결 행은 running 조건부로만 갱신된다).
      doneNotified = false;
      return;
    }
    if (!sawRunning || doneNotified || data.review.status !== 'completed') return;
    doneNotified = true;
    if (!canNotify()) return;
    showNotification(`리뷰가 끝났어요 — ${title}`, '결과가 정리돼 있어요.', `done:${data.session.id}`);
  });

  // 답변 문면은 해소 이벤트(tool.resolved)가 실어 리듀서 엔트리에 남는다 — 실행 중(SSE)·종결(사영된 events) 모두 같은 원천이다.
  const askAnswers = $derived(askAnswerIndex(live.questions));

  let drawerOpen = $state(false);
  let activeId = $state<string | null>(null);
  let infoOpen = $state(false);
  let diagnosticsOpen = $state(false);
  let artifactsOpen = $state(false);

  // 산출물 모달은 high 파이프라인의 정상 완료 회차에만 선다 — 정리 UI가 그 티어의 계약을 미러하고(다른 티어는 같은
  // 이름의 파일도 형태가 다르다), 거부 종결은 판정 산출물 자체가 없다. 엔드포인트도 같은 조건으로 409를 낸다.
  const artifactsAvailable = $derived(data.review.status === 'completed' && !data.review.rejection && data.review.tier === 'high');

  // 실행 중 비용은 턴 누적 + modelConfig 스냅샷의 합성이다(스펙 §3.1) — admin이 아니면 계산 자체를 하지 않는다.
  const costFolds = $derived(data.isAdmin ? synthesizeFolds(live.usage, data.modelConfig) : []);

  // 정보 모달의 비용 원천은 상태가 가른다 — 실행 중은 위 합성이 유일한 표면이고, 종결은 원장 기록이 정본이다.
  const infoRunning = $derived(data.review.status === 'running');
  const infoFolds = $derived(infoRunning ? costFolds : data.usage === null ? null : (data.usage.folds ?? []));
  const infoLowerBound = $derived(infoRunning || usageLowerBound(data.usage));

  // 회차 첫 방문만 드로어가 열린 채 진입한다(오너 결정) — 정독은 첫 만남의 의식이고, 이후 방문은 상주 패널이
  // 잇는다. 3컬럼이 먼저 그려진 뒤 400ms 뒤에 열어 어디서 열렸는지가 보인다(모션 명세). 기억은 열어 본
  // 사실이 아니라 자동 확장을 소모했다는 표식이라 열자마자 적는다.
  $effect(() => {
    if (!conclusionShown || data.review.status !== 'completed') return;
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

  // 경과·소요의 기준 시각은 workflow.started의 봉투 시각이다. 스냅샷도 라이브도 같은 축을 쓰고, 없으면 DB 시작 시각으로 떨어진다.
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
    let lastProgressAt = Date.now();

    // data 라인은 항상 한 줄 JSON이다 — 델타는 그 자체가 프레임이고, 로그 이벤트는 {seq,kind,occurredAt,context,data} 봉투다.
    const parseFrame = (raw: string): unknown => {
      try {
        return JSON.parse(raw);
      } catch {
        return null;
      }
    };

    // 봉인·시작 판정은 리듀서 밖 경로라 봉투의 좌표(context)를 여기서 꺼낸다. 못 꺼내면 빈 좌표로 본다 — 어느
    // 턴의 확정인지 모르는 채 조각을 남겨 두는 것보다 지우는 편이 안전하다(확정 텍스트가 곧 뒤따른다).
    const contextOf = (raw: string): unknown => {
      const envelope = parseFrame(raw);
      return envelope && typeof envelope === 'object' ? ((envelope as Record<string, unknown>).context ?? {}) : {};
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
        if (name === 'turn.completed' || TERMINAL_EVENTS.has(name)) turnLive = sealTurn(turnLive, contextOf(event.data));
        // 턴의 시작도 조각의 유통기한이다. 재생분은 제외한다 — 재생은 커서 이전 id로 도착하므로, 판별은
        // 리듀서와 같은 잣대인 커서 전진으로 한다.
        if (name === 'turn.started' && next.cursor > wasCursor) turnLive = startTurn(turnLive, contextOf(event.data));
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
      lastProgressAt = Date.now();
      streamStale = false;
      batch.push({ name, event });
      if (flushScheduled) return;
      flushScheduled = true;
      // rAF는 숨은 탭에서 멎는다 — 그대로면 백그라운드에서 질문·종결이 접히지 않아 탭 제목도 알림도 침묵한다.
      // 타이머를 나란히 걸어 어느 쪽이든 먼저 접게 한다(뒤에 온 쪽은 빈 배치를 보고 그냥 돌아간다).
      requestAnimationFrame(flushBatch);
      setTimeout(flushBatch, 300);
    };

    const connect = () => {
      if (stopped) return;
      // 시드된 커서부터 잇는다 — 자동 재접속은 Last-Event-ID 헤더가 우선하므로(relay.resolveCursor) 무해하다.
      // 커서 읽기는 untrack — 첫 connect는 effect 본문의 동기 호출이라, 그대로 읽으면 커서가 effect 의존성에
      // 들어가 배치마다 연결을 끊고 다시 연다.
      const opened = new EventSource(`${url}?lastEventId=${untrack(() => live.cursor)}`);
      source = opened;

      // 확립된 스트림의 종료는 EventSource가 Last-Event-ID를 들고 스스로 다시 잇는다. 하지만 연결 시점의
      // 비200 응답은 재접속 없이 영구 CLOSED다(WHATWG HTML §9.2) — 그 경우만 지연 후 직접 다시 연다.
      opened.addEventListener('error', () => {
        if (stopped) return;
        streamStale = true; // CONNECTING(브라우저 자동 재시도)이든 CLOSED든, 오류 순간부터 수신 재개까지는 두절이다
        if (opened.readyState !== EventSource.CLOSED) return;
        opened.close();
        timer = setTimeout(connect, RETRY_MS);
      });

      for (const name of EVENT_NAMES) {
        opened.addEventListener(name, (event) => handle(event as MessageEvent, name));
      }

      // 델타는 id가 없어 커서를 건드리지 않는다 — 유실이 계약이라 놓친 조각은 뒤따르는 확정 텍스트가 바로잡는다.
      opened.addEventListener('turn.delta', (event) => {
        lastSeenAt = Date.now();
        lastDeltaAt = Date.now();
        lastProgressAt = Date.now();
        streamStale = false;
        turnLive = applyDelta(turnLive, parseFrame((event as MessageEvent).data));
      });

      // 하트비트(15초 주기, prism sse.ts)는 생존 신호다 — 워치독의 기준 시각만 갱신한다.
      opened.addEventListener('heartbeat', () => {
        lastSeenAt = Date.now();
        streamStale = false;
      });
    };

    // 절반 열림(half-open) 워치독 — 연결만 살고 데이터가 끊긴 스트림은 EventSource가 스스로 알아채지 못해
    // 화면이 문장 중간에서 조용히 얼어붙는다(오류 이벤트 없음). 하트비트가 두 번 넘게 유실되면 끊긴 것으로
    // 보고 닫은 뒤 즉시 다시 연다 — 재생이 놓친 구간을 이어붙인다.
    const STALL_MS = 40_000;
    // 진행(이벤트·델타) 기준의 강제 재구독 창 — 하트비트는 파이프의 생존만 증명하고 구독의 생존은 증명하지
    // 못한다: 실행은 진행되는데 이벤트만 안 오는 죽은 구독 위로 하트비트가 계속 흐르면(90분 관측 — 새로고침
    // 후 진행분 대량 도착) 바이트 기준 워치독 둘 다 눈이 먼다. 리뷰가 도는 동안 이벤트·델타의 침묵은 이
    // 창을 넘지 않으므로(자식 델타도 루트로 중계된다), 넘으면 구독을 의심하고 새로 잇는다 — 재생이 커서부터
    // 이어 붙어 멀쩡한 구독이었어도 무해하다. 질문 대기는 정당한 침묵이라 창을 넓게 잡는다.
    const PROGRESS_STALL_MS = 60_000;
    const PARKED_PROGRESS_STALL_MS = 300_000;
    const watchdog = setInterval(() => {
      if (stopped) return;
      if (Date.now() - lastSeenAt >= STALL_MS) {
        lastSeenAt = Date.now();
        streamStale = true; // 수신이 재개될 때까지 유지 — 기준 시각 리셋은 재시도 폭주 방지일 뿐 생존 신호가 아니다
        source?.close();
        connect();
        return;
      }
      const parked = live.questions.some((question) => question.status === 'pending');
      if (Date.now() - lastProgressAt < (parked ? PARKED_PROGRESS_STALL_MS : PROGRESS_STALL_MS)) return;
      lastProgressAt = Date.now();
      source?.close();
      connect();
    }, 10_000);

    // 백그라운드 탭은 인터벌이 스로틀되어 워치독이 늦는다 — 탭 복귀 순간 무수신·무진행이면 기다리지 않고 바로 다시 잇는다.
    const onVisible = () => {
      if (stopped || document.visibilityState !== 'visible') return;
      const bytesStale = Date.now() - lastSeenAt >= STALL_MS;
      const parked = live.questions.some((question) => question.status === 'pending');
      const progressStale = Date.now() - lastProgressAt >= (parked ? PARKED_PROGRESS_STALL_MS : PROGRESS_STALL_MS);
      if (!bytesStale && !progressStale) return;
      lastSeenAt = Date.now();
      lastProgressAt = Date.now();
      if (bytesStale) streamStale = true;
      source?.close();
      connect();
    };
    document.addEventListener('visibilitychange', onVisible);

    connect();
    return () => {
      stopped = true;
      clearTimeout(timer);
      clearInterval(watchdog);
      document.removeEventListener('visibilitychange', onVisible);
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

  // 재리뷰 확인도 중단과 같은 형태 — 버튼은 다이얼로그만 열고 제출은 폼이 한다.
  const confirmRereview = () => {
    Dialog.confirm({
      title: '리뷰를 다시 요청할까요?',
      message: '원고의 최신 상태를 다시 불러와 새 리뷰를 시작해요 · 보통 60~90분 걸려요',
      actionLabel: '리뷰 다시 요청',
      actionHandler: () => rereviewForm?.requestSubmit(),
    });
  };

  const submitRereview: SubmitFunction = () => {
    rereviewing = true;
    return async ({ result, update }) => {
      if (result.type === 'success') Toast.success('새 리뷰를 시작했어요');
      else if (result.type === 'failure') Toast.error(String(result.data?.error ?? '리뷰를 다시 시작하지 못했어요'));
      // 재로드가 새 회차를 표시 회차로 집어 화면이 실행 중으로 바뀐다 — 기본 update가 invalidateAll을 태운다.
      // 로딩 해제는 재로드 완료 뒤다(submitResume과 같은 사유) — 응답~전환 사이에 풀면 완료 화면 위에서 연타가 된다.
      await update();
      rereviewing = false;
    };
  };

  // 재개는 비파괴라 확인 다이얼로그 없이 바로 제출한다 — 재로드가 되돌아온 running 회차를 표시 회차로 집는다.
  // 로딩 해제는 재로드 완료 뒤다 — 액션 응답과 화면 전환 사이(사영·시드 재조회 수 초)에 풀면 실패 화면 위에서
  // 버튼이 되살아나 연타가 된다. 성공이면 그 사이 실패 UI째 내려가므로 해제는 실패 반려의 재시도 몫이다.
  const submitResume: SubmitFunction = () => {
    resuming = true;
    return async ({ result, update }) => {
      if (result.type === 'success') Toast.success('멈춘 단계부터 다시 시작했어요');
      else if (result.type === 'failure') Toast.error(String(result.data?.error ?? '다시 시도 요청을 보내지 못했어요'));
      await update();
      resuming = false;
    };
  };

  // 상태 배지보다 한 급 눌러 세운다 — 코드 명칭을 그대로 쓰는 자리라 mono다(코드명 노출은 오너 결정 2026-08-17).
  const tierBadgeClass = css({
    flexShrink: '0',
    paddingX: '8px',
    paddingY: '2px',
    borderRadius: 'full',
    backgroundColor: 'surface.muted',
    fontFamily: 'mono',
    fontSize: '10px',
    letterSpacing: '0',
    fontWeight: 'semibold',
    color: 'text.faint',
  });

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

<Helmet title={tabTitle} />

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
      리뷰 · {data.review.roundNumber}회차
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
      <span class={tierBadgeClass}>{data.review.tier}</span>
      {#if data.isAdmin}
        <Button onclick={() => (infoOpen = true)} size="sm" type="button" variant="secondary">정보</Button>
        {#if data.review.status !== 'running'}
          <Button onclick={() => (diagnosticsOpen = true)} size="sm" type="button" variant="secondary">진단</Button>
        {/if}
      {/if}
      {#if artifactsAvailable}
        <Button onclick={() => (artifactsOpen = true)} size="sm" type="button" variant="secondary">산출물 보기</Button>
      {/if}
      {#if data.review.status === 'completed'}
        <Button href={`/sessions/${data.session.id}/process`} size="sm" type="link" variant="secondary">과정 보기</Button>
      {/if}
      {#if data.review.status === 'running'}
        <form bind:this={cancelForm} action="?/cancel" method="post" use:enhance={submitCancel}>
          <Button disabled={canceling} loading={canceling} onclick={confirmCancel} size="sm" type="button" variant="secondary">
            리뷰 중단
          </Button>
        </form>
      {/if}
      <!-- 거부 본체(정상 완료가 없는 세션)에는 다시 요청 자리 자체를 두지 않는다 — 새 피드백 시작이 경로다. -->
      {#if data.review.status === 'completed' && !data.review.rejection}
        {#if data.rounds.canRereview}
          <form bind:this={rereviewForm} action="?/rereview" method="post" use:enhance={submitRereview}>
            <Button disabled={rereviewing} loading={rereviewing} onclick={confirmRereview} size="sm" type="button">
              리뷰 다시 요청하기
            </Button>
          </form>
        {:else}
          <Tooltip message="아직 이 리뷰에서는 다시 요청하기를 지원하지 않아요">
            <Button disabled size="sm" type="button">리뷰 다시 요청하기</Button>
          </Tooltip>
        {/if}
      {/if}
      <ThemeToggle />
    </div>
  </header>

  <!-- 재개 폼은 표식 없이 상주한다 — 트리거(실패 패널·아래 배너)가 requestSubmit으로 쏜다. -->
  <form bind:this={resumeForm} action="?/resume" hidden method="post" use:enhance={submitResume}></form>

  <!-- 실패한 최신 회차는 화면을 빼앗지 않고 이 띠로 강등된다 — 본체는 최신 완료 회차가 맡는다(pickRounds). -->
  {#if data.rounds.failed}
    <div
      class={flex({
        align: 'center',
        gap: '8px',
        flex: 'none',
        paddingX: '16px',
        paddingY: '8px',
        borderBottomWidth: '1px',
        borderColor: 'border.subtle',
        backgroundColor: 'accent.danger.subtle',
        fontSize: '12px',
        color: 'text.danger',
      })}
    >
      <span class={css({ flexGrow: '1', minWidth: '0' })}>새 리뷰를 마치지 못했어요 — 이전 결과를 보여드리고 있어요</span>
      <!-- 중단(canceled)은 재개 대상이 아니다 — prism retry가 failed 종결만 수용한다. -->
      {#if data.rounds.failed.status === 'failed'}
        <Button
          style={{ flex: 'none' }}
          disabled={resuming}
          loading={resuming}
          onclick={() => resumeForm?.requestSubmit()}
          size="sm"
          type="button"
          variant="secondary"
        >
          멈춘 단계부터 다시 시도
        </Button>
      {/if}
    </div>
  {/if}

  <!-- 거부로 끝난 최신 재검토도 화면을 빼앗지 않는다 — 문면은 prism 정본 그대로 싣는다. -->
  {#if data.rounds.rejected}
    <div
      class={flex({
        align: 'center',
        gap: '8px',
        flex: 'none',
        paddingX: '16px',
        paddingY: '8px',
        borderBottomWidth: '1px',
        borderColor: 'border.subtle',
        backgroundColor: 'surface.muted',
        fontSize: '12px',
        color: 'text.subtle',
      })}
    >
      <span class={css({ flexGrow: '1', minWidth: '0' })}>{data.rounds.rejected.message} — 이전 결과를 보여드리고 있어요</span>
    </div>
  {/if}

  {#if data.review.status === 'completed' && data.review.rejection}
    <div class={flex({ flexGrow: '1', minHeight: '0' })}>
      <div class={css({ flexGrow: '1', minWidth: '0', overflowY: 'auto' })}>
        <div class={css({ width: 'full', maxWidth: '620px', marginX: 'auto', paddingX: '32px', paddingY: '52px' })}>
          <h1 class={css({ marginBottom: '24px', fontFamily: 'RIDIBatang', fontSize: '24px' })}>{title}</h1>
          <p class={css({ fontSize: '15px', lineHeight: '[1.8]', color: 'text.subtle' })}>{data.review.rejection.message}</p>
          {#if data.isAdmin && data.review.rejection.category}
            <p class={css({ marginTop: '16px', fontSize: '12px', color: 'text.faint' })}>
              {data.review.rejection.category}{data.review.rejection.basis ? ` — ${data.review.rejection.basis}` : ''}
            </p>
          {/if}
        </div>
      </div>
    </div>
  {:else if data.review.status === 'completed'}
    <div class={flex({ flexGrow: '1', minHeight: '0' })}>
      {#if conclusion && conclusionShown}
        <ConclusionPanel
          {activeId}
          {conclusion}
          content={data.version.content}
          {elevations}
          onActivate={(id) => activate(id, 'jump')}
          onExpand={() => (drawerOpen = true)}
          reaction={data.reaction}
          threads={conclusionThreads}
          {verdicts}
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
            round={data.review.round}
            {strengths}
            subtitle={data.version.subtitle}
            threads={data.threads}
            {title}
          />

          <ThreadColumn
            {activeId}
            comments={data.comments}
            content={data.version.content}
            locked={data.rounds.locked}
            onActivate={(threadId) => activate(threadId, 'thread')}
            patterns={conclusion?.patterns ?? []}
            priorities={conclusion?.priorities ?? []}
            round={data.review.round}
            threads={data.threads}
          />
        </div>
      </div>
    </div>

    {#if conclusion && conclusionShown}
      <ConclusionDrawer
        {activeId}
        {conclusion}
        {conclusionThreads}
        content={data.version.content}
        {elevations}
        finishedAt={data.review.finishedAt}
        onActivate={jump}
        onClose={closeDrawer}
        open={drawerOpen}
        reaction={data.reaction}
        roundNumber={data.review.roundNumber}
        {title}
        {verdicts}
      />
    {/if}
  {:else}
    <div class={flex({ flexGrow: '1', minHeight: '0' })}>
      <div class={css({ flexGrow: '1', minWidth: '0', overflowY: 'auto' })}>
        <div class={css({ width: 'full', maxWidth: '620px', marginX: 'auto', paddingX: '32px', paddingY: '52px' })}>
          <div class={css({ marginBottom: '40px' })}>
            <h1 class={css({ fontFamily: 'RIDIBatang', fontSize: '24px' })}>{title}</h1>
            {#if data.version.subtitle}
              <p class={css({ marginTop: '10px', fontFamily: 'RIDIBatang', fontSize: '18px', color: 'text.faint' })}>
                {data.version.subtitle}
              </p>
            {/if}
          </div>
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

      <RunningPanel
        {askAnswers}
        error={data.review.error}
        {lastDeltaAt}
        {live}
        onResume={() => resumeForm?.requestSubmit()}
        {resuming}
        startedAt={originAt}
        status={data.review.status}
        {streamStale}
        tier={data.review.tier}
        {turnLive}
      />
    </div>
  {/if}
</div>

{#if artifactsAvailable}
  <ArtifactsModal roundNumber={data.review.roundNumber} sessionId={data.session.id} bind:open={artifactsOpen} />
{/if}

{#if data.isAdmin}
  <InfoModal
    folds={infoFolds}
    lowerBound={infoLowerBound}
    modelConfig={data.modelConfig}
    priceTable={data.priceTable}
    refId={data.session.refId}
    running={infoRunning}
    tier={data.review.tier}
    workflowId={data.review.prismWorkflowId}
    bind:open={infoOpen}
  />
  <DiagnosticsModal diagnostics={data.review.result?.diagnostics ?? null} bind:open={diagnosticsOpen} />
{/if}
