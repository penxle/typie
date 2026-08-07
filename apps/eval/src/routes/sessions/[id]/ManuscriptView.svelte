<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tick } from 'svelte';
  import { kindOf, markParagraphs } from '$lib/feedback/anchors.ts';
  import type { MarkKind, MarkSegment, MarkThread } from '$lib/feedback/anchors.ts';
  import type { Anchor } from '$lib/feedback/types.ts';
  import type { PageData } from './$types';

  type Thread = PageData['threads'][number];

  type Props = {
    title: string;
    content: string;
    threads: Thread[];
    strengths: (Anchor & { body: string | null })[];
    activeId: string | null;
    onActivate: (threadId: string) => void;
  };

  const { title, content, threads, strengths, activeId, onActivate }: Props = $props();

  // 잘 작동하는 대목도 지적과 같은 문법으로 원고에 선다(오너 결정) — 스팬·레일·번호 칩·클릭 활성화를
  // 그대로 타되, id는 'strength.N' 네임스페이스로 스레드와 갈라지고 표기 계열만 긍정(초록)이다.
  // 번호는 각자 제 목록 안에서 센다 — 지적 번호는 카드와, 강점 번호는 패널의 나열 순서와 짝이다.
  type Markable = MarkThread & { number: number };

  const marks = $derived<Markable[]>([
    ...threads.map((thread) => ({ ...thread, number: thread.issueIndex + 1 })),
    ...strengths.map((strength, index) => ({
      id: `strength.${index}`,
      pass: 'strength' as const,
      state: 'open',
      anchors: [{ start: strength.start, end: strength.end, head: strength.head, tail: strength.tail }],
      number: index + 1,
    })),
  ]);

  const paragraphs = $derived(markParagraphs(content, marks));

  // 지적 표기는 앵커 길이 불문 레일 하나다 — 본문을 칠하는 대신 원고 왼쪽 전용 거터에 세로 레일을 세운다.
  // 거터는 레이아웃에 실제로 확보한 공간이다(루트 paddingLeft) — 레인이 쌓여도 본문·카드를 침범하지 않는다.
  // 레일 머리에는 카드와 같은 지적 번호 칩이 붙는다 — 반대편 카드와의 연결 어포던스는 이 번호가 담당한다.
  type Rail = { id: string; kind: MarkKind; number: number; top: number; height: number; lane: number; chipTop: number; chipLeft: number };

  const RAIL_LANE_STEP = 7;
  const RAIL_MIN_HEIGHT = 14;
  const RAIL_WIDTH = 3;
  const RAIL_ACTIVE_WIDTH = 5;
  const RAIL_CHIP_SIZE = 16;
  const RAIL_CHIP_GAP = 2; // 칩과 레일 사이 가로 간격
  // 레일 스택과 본문 사이 숨 쉴 간격. 오른쪽(본문↔카드 28px)보다 넉넉해야 한다 — 왼쪽엔 빈 여백이 아니라
  // 연속된 유색 막대가 붙어 있어 같은 거리도 더 좁게 읽힌다(오너 실감 2회 반영: 16→32→44).
  const RAIL_TEXT_GAP = 44;
  // 루트 paddingLeft와 같은 값. 폭의 근거: 가장 왼쪽 레인(5)의 왼쪽 칩 자리까지 거터 안에 들어와야 한다 —
  // GAP 44 + 막대 3 + 레인 5×7 + 칩 자리 18 = 100. 더 좁히면 깊은 레인의 칩이 왼쪽 대신 상단 폴백으로 밀린다.
  const RAIL_GUTTER = 100;
  // 레인 상한의 구속 조건은 막대가 아니라 "왼쪽 칩 자리"다 — 마지막 레인도 왼쪽 배치가 가능해야
  // 깊은 레인의 칩이 상단 폴백으로 밀리지 않는다. 현재 값으로 6레인(0~5).
  const MAX_RAIL_LANES = Math.floor((RAIL_GUTTER - RAIL_TEXT_GAP - RAIL_WIDTH - RAIL_CHIP_SIZE - RAIL_CHIP_GAP) / RAIL_LANE_STEP) + 1;

  // 레인 0이 본문에 가장 가깝고, 겹칠수록 왼쪽 바깥으로 나간다. 활성으로 굵어질 때는 중앙선을 유지한 채
  // 양쪽으로 늘어난다 — 번호 칩과의 정렬이 흔들리면 안 된다.
  const railLeft = (lane: number, active: boolean) =>
    RAIL_GUTTER - RAIL_TEXT_GAP - RAIL_WIDTH - lane * RAIL_LANE_STEP - (active ? (RAIL_ACTIVE_WIDTH - RAIL_WIDTH) / 2 : 0);

  let containerEl = $state<HTMLDivElement>();
  let rails = $state<Rail[]>([]);

  const measureRails = () => {
    if (!containerEl) return;
    const base = containerEl.getBoundingClientRect().top;
    const next: Rail[] = [];
    // 앵커 길이 불문 전 마크가 레일이다(오너 결정) — 스팬이 없는 마크(앵커 없음·해석 불가)만 빠진다.
    for (const mark of marks) {
      const spans = containerEl.querySelectorAll(`[data-thread-range~="${mark.id}"]`);
      if (spans.length === 0) continue;
      let top = Infinity;
      let bottom = -Infinity;
      for (const span of spans) {
        const rect = span.getBoundingClientRect();
        top = Math.min(top, rect.top);
        bottom = Math.max(bottom, rect.bottom);
      }
      next.push({
        id: mark.id,
        kind: kindOf(mark),
        number: mark.number,
        top: top - base,
        height: Math.max(RAIL_MIN_HEIGHT, bottom - top),
        lane: 0,
        chipTop: top - base,
        chipLeft: 0,
      });
    }
    next.sort((a, b) => a.top - b.top);
    // 겹치는 레일은 옆 레인으로 — 시작 순서대로 훑으며 비는 첫 레인을 준다. 레인이 소진되면 마지막 레인에
    // 겹쳐 싣는다 — 거터 밖(본문·페이지 여백)으로 탈출하는 것보다 겹침이 낫다.
    const laneBottoms: number[] = [];
    for (const rail of next) {
      let lane = laneBottoms.findIndex((laneBottom) => laneBottom + 6 <= rail.top);
      if (lane === -1) {
        if (laneBottoms.length < MAX_RAIL_LANES) {
          lane = laneBottoms.length;
          laneBottoms.push(0);
        } else {
          lane = MAX_RAIL_LANES - 1;
        }
      }
      laneBottoms[lane] = Math.max(laneBottoms[lane] ?? 0, rail.top + rail.height);
      rail.lane = lane;
    }
    // 번호 칩 배치 — 레일 꼭대기의 왼쪽이 비면 왼쪽, 왼쪽이 막히면 오른쪽, 양쪽 다 막히면 그때만 레일 위
    // 중앙에 얹는다(오너 결정). 막힘 판정은 다른 레일 막대·이미 놓인 칩과의 사각형 교차다.
    type Box = { left: number; top: number; right: number; bottom: number };
    const intersects = (a: Box, b: Box) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;
    const bars: Box[] = next.map((rail) => {
      const left = railLeft(rail.lane, false);
      return { left, top: rail.top, right: left + RAIL_WIDTH, bottom: rail.top + rail.height };
    });
    const chips: Box[] = [];
    for (const [index, rail] of next.entries()) {
      const railX = railLeft(rail.lane, false);
      const sides = [railX - RAIL_CHIP_SIZE - RAIL_CHIP_GAP, railX + RAIL_WIDTH + RAIL_CHIP_GAP];
      let chosen: Box | null = null;
      for (const left of sides) {
        if (left < 0) continue;
        const box = { left, top: rail.top, right: left + RAIL_CHIP_SIZE, bottom: rail.top + RAIL_CHIP_SIZE };
        const blocked = bars.some((bar, at) => at !== index && intersects(box, bar)) || chips.some((chip) => intersects(box, chip));
        if (!blocked) {
          chosen = box;
          break;
        }
      }
      if (!chosen) {
        // 양쪽 다 막힘 — 레일 위 중앙. 막대 위 얹힘은 허용하고, 칩끼리 겹치면 아래로 민다.
        const left = Math.max(0, railX + RAIL_WIDTH / 2 - RAIL_CHIP_SIZE / 2);
        let top = rail.top;
        for (;;) {
          const box = { left, top, right: left + RAIL_CHIP_SIZE, bottom: top + RAIL_CHIP_SIZE };
          if (chips.every((chip) => !intersects(box, chip))) {
            chosen = box;
            break;
          }
          top += RAIL_CHIP_SIZE + RAIL_CHIP_GAP;
        }
      }
      chips.push(chosen);
      rail.chipLeft = chosen.left;
      rail.chipTop = chosen.top;
    }
    rails = next;
  };

  $effect(() => {
    void marks;
    void tick().then(measureRails);
  });

  $effect(() => {
    // 폰트 로드·창 크기 변화로 본문이 리플로우하면 레일 좌표도 낡는다 — 컨테이너 높이 변화로 감지한다.
    const observer = new ResizeObserver(() => measureRails());
    if (containerEl) observer.observe(containerEl);
    return () => observer.disconnect();
  });

  // 본문은 평시 무표기다 — 클릭면은 레일이 맡고, 스팬은 레일·카드 배치의 측정 좌표와 활성 하이라이트만 담당한다.
  // 표기 색은 활성 카드 보더와 같은 브랜드 하나로 통일(오너 결정) — 레일·번호 칩·하이라이트가 전부 같은 색이고,
  // 강점만 긍정(초록) 계열로 갈린다. 전환(base)은 소속 스팬에 상시 얹는다 — 활성 클래스와 함께 떼면 해제
  // 방향이 전환 없이 툭 꺼진다. 하이라이트는 본문 가독을 해치면 안 된다 — 시맨틱 subtle(100/900)보다 한 단계
  // 옅은 원시 팔레트 양끝 단계(50/950)를 양 테마로 직접 짝지어 쓴다(더 옅은 시맨틱 토큰이 없다).
  const spanRecipe = cva({
    base: { borderRadius: '2px', transition: '[background-color 0.25s cubic-bezier(0.2, 0, 0, 1)]' },
    variants: {
      active: { true: {}, false: {} },
      tone: { issue: {}, strength: {} },
    },
    compoundVariants: [
      { active: true, tone: 'issue', css: { backgroundColor: 'brand.50', _dark: { backgroundColor: 'dark.brand.950' } } },
      { active: true, tone: 'strength', css: { backgroundColor: 'green.50', _dark: { backgroundColor: 'dark.green.950' } } },
    ],
  });

  const pieceAttrs = (piece: MarkSegment) => {
    if (piece.threadIds.length === 0) return {};
    const active = activeId !== null && piece.threadIds.includes(activeId);
    return {
      'data-thread-range': piece.threadIds.join(' '),
      class: css(spanRecipe.raw({ active, tone: activeId?.startsWith('strength.') ? 'strength' : 'issue' })),
    };
  };

  const paragraphClass = css({
    fontFamily: 'RIDIBatang',
    fontSize: '16px',
    lineHeight: '[1.95]',
    color: 'text.subtle',
    textIndent: '[1em]',
  });

  // 계열 색 구분은 뺀다(오너 결정) — 열린 지적은 전부 브랜드, 닫힌 지적만 회색. 강점은 별개 범주라
  // 긍정(초록) 계열을 받는다(오너 결정) — 지적 패스 간 구분 금지와 어긋나지 않는다.
  const railRecipe = cva({
    base: {
      position: 'absolute',
      width: '3px',
      borderRadius: 'full',
      cursor: 'pointer',
      opacity: '45',
      transition:
        '[opacity 0.25s cubic-bezier(0.2, 0, 0, 1), width 0.25s cubic-bezier(0.2, 0, 0, 1), left 0.25s cubic-bezier(0.2, 0, 0, 1)]',
      _hover: { opacity: '80' },
    },
    variants: {
      tone: {
        open: { backgroundColor: 'accent.brand.default' },
        closed: { backgroundColor: 'border.strong' },
        strength: { backgroundColor: 'accent.success.default' },
      },
      active: { true: { width: '5px', opacity: '100' }, false: {} },
    },
  });

  // 카드 헤더의 번호 칩과 같은 시각 언어 — 양쪽에 같은 번호가 보이는 것이 연결 어포던스다.
  // 강점 칩의 반대편 짝은 카드가 아니라 총평 패널의 나열이다.
  const railChipRecipe = cva({
    base: {
      position: 'absolute',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      size: '16px',
      borderRadius: '5px',
      fontSize: '10px',
      fontWeight: 'bold',
      cursor: 'pointer',
      transition: '[background-color 0.25s cubic-bezier(0.2, 0, 0, 1), color 0.25s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      tone: {
        open: { backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
        closed: { backgroundColor: 'surface.muted', color: 'text.faint' },
        strength: { backgroundColor: 'accent.success.subtle', color: 'text.success' },
      },
      active: { true: {}, false: {} },
    },
    // 활성 반전도 계열을 따른다 — 닫힌 스레드는 표기 전체가 회색조인데 칩만 브랜드로 뒤집히면 어긋난다.
    compoundVariants: [
      { tone: 'open', active: true, css: { backgroundColor: 'accent.brand.default', color: 'text.bright' } },
      { tone: 'closed', active: true, css: { backgroundColor: 'border.strong', color: 'text.bright' } },
      { tone: 'strength', active: true, css: { backgroundColor: 'accent.success.default', color: 'text.bright' } },
    ],
  });

  const axisOf = (threadId: string) => threads.find((thread) => thread.id === threadId)?.axis ?? '피드백';

  const labelOf = (rail: Rail) =>
    rail.kind === 'strength' ? `잘 작동하는 대목 ${rail.number}` : `피드백 ${rail.number}: ${axisOf(rail.id)}`;
