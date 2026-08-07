<script lang="ts">
  import { css, cva, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { tick, untrack } from 'svelte';
  import { fade } from 'svelte/transition';
  import IconCheck from '~icons/lucide/check';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronUp from '~icons/lucide/chevron-up';
  import IconCircleAlert from '~icons/lucide/circle-alert';
  import IconCircleSlash from '~icons/lucide/circle-slash';
  import { DrainFleet } from '$lib/feedback/drain-fleet.ts';
  import { capsuleLabel, durationLabel, groupFeed, minutesBetween } from '$lib/feedback/live.ts';
  import { nestedRound, STAGES } from '$lib/feedback/stages.ts';
  import { Typewriter } from '$lib/feedback/typewriter.ts';
  import NestedReviewCard from './NestedReviewCard.svelte';
  import type { TurnLive } from '$lib/feedback/delta.ts';
  import type { FeedEntry, FeedGroup, LiveState, StageStatus, StageTiming } from '$lib/feedback/live.ts';
  import type { StageKey } from '$lib/feedback/stages.ts';
  import type { Token } from '$lib/feedback/typewriter.ts';

  // 종결 리뷰를 다시 그리는 과정 화면에는 흐르는 턴이 없으므로 turnLive는 선택 프롭이다.
  type Props = {
    live: LiveState;
    status: 'running' | 'completed' | 'failed' | 'canceled';
    now: number;
    error?: string | null;
    turnLive?: TurnLive | null;
  };
  const { live, status, now, error = null, turnLive = null }: Props = $props();

  // ── 카드 상태 판정 ──────────────────────────────────────────────────────────────────────────────

  // 종결 리뷰는 마지막으로 돌던 스테이지에 종결 사유를 얹는다 — 완료는 그 자리를 닫고, 실패·취소는 멈춘 자리로 남긴다.
  const markOf = (state: StageStatus): StageStatus | 'failed' | 'canceled' => {
    if (state !== 'running' || status === 'running') return state;
    return status === 'completed' ? 'done' : status;
  };

  let expanded = $state<Record<string, boolean>>({});

  const spent = (timing: StageTiming) => {
    if (timing.firstAt === null || timing.lastAt === null) return null;
    return timing.lastAt - timing.firstAt < 60_000 ? '1분 미만' : `${minutesBetween(timing.firstAt, timing.lastAt)}분`;
  };

  const running = (timing: StageTiming) => (timing.firstAt === null ? null : `${minutesBetween(timing.firstAt, now)}분째`);

  const stalled = (timing: StageTiming) => {
    if (timing.firstAt === null || timing.lastAt === null) return '멈췄어요';
    const minutes = minutesBetween(timing.firstAt, timing.lastAt);
    return minutes < 1 ? '멈췄어요' : `${minutes}분 만에 멈췄어요`;
  };

  const clock = (at: number | null) => {
    if (at === null) return null;
    const date = new Date(at);
    return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;
  };

  // ── 스트리밍 엔진 배선 ──────────────────────────────────────────────────────────────────────────
  // 페이싱 수리는 전부 Typewriter(lib/feedback/typewriter.ts)의 몫이고, 여기는 배선만 한다:
  // 어느 텍스트를 따라가고(retarget), 봉인 때 무엇을 드레이너로 넘기며(핸드오프), 프레임을 언제 돌리는가.
  // 엔진은 비반응이라 frame 틱 하나가 화면 반응성을 씌운다 — 경계가 움직인 프레임에만 오른다.

  // 라이브 줄 = 흐르는 턴의 조각을 이어붙인 문장. 이어붙일 원본이 깨졌으면 거짓 문장을 그리느니 아무것도 그리지 않는다.
  const liveText = $derived(turnLive === null || turnLive.textBroken ? '' : turnLive.text);

  // 봉인 드레인 함대(lib/feedback/drain-fleet.ts) — 봉인된 마지막 발화의 꼬리를 자연 속도로 마저 흘리고,
  // 완주 후 링거(읽을 시간)까지 지나야 걷힌다. 그동안 그 확정 라인은 잠시 숨긴다(두 벌로 서면 이중 표시다).
  // stage·round는 렌더 라우팅 몫 — 드레인 중에 스테이지가 넘어가도 꼬리는 제 카드·제 라운드에서 흐르고,
  // 카드 경계에서는 두 카드가 동시에 활성일 수 있다(의도된 동작).
  const MAX_FRAME_MS = 100; // 멈췄던 프레임이 몰아치는 dt 폭주 방지

  const fleet = new DrainFleet();
  let liveTw = new Typewriter('stream');
  let frame = $state(0);
  let liveKey = '';

  $effect.pre(() => {
    const key = turnLive === null ? '' : `${turnLive.agent.id}:${turnLive.turn}:${turnLive.attempt}`;
    const text = liveText;
    const at = performance.now();
    if (key !== liveKey) {
      liveKey = key;
      if (key === '') {
        // untrack: 이 효과의 의존은 "따라갈 턴"뿐이다 — 봉인 순간의 피드 열람이 의존으로 등록되면 무관한
        // 갱신마다 재실행된다.
        untrack(() => {
          const line = live.activity.at(-1);
          if (line !== undefined) {
            fleet.seal(
              { id: line.id, text: line.text, stage: line.stage, round: nestedRound(line.step) },
              liveTw.text,
              liveTw.boundary,
              liveTw.rate,
              at,
            );
          }
        });
      }
      liveTw = new Typewriter('stream'); // 턴마다 새 기계 — 이월할 상태가 없다
    }
    liveTw.retarget(text, at);
    // untrack: `frame += 1`은 자기 값을 읽으며 쓰는 자기 의존이라 효과가 스스로를 무한 재트리거한다
    // (effect_update_depth_exceeded). 읽기를 추적 밖으로 빼 쓰기 전용으로 만든다.
    untrack(() => (frame += 1)); // 봉인·리셋·점프도 다음 페인트에 바로 선다
  });

  const fleetActive = $derived.by(() => {
    void frame;
    return fleet.active;
  });
  const liveActive = $derived(turnLive !== null || fleetActive);

  // 루프 수명은 이 효과가 통째로 소유한다 — 콜백 안에서 상태를 검사해 자멸하는 조기 return을 두지 않는다
  // (루프가 죽은 채 드레인이 남으면 화면이 문장 중간에서 얼어붙는다). disposed 플래그와 지역 id로 티어다운
  // 경합도 차단한다. 함대가 비고 턴도 없으면 fleetActive가 꺼져 여기 티어다운이 루프를 걷는다.
  $effect(() => {
    if (!liveActive) return;
    let disposed = false;
    let last = performance.now();
    let id = requestAnimationFrame(function loop(time) {
      if (disposed) return;
      const dt = Math.min(time - last, MAX_FRAME_MS);
      last = time;
      let changed = turnLive !== null && liveTw.advance(time, dt);
      changed = fleet.advance(time, dt) || changed;
      if (changed) frame += 1; // rAF 콜백은 추적 문맥이 아니라 자기 의존이 생기지 않는다
      id = requestAnimationFrame(loop);
    });
    return () => {
      disposed = true;
      cancelAnimationFrame(id);
    };
  });

  const liveTokens = $derived.by((): Token[] => {
    void frame;
    return liveTw.tokens();
  });

  type DrainView = { lineId: number; stage: StageKey | null; round: number | null; tokens: Token[] };
  const drainViews = $derived.by((): DrainView[] => {
    void frame;
    return fleet.slots.map((slot) => ({ lineId: slot.lineId, stage: slot.stage, round: slot.round, tokens: slot.tw.tokens() }));
  });

  // 드레인 중인 라인은 확정 평문 대신 드레이너의 토큰으로 "그 자리에서" 그린다 — 자리를 옮기면(피드 끝 등)
  // 나중 커서 항목(검수 카드·캡슐)이 위로 갔다가 드레인이 끝날 때 순서가 되돌아오는 점프가 생긴다.
  const drainedIds = $derived.by(() => {
    void frame;
    return fleet.lineIds();
  });

  const lastLineIdOf = (feed: FeedEntry[]): number | null => {
    for (let index = feed.length - 1; index >= 0; index -= 1) {
      const entry = feed[index];
      if (entry.kind === 'line') return entry.line.id;
    }
    return null;
  };

  // ── 라이브 꼬리(헤일로 행) ──────────────────────────────────────────────────────────────────────
  // 공백을 메우는 존재다 — 다른 것(흐르는 발화·새 캡슐)이 진행을 보여주는 동안에는 서지 않는다. 쓰기 카운터만
  // 예외로 즉시 선다: 쓰는 동안에는 화면에 움직이는 것이 이것뿐이다. 생각 경과는 "마지막으로 화면에 무언가 선
  // 순간"부터 센다 — 기록의 생각 틈(이벤트 사이 간격)과 같은 자라, 숫자가 리셋되는 순간마다 화면에 실제로 새
  // 항목이 서고, 오래 생각한 숫자는 그대로 캡슐의 "N초 생각함"으로 굳는다. 턴 경계로 리셋하지 않는 이유이기도
  // 하다 — 보이지 않는 사건으로 숫자가 튀면 고장처럼 읽힌다.

  let writeLive = $state<{ chars: number } | null>(null);
  let lastToolChars: number | null = null;
  let activityKey = '';

  $effect.pre(() => {
    const key = turnLive === null ? '' : `${turnLive.agent.id}:${turnLive.turn}:${turnLive.attempt}`;
    if (key !== activityKey) {
      activityKey = key;
      writeLive = null;
      lastToolChars = null;
    }
    if (turnLive === null) return;
    // 도구 입력은 턴이 끝날 때까지 지워지지 않는다 — 글자 수가 자라는 동안만 쓰기로 읽는다.
    const input = turnLive.toolInput;
    const previous = lastToolChars;
    lastToolChars = input === null ? null : input.chars;
    if (input !== null && (input.tool === 'write' || input.tool === 'edit') && input.chars !== previous) {
      writeLive = { chars: input.chars };
    }
  });

  // 박자 = 발화 라인·캡슐 자취의 도착, 그리고 흐르는 발화의 성장. 발화가 흐르는 동안은 박자가 계속 리셋되어
  // 카운터가 0에 머문다 — 말하는 중에 "N초째 생각"이 오르는 어긋남을 막는다.
  let lastBeat = $state(Date.now());
  $effect.pre(() => {
    void live.activity.length;
    void live.marks.length;
    void liveText;
    lastBeat = Date.now();
  });

  let nowTick = $state(Date.now());
  $effect(() => {
    if (status !== 'running') return;
    const timer = setInterval(() => (nowTick = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  const liveTail = $derived.by((): string | null => {
    if (writeLive !== null) return `노트를 쓰고 있어요 · ${writeLive.chars.toLocaleString('ko-KR')}자`;
    const seconds = Math.max(0, Math.floor((nowTick - lastBeat) / 1000));
    return seconds >= 2 ? `생각하는 중이에요 · ${durationLabel(seconds)}` : null;
  });

  // ── 스크롤 추종 ────────────────────────────────────────────────────────────────────────────────

  // 스크롤 연출은 라이브 도착에만 건다 — 첫 페인트의 스냅샷 재생분까지 부드럽게 흐르면 화면이 출렁인다.
  let animated = $state(false);
  $effect(() => {
    requestAnimationFrame(() => requestAnimationFrame(() => (animated = true)));
  });

  // 재생 따라잡기 동안은 카드 전환을 끄고 최종 상태로 바로 선다 — 재접속은 로그를 처음부터 이벤트 단위로
  // 다시 받으므로, 그 몇백 ms를 전환과 함께 재생하면 첫 카드가 열렸다 닫히는 과거사 빨리감기가 보인다.
  // 따라잡기의 끝은 도착이 뜸해진 순간(250ms 무이벤트)으로 근사한다.
  let settled = $state(false);
  $effect(() => {
    void live.cursor;
    if (untrack(() => settled)) return;
    const timer = setTimeout(() => (settled = true), 250);
    return () => clearTimeout(timer);
  });

  // 기록 영역 바닥 추종 — 사용자가 위로 스크롤해 둔 동안에는 멈추고, 바닥 근처(≤8px)로 돌아오면 재개한다.
  // 추종 자체도 scroll 이벤트를 내므로 방향으로 가른다: 위로 움직였을 때만 사용자 개입이다.
  let feedEl = $state<HTMLDivElement>();
  let stick = true;
  let lastScrollTop = 0;
  const onFeedScroll = () => {
    if (!feedEl) return;
    const top = feedEl.scrollTop;
    const atBottom = feedEl.scrollHeight - top - feedEl.clientHeight <= 8;
    if (top < lastScrollTop) stick = atBottom;
    else if (atBottom) stick = true;
    lastScrollTop = top;
  };
  $effect(() => {
    // 스테이지가 넘어가 기록 영역이 새로 열리면 이전 개입은 잊는다.
    if (!feedEl) return;
    stick = true;
    lastScrollTop = 0;
  });
  $effect(() => {
    void live.activity;
    const el = feedEl;
    if (!el || !stick) return;
    const smooth = animated && settled;
    void tick().then(() => el.scrollTo({ top: el.scrollHeight, behavior: smooth ? 'smooth' : 'auto' }));
  });
  // 라이브·드레인 줄이 자라도 바닥을 따라간다 — 자라는 줄이 접힌 자리에 숨으면 타이핑이 보이지 않는다. 추종은
  // 즉시 스크롤로 한다: 단어마다 smooth 애니메이션을 겹쳐 걸면 그 겹침 자체가 덜컥임이 된다.
  $effect(() => {
    void frame;
    void liveTail;
    const el = feedEl;
    if (!el || !stick) return;
    void tick().then(() => el.scrollTo({ top: el.scrollHeight, behavior: 'auto' }));
  });

  // ── 스타일 ─────────────────────────────────────────────────────────────────────────────────────

  const listClass = flex({ direction: 'column', gap: '6px' });
  // 따라잡기 중에는 하위 전체의 CSS 전환을 죽인다 — 애니메이션(shimmer·breathe)은 상태 재생과 무관하니 살린다.
  const settlingClass = css({ '& *': { transition: '[none !important]' } });

  // 진행 중 카드의 1px 그라데이션 링 — 카드 뒤에 깔리고, 완료 시 페이드아웃(480ms)한다.
  const glowRecipe = cva({
    base: {
      position: 'absolute',
      inset: '[-1px]',
      borderRadius: '12px',
      background:
        '[linear-gradient(120deg, token(colors.brand.300), token(colors.brand.200) 35%, token(colors.brand.400) 60%, token(colors.brand.200))]',
      backgroundSize: '[250% 100%]',
      animation: 'shimmer 8s linear infinite',
      boxShadow: '[0 0 14px rgba(108,111,200,.14), 0 1px 4px rgba(108,111,200,.08)]',
      transition: '[opacity 0.48s ease]',
      _dark: {
        background:
          '[linear-gradient(120deg, token(colors.dark.brand.100), token(colors.dark.brand.400) 35%, token(colors.dark.brand.50) 60%, token(colors.dark.brand.400))]',
        boxShadow: '[0 0 14px rgba(108,111,200,.28), 0 1px 4px rgba(108,111,200,.16)]',
      },
    },
    variants: {
      shown: {
        true: { opacity: '100' },
        false: { opacity: '0' },
      },
    },
  });

  const cardRecipe = cva({
    base: {
      position: 'relative',
      borderWidth: '1px',
      borderRadius: '11px',
      backgroundColor: 'surface.default',
      transition: '[border-color 0.48s ease, background-color 0.48s ease]',
    },
    variants: {
      mark: {
        pending: { borderStyle: 'dashed', borderColor: 'border.default' },
        running: { borderStyle: 'dashed', borderColor: 'transparent' },
        done: { borderColor: 'border.subtle' },
        failed: { borderColor: 'red.200', backgroundColor: 'accent.danger.subtle', _dark: { borderColor: 'dark.red.700' } },
        canceled: { borderColor: 'border.default', backgroundColor: 'surface.muted' },
      },
    },
  });

  // 뒤로 밀려 있는 대기 카드일수록 옅어진다. 스테이지가 넘어가면 한 칸씩 밝아진다.
  const slotRecipe = cva({
    base: { position: 'relative', transition: '[opacity 0.48s ease]' },
    variants: {
      depth: {
        0: {},
        1: { opacity: '65' },
        2: { opacity: '45' },
      },
    },
  });

  const headerRecipe = cva({
    base: { display: 'flex', alignItems: 'center', gap: '8px', width: 'full', paddingX: '14px', paddingY: '10px', textAlign: 'left' },
    variants: {
      clickable: {
        true: { cursor: 'pointer' },
        false: {},
      },
    },
  });

  // 아이콘은 한 자리에 겹쳐 두고 상태 전환 때 크로스페이드한다 — 나타나는 쪽만 120ms 늦게 켠다.
  const iconLayerRecipe = cva({
    base: {
      position: 'absolute',
      inset: '0',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      transition: '[opacity 0.24s ease]',
    },
    variants: {
      shown: {
        true: { opacity: '100', transitionDelay: '[0.12s]' },
        false: { opacity: '0', transitionDelay: '[0s]' },
      },
    },
  });

  const nameRecipe = cva({
    base: { flex: 'none', transition: '[color 0.3s ease, font-size 0.3s ease, font-weight 0.3s ease]' },
    variants: {
      mark: {
        pending: { fontSize: '13px', fontWeight: 'medium', color: 'text.disabled' },
        running: { fontSize: '13px', fontWeight: 'bold', color: 'text.default' },
        done: { fontSize: '13px', fontWeight: 'semibold', color: 'text.muted' },
        failed: { fontSize: '13px', fontWeight: 'bold', color: 'text.default' },
        canceled: { fontSize: '13px', fontWeight: 'semibold', color: 'text.muted' },
      },
    },
  });

  // 요약문은 완료 전환의 마지막 박자에 나타난다(320ms, 딜레이 200ms).
  const summaryRecipe = cva({
    base: {
      flexGrow: '1',
      minWidth: '0',
      fontSize: '11px',
      color: 'text.disabled',
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      transition: '[opacity 0.32s ease 0.2s]',
    },
    variants: {
      shown: {
        true: { opacity: '100' },
        false: { opacity: '0' },
      },
    },
  });

  // 본문 개폐 — 완료 페이지 스레드 카드와 같은 문법(ThreadCard revealRecipe): grid-rows 0fr↔1fr을
  // 0.25s·cubic-bezier(0.2,0,0,1) 한 곡선으로 돌리고 지연을 두지 않는다. 닫히는 카드의 수축과 열리는
  // 카드의 성장이 같은 창에서 상보적으로 달려 목록 높이가 한 번에 변한다 — 지연을 섞으면 위상이 어긋난
  // 두 높이 변화가 겹쳐 목록이 두 번 출렁인다. visibility는 접힌 콘텐츠의 포커스·클릭을 막는 용도로,
  // 전환 목록에 실어 닫히는 동안에는 보이게 둔다.
  const revealRecipe = cva({
    base: { display: 'grid', transition: '[grid-template-rows 0.25s cubic-bezier(0.2, 0, 0, 1), visibility 0.25s]' },
    variants: {
      open: {
        feed: { gridTemplateRows: '[1fr]' },
        history: { gridTemplateRows: '[1fr]' },
        closed: { gridTemplateRows: '[0fr]', visibility: 'hidden' },
      },
    },
  });

  const revealInnerClass = css({ overflow: 'hidden', minHeight: '0' });
  const bodyClass = css({
    paddingX: '14px',
    paddingTop: '10px',
    paddingBottom: '12px',
    borderTopWidth: '1px',
    borderColor: 'border.subtle',
  });

  // 피드는 패딩을 스크롤 컨테이너 안쪽에 둔다 — 스크롤 영역이 카드 모서리까지 닿고, 안쪽 여백(14px) 덕에
  // 헤일로의 블러 글로우도 overflow 경계에 잘리지 않는다. 스크롤바는 관례대로 숨긴다(NoteEntitySearchModal 참조).
  const feedShellClass = css({ borderTopWidth: '1px', borderColor: 'border.subtle' });
  const feedClass = css({
    height: '320px',
    paddingX: '14px',
    paddingTop: '10px',
    paddingBottom: '12px',
    overflowX: 'hidden',
    overflowY: 'auto',
    scrollbarWidth: 'none',
  });

  // 완료 카드의 펼친 기록도 같은 상자에 담아 길이를 붙든다 — 다만 짧은 기록에 빈 공간을 만들지 않게 상한만 잡는다.
  const historyFeedClass = css({
    maxHeight: '320px',
    paddingX: '14px',
    paddingTop: '10px',
    paddingBottom: '12px',
    overflowX: 'hidden',
    overflowY: 'auto',
    scrollbarWidth: 'none',
  });

  // 활동 캡슐 — 발화 사이에 남는 그 구간의 작업 요약. 발화보다 한 급 눌러(11px·disabled) 발화가 주인공으로 남는다.
  const capsuleClass = css({ paddingY: '2px', fontSize: '11px', lineHeight: '[1.7]', color: 'text.disabled' });

  // 발화는 여러 줄로 온다 — 빈 줄(\n\n)까지 원문 그대로 눕히고 자르지 않는다.
  const feedLineRecipe = cva({
    base: {
      paddingY: '2px',
      fontSize: '12px',
      lineHeight: '[1.6]',
      whiteSpace: 'pre-wrap',
      transition: '[color 0.4s ease, font-weight 0.4s ease]',
    },
    variants: {
      latest: {
        true: { fontWeight: 'semibold', color: 'text.default' },
        // 지나간 발화도 읽는 본문이다 — faint는 훑기엔 옅어서 subtle로 세운다(ThreadCard 본문과 같은 급).
        // 활동 캡슐(disabled)과의 위계는 그대로 남는다.
        false: { color: 'text.subtle' },
      },
    },
  });

  // 단어 페이드인 — 공개 커서가 경계를 넘은 단어가 마운트되며 켠다. 간격은 페이싱이 만든다(별도 스태거 없음).
  const wordClass = css({ opacity: '0', animation: 'reveal 0.3s ease forwards' });

  const shimmerTextClass = css({
    fontSize: '12px',
    background: '[linear-gradient(90deg, token(colors.text.faint) 30%, token(colors.text.default) 50%, token(colors.text.faint) 70%)]',
    backgroundSize: '[200% 100%]',
    backgroundClip: 'text',
    color: 'transparent',
    animation: 'shimmer 2.2s linear infinite',
  });

  const haloGradient =
    '[conic-gradient(from 0deg, token(colors.brand.500), #8fb5f4 30%, #cfa6ee 55%, token(colors.brand.100) 75%, token(colors.brand.500))]';

  const historyTextClass = css({
    flexGrow: '1',
    minWidth: '0',
    fontSize: '11px',
    lineHeight: '[1.55]',
    whiteSpace: 'pre-wrap',
    color: 'text.subtle', // 피드의 지나간 발화와 같은 급 — 완료 후에도 읽는 본문이다
  });
  const historyTimeClass = css({ flex: 'none', fontSize: '10px', color: 'text.disabled' });

  const waitTagClass = css({
    flex: 'none',
    paddingX: '6px',
    paddingY: '1px',
    borderRadius: '4px',
    backgroundColor: 'surface.muted',
    fontSize: '10px',
    fontWeight: 'semibold',
    color: 'text.faint',
  });
</script>

{#snippet streamLine(tokens: Token[])}
  {#if tokens.length > 0}
    <div class={css(feedLineRecipe.raw({ latest: true }))}>
      {#each tokens as token (token.start)}
        <span class={token.animated ? wordClass : undefined}>{token.text}</span>
      {/each}
    </div>
  {/if}
{/snippet}

<!-- 드레인 중인 라인의 제자리 대체 — 확정 평문이 설 자리에서 드레이너의 토큰이 흐른다. -->
{#snippet ghostLine(lineId: number)}
  {@const view = drainViews.find((entry) => entry.lineId === lineId)}
  {#if view !== undefined}
    {@render streamLine(view.tokens)}
  {/if}
{/snippet}

<!-- empty = 이 카드에 아직 아무것도 없다(호출부가 제 피드 기준으로 판정). 첫 신호가 오기 전의 빈
     상자는 어색하므로 헤일로가 "준비하고 있어요"로 즉시 서고, 발화·카운터가 오면 그 자리를 넘긴다. -->
{#snippet liveStream(empty: boolean)}
  {@const tail = liveTail ?? (empty && liveTokens.length === 0 ? '준비하고 있어요' : null)}
  {@render streamLine(liveTokens)}

  {#if tail !== null}
    <div class={flex({ align: 'center', gap: '8px', height: '24px', flex: 'none' })} transition:fade={{ duration: 200 }}>
      <span class={css({ position: 'relative', size: '14px', flex: 'none' })}>
        <span
          class={css({
            position: 'absolute',
            inset: '0',
            borderRadius: 'full',
            background: haloGradient,
            animation: '[spin 2.6s linear infinite, hue-drift 5s ease-in-out infinite]',
          })}
        ></span>
        <span
          class={css({
            position: 'absolute',
            inset: '0',
            borderRadius: 'full',
            background: haloGradient,
            animation: 'spin 2.6s linear infinite',
            mask: '[radial-gradient(farthest-side, transparent calc(100% - 2.5px), #000 calc(100% - 2px))]',
          })}
        ></span>
        <span class={css({ position: 'absolute', inset: '3px', borderRadius: 'full', backgroundColor: 'surface.default' })}></span>
      </span>
      <span class={shimmerTextClass}>{tail}</span>
    </div>
  {/if}
{/snippet}

{#snippet historyLines(groups: FeedGroup[])}
  <div class={flex({ direction: 'column', gap: '7px' })}>
    {#each groups as group (group.key)}
      {#if group.kind === 'line'}
        <div class={flex({ gap: '8px' })}>
          <span class={historyTextClass}>{group.line.text}</span>
          {#if clock(group.line.at)}
            <span class={historyTimeClass}>{clock(group.line.at)}</span>
          {/if}
        </div>
      {:else if group.kind === 'capsule'}
        <div class={capsuleClass}>{group.items.map(capsuleLabel).join(' · ')}</div>
      {:else}
        <NestedReviewCard feed={group.feed} round={group.round} spent={group.span === null ? null : spent(group.span)} />
      {/if}
    {/each}
  </div>
{/snippet}

<ol class={cx(listClass, settled ? undefined : settlingClass)}>
  {#each STAGES as stage, index (stage.key)}
    <!-- 드레이너가 이 스테이지에서 도는 동안은 done 전환을 보류한다 — 마지막 발화가 다 흐르기 전에 카드가
         접히면 꼬리가 잘린다. 접힘·글로우 소멸·요약 등장은 드레인이 끝난 다음 박자에 온다. -->
    {@const rawMark = markOf(live.stages[stage.key])}
    {@const cardDrains = drainViews.filter((view) => view.stage === stage.key)}
    {@const mark = rawMark === 'done' && cardDrains.length > 0 ? 'running' : rawMark}
    {@const lines = live.activity.filter((line) => line.stage === stage.key)}
    <!-- 드레인과 라이브는 커서가 달라 다른 카드(또는 같은 카드의 위아래)에서 동시에 흐를 수 있다 — 각자
         제 스테이지·제 라운드에만 선다. -->
    {@const liveHere = mark === 'running' && live.currentStage === stage.key}
    {@const liveRound = liveHere ? nestedRound(live.currentStep) : null}
    {@const groups = groupFeed(live, stage.key)}
    {@const summary = lines.at(-1)?.text ?? ''}
    {@const toggleable = mark === 'done' && groups.length > 0}
    {@const open = mark === 'done' && toggleable && (expanded[stage.key] ?? false)}
    {@const bodyOpen = mark === 'running' ? 'feed' : mark === 'failed' || mark === 'canceled' || open ? 'history' : 'closed'}
    {@const firstPending = STAGES.findIndex((entry) => live.stages[entry.key] === 'pending')}
    {@const depth = mark === 'pending' && firstPending !== -1 ? Math.min(index - firstPending, 2) : 0}

    <li class={css(slotRecipe.raw({ depth: depth as 0 | 1 | 2 }))}>
      <span class={css(glowRecipe.raw({ shown: mark === 'running' }))} aria-hidden="true"></span>

      <div class={css(cardRecipe.raw({ mark }))}>
        <button
          class={css(headerRecipe.raw({ clickable: toggleable }))}
          aria-expanded={toggleable ? open : undefined}
          disabled={!toggleable}
          onclick={() => (expanded[stage.key] = !open)}
          type="button"
        >
          <span class={css({ position: 'relative', size: '14px', flex: 'none' })}>
            <span class={css(iconLayerRecipe.raw({ shown: mark === 'pending' }))}>
              <span
                class={css({
                  size: '13px',
                  borderWidth: '[1.5px]',
                  borderStyle: 'dashed',
                  borderColor: 'border.strong',
                  borderRadius: 'full',
                })}
              ></span>
            </span>
            <span class={css(iconLayerRecipe.raw({ shown: mark === 'running' }))}>
              <svg
                class={css({ size: '14px', fill: 'accent.brand.default', animation: 'breathe 2s ease-in-out infinite' })}
                viewBox="0 0 24 24"
              >
                <path d="M12 2l1.8 8.2L22 12l-8.2 1.8L12 22l-1.8-8.2L2 12l8.2-1.8z" />
              </svg>
            </span>
            <span class={css(iconLayerRecipe.raw({ shown: mark === 'done' }))}>
              <Icon style={css.raw({ color: 'text.success' })} icon={IconCheck} size={12} />
            </span>
            {#if mark === 'failed'}
              <span class={css(iconLayerRecipe.raw({ shown: true }))}>
                <Icon style={css.raw({ color: 'text.danger' })} icon={IconCircleAlert} size={14} />
              </span>
            {:else if mark === 'canceled'}
              <span class={css(iconLayerRecipe.raw({ shown: true }))}>
                <Icon style={css.raw({ color: 'text.faint' })} icon={IconCircleSlash} size={14} />
              </span>
            {/if}
          </span>

          <span class={css(nameRecipe.raw({ mark }))}>{stage.label}</span>

          <span class={css(summaryRecipe.raw({ shown: mark === 'done' }))}>{mark === 'done' ? summary : ''}</span>

          {#if mark === 'pending'}
            <span class={waitTagClass} transition:fade={{ duration: 240 }}>대기 중</span>
          {:else if mark === 'running'}
            {#if running(live.timing[stage.key])}
              <span class={css({ flex: 'none', fontSize: '11px', color: 'text.disabled' })} in:fade={{ duration: 320, delay: 300 }}>
                {running(live.timing[stage.key])}
              </span>
            {/if}
          {:else if mark === 'failed'}
            <span class={css({ flex: 'none', fontSize: '11px', fontWeight: 'semibold', color: 'text.danger' })}>
              {stalled(live.timing[stage.key])}
            </span>
          {:else}
            {#if spent(live.timing[stage.key])}
              <span class={css({ flex: 'none', fontSize: '10px', color: 'text.disabled' })}>{spent(live.timing[stage.key])}</span>
            {/if}
            {#if toggleable}
              <Icon style={css.raw({ flex: 'none', color: 'text.disabled' })} icon={open ? IconChevronUp : IconChevronDown} size={10} />
            {/if}
          {/if}
        </button>

        <div class={css(revealRecipe.raw({ open: bodyOpen }))}>
          <div class={revealInnerClass}>
            <div class={mark === 'failed' || mark === 'canceled' ? bodyClass : feedShellClass}>
              {#if mark === 'running'}
                {@const latestId = liveTokens.length === 0 ? (lines.at(-1)?.id ?? null) : null}
                <div bind:this={feedEl} class={feedClass} onscroll={onFeedScroll}>
                  {#each groups as group (group.key)}
                    {#if group.kind === 'line'}
                      {#if drainedIds.has(group.line.id)}
                        {@render ghostLine(group.line.id)}
                      {:else}
                        <div class={css(feedLineRecipe.raw({ latest: group.line.id === latestId }))}>{group.line.text}</div>
                      {/if}
                    {:else if group.kind === 'capsule'}
                      <div class={capsuleClass}>{group.items.map(capsuleLabel).join(' · ')}</div>
                    {:else}
                      <NestedReviewCard
                        drained={drainedIds}
                        feed={group.feed}
                        {ghostLine}
                        latest={lastLineIdOf(group.feed) === latestId}
                        live={liveHere && group.round === liveRound ? liveStream : null}
                        round={group.round}
                        spent={group.span === null ? null : spent(group.span)}
                      />
                    {/if}
                  {/each}

                  {#if liveHere && liveRound === null}
                    {@render liveStream(groups.length === 0 && cardDrains.length === 0)}
                  {/if}
                </div>
              {:else if mark === 'failed' || mark === 'canceled'}
                {#if groups.length > 0}
                  <div class={css({ marginBottom: '10px' })}>
                    {@render historyLines(groups)}
                  </div>
                {/if}
                <p class={css({ fontSize: '13px', lineHeight: '[1.6]', color: 'text.subtle' })}>
                  {mark === 'failed' ? '이 단계를 진행하던 중 문제가 생겨 리뷰가 멈췄어요.' : '여기에서 리뷰를 중단했어요.'}
                </p>
                {#if mark === 'failed' && error}
                  <p
                    class={css({
                      marginTop: '8px',
                      paddingX: '10px',
                      paddingY: '7px',
                      borderWidth: '1px',
                      borderColor: 'border.subtle',
                      borderRadius: '6px',
                      backgroundColor: 'surface.subtle',
                      fontFamily: 'mono',
                      fontSize: '10px',
                      letterSpacing: '0',
                      lineHeight: '[1.5]',
                      color: 'text.faint',
                      wordBreak: 'break-all',
                    })}
                  >
                    {error}
                  </p>
                {/if}
              {:else}
                <div class={historyFeedClass}>
                  {@render historyLines(groups)}
                </div>
              {/if}
            </div>
          </div>
        </div>
      </div>
    </li>
  {/each}
</ol>
