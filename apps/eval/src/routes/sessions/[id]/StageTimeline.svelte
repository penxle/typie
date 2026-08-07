<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import IconCheck from '~icons/lucide/check';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronUp from '~icons/lucide/chevron-up';
  import IconCircleAlert from '~icons/lucide/circle-alert';
  import IconCircleSlash from '~icons/lucide/circle-slash';
  import { capsuleLabel, groupFeed, minutesBetween } from '$lib/feedback/live.ts';
  import { nestedRound, STAGES } from '$lib/feedback/stages.ts';
  import NestedReviewCard from './NestedReviewCard.svelte';
  import type { TurnLive } from '$lib/feedback/delta.ts';
  import type { FeedEntry, FeedGroup, LiveState, StageStatus, StageTiming } from '$lib/feedback/live.ts';
  import type { StageKey } from '$lib/feedback/stages.ts';

  // 종결 리뷰를 다시 그리는 과정 화면에는 흐르는 턴이 없으므로 turnLive는 선택 프롭이다.
  type Props = {
    live: LiveState;
    status: 'running' | 'completed' | 'failed' | 'canceled';
    now: number;
    error?: string | null;
    turnLive?: TurnLive | null;
  };
  const { live, status, now, error = null, turnLive = null }: Props = $props();

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

  // 스크롤 연출은 라이브 도착에만 건다 — 첫 페인트의 스냅샷 재생분까지 부드럽게 흐르면 화면이 출렁인다.
  let animated = $state(false);
  $effect(() => {
    requestAnimationFrame(() => requestAnimationFrame(() => (animated = true)));
  });

  // 라이브 줄 = 흐르는 턴의 조각을 이어붙인 문장. 이어붙일 원본이 깨졌으면 거짓 문장을 그리느니 아무것도 그리지 않는다.
  const liveText = $derived(turnLive === null || turnLive.textBroken ? '' : turnLive.text);

  // 수신과 표시의 분리 — 조각은 코얼레싱 창(200ms)과 모델의 생각 공백 탓에 뭉치로 도착한다. 도착 리듬을
  // 그대로 재생하면 화면이 우르르-멈춤-우르르로 끊기므로, 공개 커서를 rAF 루프가 백로그 비례 속도로 민다.
  // 밀린 양을 TARGET_LATENCY_MS에 걸쳐 소진하는 속도를 목표로 ADAPT_MS 시정수로 이징한다 — 뭉치는 완만한
  // 가속으로 번역되고, 상류가 어떤 리듬으로 주든 고정 속도 튜닝이 필요 없다. 상류가 IDLE_GRACE_MS 넘게
  // 조용하면 소진 지평을 SETTLE_LATENCY_MS로 줄여 꼬리를 마저 흘린다 — 문장 사이의 멈춤은 자연스러운
  // 쉼으로 남기되, 봉인 때 안 보여준 꼬리가 한꺼번에 쏟아지는 것은 줄인다.
  const TARGET_LATENCY_MS = 550;
  const SETTLE_LATENCY_MS = 250;
  const IDLE_GRACE_MS = 450;
  const ADAPT_MS = 300;
  const MIN_CPS = 15; // 잔량이 남은 동안 기는 최저 속도 — 상류가 침묵해도 꼬리가 뚝 멎지 않는다
  const MAX_CPS = 700;
  // 합류 스냅샷(첫 프레임에 누적 전체)은 애니메이션 대상이 아니다 — 시작부터 이만큼 밀려 있으면 점프한다.
  const JOIN_LAG = 400;
  // 진행 중에는 점프하지 않는다 — 몰아 도착한 뭉치는 점프가 아니라 가속으로 소화한다(점프가 곧 "한꺼번에
  // 우르르"다). 이 상한은 숨은 탭 복귀처럼 rAF가 서 있던 동안 쌓인 병리적 백로그만 자른다.
  const HARD_LAG = 2500;
  const MAX_FRAME_MS = 100; // 멈췄던 프레임이 몰아치는 dt 폭주 방지

  let revealedBoundary = $state(0); // 단어 경계로 스냅된 공개 길이 — 렌더는 이 값만 본다
  let plainUntil = $state(0); // 점프로 한꺼번에 공개된 앞부분 — 페이드 없이 평문으로 선다
  let revealed = 0; // 글자 단위 공개 커서(비반응 — 프레임마다 움직인다)
  let rate = MIN_CPS;
  let budget = 0;
  let lastTime = 0;
  let lastGrowth = 0; // 마지막으로 새 텍스트가 닿은 시각 — 유휴 판정(IDLE_GRACE)의 기준
  let prevTarget = 0;
  let prevText = '';
  let liveKey = '';
  let raf = 0;

  // 봉인 드레이너 — 확정은 마지막 조각과 거의 동시에 닿아, 라이브 줄을 그 자리에서 버리면 못 보여준 꼬리가
  // 확정 줄로 한꺼번에 나타난다. 봉인 순간 방금 확정된 라인을 넘겨받아 전용 커서로 꼬리까지 흘리고, 그동안
  // 그 확정 라인은 잠시 숨긴다(두 벌로 서면 그게 곧 이중 표시다). 라이브 페이서와 커서를 나눠 갖는 이유:
  // 다음 턴·다음 스테이지가 아무리 빨리 시작해도 꼬리는 제 카드·제 라운드에서 끝까지 흐른다.
  let drain = $state<{ lineId: number; text: string; stage: StageKey | null; round: number | null } | null>(null);
  let drainBoundary = $state(0);
  let drainPlain = 0; // 드레인 시작 시점까지는 이미 보인 부분 — 페이드 없이 그대로 선다
  let drainRevealed = 0;
  let drainRate = MIN_CPS;
  let drainBudget = 0;

  // 공개 커서가 단어 안에 서면 그 단어의 시작으로 물린다 — 단어는 통째로만 나타난다(페이드 단위). 버퍼 끝의
  // 미완 단어도 잡아둔다: 코얼레싱 창은 단어 중간에서도 끊기므로, 끝까지 왔다고 단어가 끝난 것이 아니다.
  const snapToWord = (text: string, cursor: number): number => {
    if (cursor < text.length && /\s/.test(text[cursor])) return cursor;
    const partial = /\S+$/.exec(text.slice(0, cursor));
    return partial === null ? cursor : cursor - partial[0].length;
  };

  const jumpTo = (length: number) => {
    revealed = length;
    revealedBoundary = length;
    plainUntil = length;
    budget = 0;
  };

  $effect.pre(() => {
    const key = turnLive === null ? '' : `${turnLive.agent.id}:${turnLive.turn}:${turnLive.attempt}`;
    const text = liveText;
    if (key !== liveKey) {
      const previous = prevText;
      liveKey = key;
      if (key === '') {
        // 방금 확정된 라인이 라이브 줄의 연장선일 때만 드레이너에 넘긴다 — 내용 대조가 안 되면 그대로 접는다.
        const line = live.activity.at(-1);
        const matches = line !== undefined && previous.length > 0 && (line.text.startsWith(previous) || previous.startsWith(line.text));
        if (matches && revealedBoundary < line.text.length) {
          drain = { lineId: line.id, text: line.text, stage: line.stage, round: nestedRound(line.step) };
          drainRevealed = revealedBoundary;
          drainBoundary = revealedBoundary;
          drainPlain = revealedBoundary;
          drainRate = Math.max(rate, MIN_CPS); // 라이브의 속도를 물려받아 흐름이 이어진다
          drainBudget = 0;
        }
      }
      rate = MIN_CPS;
      prevTarget = 0;
      jumpTo(0);
    }
    prevText = text;
    // 축소는 스냅샷 덮어쓰기(내용 교체)라 그대로 점프한다. 그 외 점프는 합류·병리 백로그뿐 — 진행 중 뭉치는
    // pace 루프가 가속으로 소화한다.
    if (text.length < revealed || (revealed === 0 && text.length > JOIN_LAG) || text.length - revealed > HARD_LAG) {
      jumpTo(text.length);
    }
  });

  const pace = (time: number) => {
    if (turnLive === null && drain === null) return;
    const dt = Math.min(time - lastTime, MAX_FRAME_MS);
    lastTime = time;

    if (turnLive !== null) {
      const target = liveText.length;
      if (target > prevTarget) {
        lastGrowth = time;
        prevTarget = target;
      }
      const pending = target - revealed;
      if (pending > 0) {
        const horizon = time - lastGrowth > IDLE_GRACE_MS ? SETTLE_LATENCY_MS : TARGET_LATENCY_MS;
        const desired = (pending / horizon) * 1000;
        rate += (desired - rate) * (1 - Math.exp(-dt / ADAPT_MS));
        budget += (Math.min(MAX_CPS, Math.max(MIN_CPS, rate)) * dt) / 1000;
        const step = Math.floor(budget);
        if (step > 0) {
          budget -= step;
          revealed = Math.min(target, revealed + step);
          const snapped = snapToWord(liveText, revealed);
          if (snapped > revealedBoundary) revealedBoundary = snapped;
        }
      } else {
        // 조용한 동안 속도를 바닥으로 되돌린다 — 다음 뭉치가 지난 뭉치의 속도로 첫 프레임부터 쏟아지지 않게.
        rate += (MIN_CPS - rate) * (1 - Math.exp(-dt / ADAPT_MS));
        budget = 0;
      }
      // 끝 단어 보류는 코얼레싱 창 사이(200ms)의 중간 절단만 가리는 장치다 — 유휴가 그보다 길면 버퍼 끝은
      // 완성된 문장이므로 보류를 푼다: 도구 입력이 흐르는 내내 직전 문장의 꼬리를 인질로 잡지 않는다.
      if (revealedBoundary < revealed && time - lastGrowth > IDLE_GRACE_MS) revealedBoundary = revealed;
    }

    // 드레이너 — 넘겨받은 확정 전문을 settle 지평으로 마저 흘린다. 라이브와 커서가 달라 새 턴이 시작돼도
    // 서로를 방해하지 않는다. 전문이라 더 올 것이 없으니 단어 보류 없이 글자 단위로 흐른다(타이핑 질감).
    if (drain !== null) {
      const target = drain.text.length;
      const pending = target - drainRevealed;
      const desired = (pending / SETTLE_LATENCY_MS) * 1000;
      drainRate += (desired - drainRate) * (1 - Math.exp(-dt / ADAPT_MS));
      drainBudget += (Math.min(MAX_CPS, Math.max(MIN_CPS, drainRate)) * dt) / 1000;
      const step = Math.floor(drainBudget);
      if (step > 0) {
        drainBudget -= step;
        drainRevealed = Math.min(target, drainRevealed + step);
        if (drainRevealed > drainBoundary) drainBoundary = drainRevealed;
      }
      if (drainBoundary >= target) drain = null; // 꼬리까지 흘렸다 — 확정 줄이 제자리로 돌아온다
    }

    raf = requestAnimationFrame(pace);
  };

  const liveActive = $derived(turnLive !== null || drain !== null);
  $effect(() => {
    if (!liveActive) return;
    lastTime = performance.now();
    raf = requestAnimationFrame(pace);
    return () => cancelAnimationFrame(raf);
  });

  // 단어만 스팬에 담고 공백은 밖에 둔다 — 페이드 단위가 단어이고, 스팬이 평문과 같은 inline이라 줄바꿈도
  // 평문과 동일하다(확정 평문으로 바뀌는 순간의 레이아웃 시프트 방지 — keyframes.ts reveal 참조).
  type LiveToken = { start: number; text: string; animated: boolean };

  const tokenize = (text: string, plainBefore: number): LiveToken[] => {
    const tokens: LiveToken[] = [];
    let start = 0;
    for (const part of text.split(/(\s+)/)) {
      if (part.length > 0) tokens.push({ start, text: part, animated: part.trim().length > 0 && start >= plainBefore });
      start += part.length;
    }
    return tokens;
  };

  const liveTokens = $derived(tokenize(liveText.slice(0, revealedBoundary), plainUntil));
  // 드레인 시작 전에 이미 보였던 앞부분은 평문으로 선다 — 라이브 줄에서 드레인 줄로 갈아타는 순간이 눈에 안 띈다.
  const drainTokens = $derived(drain === null ? [] : tokenize(drain.text.slice(0, drainBoundary), drainPlain));

  // 라이브 꼬리 — 지금 하는 활동 하나만 선다. 쓰기는 델타 카운트로 직접 보이고, 그 외에는 "마지막으로 화면에
  // 무언가 선 순간"부터의 경과를 생각으로 센다. 기록의 생각 틈(이벤트 사이 간격)과 같은 자를 쓰므로 숫자가
  // 리셋되는 순간마다 화면에 실제로 새 항목이 서고, 오래 생각한 숫자는 그대로 캡슐의 "N초 생각"으로 굳는다.
  // 턴 경계로 리셋하지 않는 이유이기도 하다 — 보이지 않는 사건으로 숫자가 튀면 고장처럼 읽힌다.
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

  // 헤일로 행은 공백을 메우는 존재다 — 다른 것(흐르는 발화·새 캡슐)이 진행을 보여주는 동안에는 서지 않는다.
  // null = 숨김. 쓰기 카운터만 예외로 즉시 선다: 쓰는 동안에는 화면에 움직이는 것이 이것뿐이다.
  const liveTail = $derived.by((): string | null => {
    if (writeLive !== null) return `노트를 쓰고 있어요 · ${writeLive.chars.toLocaleString('ko-KR')}자`;
    const seconds = Math.max(0, Math.floor((nowTick - lastBeat) / 1000));
    return seconds >= 2 ? `생각하는 중이에요 · ${seconds}초` : null;
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
    const smooth = animated;
    void tick().then(() => el.scrollTo({ top: el.scrollHeight, behavior: smooth ? 'smooth' : 'auto' }));
  });
  // 라이브 줄이 자라도 바닥을 따라간다 — 자라는 줄이 접힌 자리에 숨으면 타이핑이 보이지 않는다. 추종은 즉시
  // 스크롤로 한다: 단어마다 smooth 애니메이션을 겹쳐 걸면 그 겹침 자체가 덜컥임이 된다.
  $effect(() => {
    void liveTokens;
    void drainTokens;
    void liveTail;
    const el = feedEl;
    if (!el || !stick) return;
    void tick().then(() => el.scrollTo({ top: el.scrollHeight, behavior: 'auto' }));
  });

  const listClass = flex({ direction: 'column', gap: '6px' });

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

  // 본문 개폐 — 시작(pending→running)은 240ms 늦게 펼치고, 완료 접힘은 160ms 늦게 접는다.
  const revealRecipe = cva({
    base: { display: 'grid', transition: '[grid-template-rows 0.32s ease-in-out, visibility 0.32s]' },
    variants: {
      open: {
        feed: { gridTemplateRows: '[1fr]', visibility: 'visible', transitionDelay: '[0.24s]' },
        history: { gridTemplateRows: '[1fr]', visibility: 'visible' },
        closed: { gridTemplateRows: '[0fr]', visibility: 'hidden', transitionDelay: '[0.16s]' },
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
    height: '180px',
    paddingX: '14px',
    paddingTop: '10px',
    paddingBottom: '12px',
    overflowX: 'hidden',
    overflowY: 'auto',
    overscrollBehavior: 'contain',
    scrollbarWidth: 'none',
  });

  // 완료 카드의 펼친 기록도 같은 상자에 담아 길이를 붙든다 — 다만 짧은 기록에 빈 공간을 만들지 않게 상한만 잡는다.
  const historyFeedClass = css({
    maxHeight: '180px',
    paddingX: '14px',
    paddingTop: '10px',
    paddingBottom: '12px',
    overflowX: 'hidden',
    overflowY: 'auto',
    overscrollBehavior: 'contain',
    scrollbarWidth: 'none',
  });

  // 활동 캡슐 — 발화 사이에 남는 그 구간의 작업 요약. 발화보다 한 급 눌러(11px·disabled) 발화가 주인공으로 남는다.
  const capsuleClass = css({ paddingY: '2px', fontSize: '11px', lineHeight: '[1.7]', color: 'text.disabled' });

  const lastLineIdOf = (feed: FeedEntry[]): number | null => {
    for (let index = feed.length - 1; index >= 0; index -= 1) {
      const entry = feed[index];
      if (entry.kind === 'line') return entry.line.id;
    }
    return null;
  };

  // 드레인 중인 라인은 드레이너가 대신 그리는 동안 잠시 숨긴다.
  const visibleFeed = (feed: FeedEntry[]): FeedEntry[] => {
    const active = drain;
    if (active === null) return feed;
    return feed.filter((entry) => entry.kind !== 'line' || entry.line.id !== active.lineId);
  };

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
        false: { color: 'text.faint' },
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
    color: 'text.faint',
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

{#snippet drainStream()}
  {#if drainTokens.length > 0}
    <div class={css(feedLineRecipe.raw({ latest: true }))}>
      {#each drainTokens as token (token.start)}
        <span class={token.animated ? wordClass : undefined}>{token.text}</span>
      {/each}
    </div>
  {/if}
{/snippet}

{#snippet liveStream()}
  {#if liveTokens.length > 0}
    <div class={css(feedLineRecipe.raw({ latest: true }))}>
      {#each liveTokens as token (token.start)}
        <span class={token.animated ? wordClass : undefined}>{token.text}</span>
      {/each}
    </div>
  {/if}

  {#if liveTail !== null}
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
      <span class={shimmerTextClass}>{liveTail}</span>
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

<ol class={listClass}>
  {#each STAGES as stage, index (stage.key)}
    <!-- 드레이너가 이 스테이지에서 도는 동안은 done 전환을 보류한다 — 마지막 발화가 다 흐르기 전에 카드가
         접히면 꼬리가 잘린다. 접힘·글로우 소멸·요약 등장은 드레인이 끝난 다음 박자에 온다. -->
    {@const rawMark = markOf(live.stages[stage.key])}
    {@const mark = rawMark === 'done' && drain?.stage === stage.key ? 'running' : rawMark}
    {@const lines = live.activity.filter((line) => line.stage === stage.key)}
    <!-- 드레인과 라이브는 커서가 달라 다른 카드(또는 같은 카드의 위아래)에서 동시에 흐를 수 있다 — 각자
         제 스테이지·제 라운드에만 선다. -->
    {@const cardDrain = drain !== null && drain.stage === stage.key ? drain : null}
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
                      {#if group.line.id !== drain?.lineId}
                        <div class={css(feedLineRecipe.raw({ latest: group.line.id === latestId }))}>{group.line.text}</div>
                      {/if}
                    {:else if group.kind === 'capsule'}
                      <div class={capsuleClass}>{group.items.map(capsuleLabel).join(' · ')}</div>
                    {:else}
                      <NestedReviewCard
                        feed={visibleFeed(group.feed)}
                        ghost={cardDrain !== null && group.round === cardDrain.round ? drainStream : null}
                        latest={lastLineIdOf(group.feed) === latestId}
                        live={liveHere && group.round === liveRound ? liveStream : null}
                        round={group.round}
                        spent={group.span === null ? null : spent(group.span)}
                      />
                    {/if}
                  {/each}

                  <!-- 드레인 줄이 라이브 줄보다 먼저(위에) 선다 — 시간 순서 그대로다. 라운드 안에서 돌던 것은
                       각자 제 카드로 옮겨 간다. -->
                  {#if cardDrain !== null && cardDrain.round === null}
                    {@render drainStream()}
                  {/if}
                  {#if liveHere && liveRound === null}
                    {@render liveStream()}
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
