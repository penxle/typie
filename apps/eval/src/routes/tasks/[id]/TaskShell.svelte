<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { generationUi } from '$lib/generation-ui.ts';
  import { targetFor } from '../../../../core/evaluation.ts';
  import DocumentPane from './DocumentPane.svelte';
  import FindingRail from './FindingRail.svelte';
  import type { Snippet } from 'svelte';
  import type { GenerationUi } from '$lib/generation-ui.ts';
  import type { RunView, ViewItem } from '$lib/server/run-view.ts';
  import type { EvaluationSpec } from '../../../../core/contracts.ts';
  import type { RailMark } from './FindingRail.svelte';

  type Props = {
    view: RunView;
    evaluation: EvaluationSpec | null;
    answers: Record<string, Record<string, unknown>>;
    runAnswer: Record<string, unknown>;
    stageKey?: string | null;
    artifacts?: { label: string; value: unknown } | null;
    readOnly?: boolean;
    onItemChange?: (itemId: string, next: Record<string, unknown>) => void;
    onRunChange?: (next: Record<string, unknown>) => void;
    footer?: Snippet;
  };
  const {
    view,
    evaluation,
    answers,
    runAnswer,
    stageKey = null,
    artifacts = null,
    readOnly = false,
    onItemChange,
    onRunChange,
    footer,
  }: Props = $props();

  const ui = $derived(generationUi(view.generationId));

  // 지적에만 번호를 매긴다 — 총평의 참조와 레일이 같은 번호를 쓴다.
  const numbers = $derived(
    Object.fromEntries(
      view.items
        .filter((i) => i.kind === 'finding')
        .toSorted((a, b) => a.ord - b.ord)
        .map((item, i) => [item.id, i + 1]),
    ),
  );

  // 본문에 표시하는 것은 지적뿐이다. 총평 항목까지 칠하면 번호 없는 하이라이트가 생기고,
  // 눌러도 갈 카드가 없다 — 강점의 인용은 총평 탭에 글로 적힌다.
  const anchors = $derived(
    view.items
      .filter((i) => i.kind === 'finding')
      .flatMap((item) =>
        item.anchors
          .map((a, i) => ({ key: `${item.id}:${i}`, itemId: item.id, start: a.matchStart, end: a.matchEnd }))
          .filter((a): a is { key: string; itemId: string; start: number; end: number } => a.start !== null && a.end !== null),
      ),
  );

  let hoveredId = $state<string | null>(null);
  let focusedId = $state<string | null>(null);
  let focusedKey = $state<string | null>(null);
  let viewport = $state<{ start: number; end: number } | null>(null);
  let pane = $state<ReturnType<typeof DocumentPane> | undefined>();

  const targets = (item: ViewItem) => (evaluation ? targetFor(evaluation, item) !== null : false);

  const answered = (item: ViewItem) => {
    const target = evaluation ? targetFor(evaluation, item) : null;
    if (!target) return null;
    const payload = answers[item.id] ?? {};
    const asked = target.fields.filter((f) => f.required);
    const filled = asked.filter((f) => f.sanitize(payload[f.key]) !== null);
    const failed = target.fields.some((f) => payload[f.key] === false);
    if (failed) return 'fail' as const;
    return filled.length === asked.length ? ('seen' as const) : ('unseen' as const);
  };

  const marks = $derived<RailMark[]>(
    view.items
      .filter((i) => i.kind === 'finding')
      .flatMap((item) => {
        const first = item.anchors.find((a) => a.matchStart !== null);
        if (first?.matchStart === null || first === undefined) return [];
        const length = Math.max(1, view.document.characterCount);
        return [
          {
            itemId: item.id,
            number: numbers[item.id],
            position: Math.min(1, (first.matchStart ?? 0) / length),
            // 판정을 걸지 않는 열람에서도 눈금은 '아직 안 본 것'과 같은 굵기여야 보인다.
            state: answered(item) ?? 'unseen',
          },
        ];
      }),
  );

  let list = $state<ReturnType<GenerationUi['GenerationView']> | undefined>();

  // 목록에서 본문으로 — 원고만 옮긴다.
  const seekTo = (itemId: string, anchorIndex = 0) => {
    focusedId = itemId;
    focusedKey = `${itemId}:${anchorIndex}`;
    pane?.seek(focusedKey);
  };

  // 본문에서 목록으로 — 탭을 옮기고 카드를 세운다. 원고는 이미 그 자리이므로 건드리지 않는다.
  // 앵커가 여럿인 지적에서 0번으로 튀면 방금 누른 위치를 잃는다.
  const focusItem = (itemId: string) => {
    focusedId = itemId;
    list?.focus(itemId);
  };

  // 레일·단축키처럼 제3의 자리에서 건너뛸 때는 목록과 본문을 함께 옮긴다. 목록만 움직이면
  // 그 지적이 원고 어디를 가리키는지 직접 찾아야 해서 대조가 끊긴다.
  const reveal = (itemId: string) => {
    focusItem(itemId);
    seekTo(itemId, 0);
  };

  const findings = $derived(view.items.filter((i) => i.kind === 'finding').toSorted((a, b) => a.ord - b.ord));

  // 판정이 걸리는 항목 전부 — 지적 밖에도 필수 문항이 있어, 지적만 세면 '남음 0'인데 제출이 막힌다.
  const judgeables = $derived(view.items.filter((i) => targets(i)));

  // 집계에서 "이 지적을 왜 아니오로 봤나"를 되짚어 올 때, 그 지적과 본문 위치까지 바로 잡아준다.
  onMount(() => {
    const wanted = page.url.searchParams.get('item');
    if (!wanted || view.items.every((i) => i.id !== wanted)) return;
    reveal(wanted);
  });

  // 지적 사이 이동은 본문과 목록을 함께 옮긴다 — 마흔 건짜리 목록에서 마우스로 짝을 맞추는 일이 없도록.
  export const stepItem = (delta: number) => {
    if (findings.length === 0) return;
    const current = findings.findIndex((f) => f.id === focusedId);
    const next = findings[(current + delta + findings.length) % findings.length];
    if (!next) return;
    reveal(next.id);
  };

  export const jumpToPending = () => {
    const finding = findings.find((item) => answered(item) === 'unseen');
    if (finding) {
      reveal(finding.id);
      return;
    }
    // 총평 항목은 본문 위치가 없다 — 목록만 옮긴다.
    const next = judgeables.find((item) => answered(item) === 'unseen');
    if (!next) return;
    focusItem(next.id);
  };

  export const toggleTab = () => list?.toggleTab();

  // '남음'은 필수 문항을 다 채우지 못한 판정 항목 수다. 하나라도 비면 그 항목은 집계에 쓸 수 없다.
  export const pendingCount = () => judgeables.filter((item) => answered(item) === 'unseen').length;