</script>

<!-- 본문 폭 640은 유지하고 왼쪽에 레일 거터(100px)를 더한다 — paddingLeft와 RAIL_GUTTER는 같은 값이어야 한다. -->
<div bind:this={containerEl} class={css({ position: 'relative', width: '740px', flex: 'none', paddingLeft: '100px' })}>
  <h1 class={css({ marginBottom: '40px', fontFamily: 'RIDIBatang', fontSize: '24px' })}>{title}</h1>

  <div class={flex({ direction: 'column', gap: '16px' })}>
    {#each paragraphs as pieces, index (index)}
      <p class={paragraphClass}>
        {#each pieces as piece, at (at)}<span {...pieceAttrs(piece)}>{piece.text}</span>{/each}
      </p>
    {/each}
  </div>

  {#each rails as rail (rail.id)}
    {@const tone = rail.kind === 'closed' ? 'closed' : rail.kind === 'strength' ? 'strength' : 'open'}
    {@const active = activeId === rail.id}
    <button
      style:top={`${rail.top}px`}
      style:height={`${rail.height}px`}
      style:left={`${railLeft(rail.lane, active)}px`}
      class={css(railRecipe.raw({ tone, active }))}
      aria-label={labelOf(rail)}
      onclick={() => onActivate(rail.id)}
      type="button"
    ></button>
    <button
      style:top={`${rail.chipTop}px`}
      style:left={`${rail.chipLeft}px`}
      class={css(railChipRecipe.raw({ tone, active }))}
      aria-label={labelOf(rail)}
      onclick={() => onActivate(rail.id)}
      type="button"
    >
      {rail.number}
    </button>
  {/each}
</div>