</script>

{#if !ui}
  <p class={css({ padding: '24px', fontSize: '14px', color: 'text.danger' })}>
    이 세대({view.generationId ?? '알 수 없음'})의 렌더러가 제거되었습니다. 항목 본문만 확인할 수 있습니다.
  </p>
{:else}
  <div class={grid({ columns: 2, gap: '0', gridTemplateColumns: '[minmax(0, 1fr) 460px]', height: 'full', minHeight: '0' })}>
    <div class={flex({ minHeight: '0', paddingRight: '12px' })}>
      <DocumentPane
        bind:this={pane}
        {anchors}
        content={view.document.content}
        {focusedId}
        {focusedKey}
        {hoveredId}
        {numbers}
        onHover={(id) => (hoveredId = id)}
        onSelect={focusItem}
        onViewport={(next) => (viewport = next)}
      />
      {#if marks.length > 0}
        <FindingRail {marks} onSeek={(fraction) => pane?.seekFraction(fraction)} onSelect={reveal} {viewport} />
      {/if}
    </div>

    <aside
      class={css({
        display: 'flex',
        flexDirection: 'column',
        minHeight: '0',
        borderLeftWidth: '1px',
        borderColor: 'border.default',
        backgroundColor: 'surface.default',
      })}
    >
      <div class={flex({ direction: 'column', flex: '1', minHeight: '0' })}>
        {#snippet itemControl(item: ViewItem)}
          {#if targets(item) && ui}
            <ui.ItemControl
              {item}
              onchange={(next) => onItemChange?.(item.id, next)}
              {readOnly}
              {stageKey}
              value={answers[item.id] ?? {}}
            />
          {/if}
        {/snippet}

        {#snippet runReview()}
          {#if ui}
            <ui.RunReview onchange={(next) => onRunChange?.(next)} {readOnly} {stageKey} value={runAnswer} />
          {/if}
        {/snippet}

        <ui.GenerationView
          bind:this={list}
          {artifacts}
          control={evaluation ? itemControl : undefined}
          {focusedId}
          items={view.items}
          {numbers}
          onHover={(id) => (hoveredId = id)}
          onReveal={reveal}
          onSelect={seekTo}
          runReview={evaluation ? runReview : undefined}
          {stageKey}
        />
      </div>

      {#if evaluation}
        <ui.RunControl onchange={(next) => onRunChange?.(next)} {readOnly} {stageKey} value={runAnswer} />
      {/if}

      {@render footer?.()}
    </aside>
  </div>
{/if}
