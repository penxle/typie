<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronRight from '~icons/lucide/chevron-right';
  import { targetFor } from '../../../core/evaluation.ts';
  import { reasonKinds } from '../evaluations/fields.ts';
  import { NO_REASONS, TRIAXIAL } from '../evaluations/triaxial.ts';
  import { EDITORIAL_MANIFEST } from '../manifest.ts';
  import AxisMark from './AxisMark.svelte';
  import type { RoundView } from '$lib/server/round-view.ts';
  import type { ViewItem } from '$lib/server/run-view.ts';
  import type { FieldSpec } from '../../../core/contracts.ts';
  import type { EditorialRender } from '../evaluations/fields.ts';

  type Props = { view: RoundView };
  const { view }: Props = $props();

  // 계산이 전부 "예/아니오 축이 있다"는 전제 위에 있다 — 그래서 이 화면은 평가 모듈이 소유한다.
  const confirmed = $derived(view.judgments.filter((j) => !j.draft));

  type BoolRender = Extract<EditorialRender, { kind: 'yesNo' } | { kind: 'triState' }>;
  const isBool = (field: FieldSpec) => {
    const kind = (field.render as EditorialRender).kind;
    return kind === 'yesNo' || kind === 'triState';
  };

  // 판정 축은 항목 종류마다 다르다. 키가 전역에서 겹치지 않게 지어져 있어 평평하게 모아 쓴다.
  const itemBoolFields = TRIAXIAL.stages.flatMap((s) => s.items).flatMap((t) => t.fields.filter(isBool));
  const runFields = TRIAXIAL.stages.flatMap((s) => s.run).filter(isBool);
  const boolFieldsOf = (item: ViewItem) => targetFor(TRIAXIAL, item)?.fields.filter(isBool) ?? [];

  const renderOf = (field: FieldSpec) => field.render as BoolRender;

  const kindLabels = Object.fromEntries(EDITORIAL_MANIFEST.itemKinds.map((k) => [k.key, k.label]));
  const noReasonLabels = Object.fromEntries(NO_REASONS.map((r) => [r.value, r.label]));

  // 비율은 확정 판정이 몇 건은 쌓여야 읽을 만하다 — 한두 건에서 나온 100%는 아무 뜻이 없다.
  const MIN_JUDGMENTS = 5;

  const percent = (value: number | null) => (value === null || Number.isNaN(value) ? '—' : `${Math.round(value * 100)}%`);
  const fixed = (value: number) => (Number.isNaN(value) ? '—' : value.toFixed(2));

  // 모름은 비율에서 뺀다 — 판단하지 않은 것을 어느 쪽으로도 세지 않는다.
  const tally = (values: unknown[]) => ({
    yes: values.filter((v) => v === true).length,
    no: values.filter((v) => v === false).length,
    unknown: values.filter((v) => v === 'unknown').length,
  });
  const rate = (t: { yes: number; no: number }) => (t.yes + t.no === 0 ? null : t.yes / (t.yes + t.no));

  const allEntries = $derived(confirmed.flatMap((j) => j.entries));

  const itemTallies = $derived(
    itemBoolFields.map((f) => ({ key: f.key, question: renderOf(f).question, tally: tally(allEntries.map((e) => e.payload[f.key])) })),
  );
  const runTallies = $derived(
    runFields.map((f) => ({ key: f.key, question: renderOf(f).question, tally: tally(confirmed.map((j) => j.payload[f.key])) })),
  );

  const choiceFields = TRIAXIAL.stages.flatMap((s) => s.run).filter((f) => (f.render as EditorialRender).kind === 'choice');
  const choiceTallies = $derived(
    choiceFields.map((f) => {
      const render = f.render as Extract<EditorialRender, { kind: 'choice' }>;
      return {
        key: f.key,
        question: render.question,
        counts: render.options.map((o) => ({ label: o.label, n: confirmed.filter((j) => j.payload[f.key] === o.value).length })),
      };
    }),
  );

  const reasonKindCounts = $derived(
    NO_REASONS.map((r) => ({ label: r.label, n: allEntries.filter((e) => reasonKinds(e.payload.reasonKind).includes(r.value)).length })),
  );
  // 복수 선택이라 분류별 합이 판정 수를 넘을 수 있다 — 총계는 사유가 달린 판정 수로 센다.
  const reasonTagged = $derived(allEntries.filter((e) => reasonKinds(e.payload.reasonKind).length > 0).length);

  // 평균이 아니라 분포다 — 평균만 내면 갈린 분포와 몰린 분포가 구별되지 않는다.
  const helpfulness = $derived(confirmed.map((j) => j.payload.helpfulness).filter((h): h is number => typeof h === 'number'));

  const itemById = $derived(new Map(view.runs.flatMap((r) => r.items.map((i) => [i.id, { item: i, run: r }]))));

  // 지적 번호는 판정 화면과 같아야 한다 — 실행 안에서 kind='finding'만 ord 순으로 센다.
  const numberOf = $derived(
    new Map(
      view.runs.flatMap((run) =>
        run.items
          .filter((i) => i.kind === 'finding')
          .toSorted((a, b) => a.ord - b.ord)
          .map((item, i) => [item.id, i + 1] as const),
      ),
    ),
  );

  // 사유 없는 아니오도 신고다 — 사유 있는 것만 모으면 반대가 실제보다 적어 보인다.
  const rejections = $derived(
    confirmed
      .flatMap((judgment) =>
        judgment.entries.flatMap((entry) => {
          const found = itemById.get(entry.itemId);
          const fields = found ? boolFieldsOf(found.item) : [];
          if (fields.every((f) => entry.payload[f.key] !== false)) return [];
          return [
            {
              key: `${judgment.id}:${entry.itemId}`,
              itemId: entry.itemId,
              kind: found?.item.kind ?? 'finding',
              number: numberOf.get(entry.itemId) ?? null,
              refId: found?.run.refId ?? '?',
              taskId: found?.run.taskId ?? null,
              body: found?.item.body ?? '',
              axis: found?.item.facets.axis ?? '',
              evaluator: judgment.evaluatorEmail,
              failed: Object.fromEntries(fields.map((f) => [f.key, entry.payload[f.key] === false])),
              slots: fields.map((f) => ({ label: renderOf(f).short, failed: entry.payload[f.key] === false })),
              reasonKind: reasonKinds(entry.payload.reasonKind),
              note: typeof entry.payload.note === 'string' ? entry.payload.note : null,
            },
          ];
        }),
      )
      .toSorted((a, b) => a.refId.localeCompare(b.refId)),
  );

  const reviewNotes = $derived(
    confirmed
      .map((judgment) => {
        const run = view.runs.find((r) => r.id === judgment.runId);
        return {
          key: judgment.id,
          refId: run?.refId ?? '?',
          taskId: run?.taskId ?? null,
          evaluator: judgment.evaluatorEmail,
          failed: Object.fromEntries(runFields.map((f) => [f.key, judgment.payload[f.key] === false])),
          note: typeof judgment.payload.note === 'string' ? judgment.payload.note : null,
          missed: typeof judgment.payload.missed === 'string' ? judgment.payload.missed : null,
          consistentNote: typeof judgment.payload.consistentNote === 'string' ? judgment.payload.consistentNote : null,
          researchNote: typeof judgment.payload.researchNote === 'string' ? judgment.payload.researchNote : null,
          planNote: typeof judgment.payload.planNote === 'string' ? judgment.payload.planNote : null,
          revisit: typeof judgment.payload.revisit === 'string' ? judgment.payload.revisit : null,
          artifactComment: typeof judgment.payload.artifactComment === 'string' ? judgment.payload.artifactComment : null,
          comment: typeof judgment.payload.comment === 'string' ? judgment.payload.comment : null,
        };
      })
      .filter(
        (r) =>
          Object.values(r.failed).some(Boolean) ||
          r.note ||
          r.missed ||
          r.consistentNote ||
          r.researchNote ||
          r.planNote ||
          r.revisit ||
          r.artifactComment ||
          r.comment,
      ),
  );

  // 아니오 비율 내림차순 — 오라클이 어디서 무너지는지가 먼저 보여야 한다.
  const entryNo = (entry: { itemId: string; payload: Record<string, unknown> }) => {
    const found = itemById.get(entry.itemId);
    return found ? boolFieldsOf(found.item).some((f) => entry.payload[f.key] === false) : false;
  };
  const documents = $derived(
    view.runs
      .map((run) => {
        const entries = confirmed.filter((j) => j.runId === run.id).flatMap((j) => j.entries);
        return {
          refId: run.refId,
          characterCount: run.characterCount,
          findings: run.items.filter((i) => i.kind === 'finding').length,
          judged: entries.length,
          no: entries.filter(entryNo).length,
        };
      })
      .filter((d) => d.judged > 0)
      .toSorted((a, b) => b.no / b.judged - a.no / a.judged),
  );

  type AxisFilter = 'all' | string;
  let axisFilter = $state<AxisFilter>('all');
  let evaluatorFilter = $state<string>('all');
  let openIds = $state<string[]>([]);

  const toggle = (key: string) => {
    openIds = openIds.includes(key) ? openIds.filter((k) => k !== key) : [...openIds, key];
  };

  const counts = $derived(
    Object.fromEntries([
      ['all', rejections.length],
      ...itemBoolFields.map((f) => [f.key, rejections.filter((r) => r.failed[f.key]).length] as const),
    ]),
  );

  const evaluators = $derived([...new Set(rejections.map((r) => r.evaluator))].toSorted((a, b) => a.localeCompare(b)));

  const shown = $derived(
    rejections
      .filter((r) => axisFilter === 'all' || r.failed[axisFilter])
      .filter((r) => evaluatorFilter === 'all' || r.evaluator === evaluatorFilter),
  );

  // 같은 문서의 반대를 붙여 읽어야 패턴이 보인다. 목록이 이미 문서별로 정렬돼 있어 순서대로 묶는다.
  const groups = $derived.by(() => {
    const out: { refId: string; taskId: string | null; items: typeof shown }[] = [];
    for (const r of shown) {
      const last = out.at(-1);
      if (last?.refId === r.refId) last.items.push(r);
      else out.push({ refId: r.refId, taskId: r.taskId, items: [r] });
    }
    return out;
  });

  // 선택 여부에 따라 완성된 클래스를 하나만 고른다. 두 개를 이어 붙이면 색이 어느 쪽으로
  // 정해질지 원자 클래스의 출력 순서에 달려, 선택된 칩의 글자가 배경에 묻히는 일이 생긴다.
  const chipOff = css({
    paddingX: '9px',
    paddingY: '3px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: 'full',
    fontSize: '12px',
    color: 'text.subtle',
    backgroundColor: 'surface.default',
    cursor: 'pointer',
    transition: '[background-color 0.15s ease, color 0.15s ease]',
    _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
  });
  const chipOn = css({
    paddingX: '9px',
    paddingY: '3px',
    borderWidth: '1px',
    borderColor: 'surface.dark',
    borderRadius: 'full',
    fontSize: '12px',
    color: 'text.bright',
    backgroundColor: 'surface.dark',
    cursor: 'pointer',
    fontWeight: 'medium',
  });

  const filterRowClass = flex({ align: 'center', gap: '6px', flexWrap: 'wrap' });
  const filterLabelClass = css({ width: '58px', flexShrink: '0', fontSize: '12px', color: 'text.faint' });

  const detailsSummaryClass = css({
    fontSize: '13px',
    color: 'text.subtle',
    cursor: 'pointer',
    transition: '[color 0.15s ease]',
    _hover: { color: 'text.default' },
  });

  const statCellClass = css({ paddingX: '12px', paddingY: '10px', borderWidth: '1px', borderColor: 'border.subtle', borderRadius: '8px' });
  const statLabelClass = css({ fontSize: '11px', color: 'text.faint' });
  const statValueClass = css({ marginTop: '2px', fontSize: '14px', fontWeight: 'semibold', fontVariantNumeric: 'tabular-nums' });
  const statNoteClass = css({ marginTop: '1px', fontSize: '11px', color: 'text.faint' });

  const linkClass = css({
    fontSize: '11px',
    color: 'text.faint',
    textDecoration: 'underline',
    textUnderlineOffset: '[2px]',
    _hover: { color: 'text.default' },
  });

  const entryClass = css({
    paddingY: '12px',
    borderBottomWidth: '1px',
    borderColor: 'border.subtle',
    ['&:last-child']: { borderBottomWidth: '0' },
  });

  const noteTextClass = css({ marginTop: '5px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' });
</script>

{#if confirmed.length === 0}
  <p
    class={css({
      paddingX: '14px',
      paddingY: '12px',
      borderRadius: '8px',
      backgroundColor: 'surface.subtle',
      fontSize: '14px',
      color: 'text.subtle',
    })}
  >
    아직 확정된 판정이 없습니다. 판정이 쌓이면 지표가 채워집니다.
  </p>
{:else}
  <!-- 사유가 먼저다. 확정이 몇 건뿐인 동안 백분율은 잡음이고, 어디서 헛짚었는지는
       첫 한 건부터 읽을 수 있다. 숫자를 위에 두면 없는 정밀도를 읽게 된다. -->
  <section>
    <div class={flex({ align: 'baseline', gap: '8px', flexWrap: 'wrap' })}>
      <h3 class={css({ fontSize: '13px', fontWeight: 'bold' })}>아니오로 갈린 판정</h3>
      <span class={css({ fontSize: '12px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
        {shown.length === rejections.length ? `${rejections.length}건` : `${shown.length} / ${rejections.length}건`}
      </span>
    </div>

    <div class={flex({ direction: 'column', gap: '6px', marginTop: '10px' })}>
      <div class={filterRowClass}>
        <span class={filterLabelClass}>아니오 축</span>
        <button class={axisFilter === 'all' ? chipOn : chipOff} onclick={() => (axisFilter = 'all')} type="button">
          전체
          <span class={css({ fontVariantNumeric: 'tabular-nums' })}>{counts.all}</span>
        </button>
        {#each itemBoolFields as field (field.key)}
          <button class={axisFilter === field.key ? chipOn : chipOff} onclick={() => (axisFilter = field.key)} type="button">
            {renderOf(field).short}
            <span class={css({ fontVariantNumeric: 'tabular-nums' })}>{counts[field.key]}</span>
          </button>
        {/each}
      </div>

      {#if evaluators.length > 1}
        <div class={filterRowClass}>
          <span class={filterLabelClass}>평가자</span>
          <button class={evaluatorFilter === 'all' ? chipOn : chipOff} onclick={() => (evaluatorFilter = 'all')} type="button">전체</button>
          {#each evaluators as email (email)}
            <button class={evaluatorFilter === email ? chipOn : chipOff} onclick={() => (evaluatorFilter = email)} type="button">
              {email}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    {#if shown.length === 0}
      <p class={css({ marginTop: '12px', fontSize: '13px', color: 'text.faint' })}>
        {rejections.length === 0 ? '아직 아니오로 갈린 판정이 없습니다.' : '이 조건에 해당하는 판정이 없습니다.'}
      </p>
    {:else}
      <div class={flex({ direction: 'column', gap: '20px', marginTop: '14px' })}>
        {#each groups as group (group.refId)}
          <section>
            <p
              class={css({
                paddingBottom: '4px',
                borderBottomWidth: '1px',
                borderColor: 'border.default',
                fontSize: '12px',
                color: 'text.subtle',
                fontVariantNumeric: 'tabular-nums',
              })}
            >
              {#if group.taskId}
                <a
                  class={css({ textDecoration: 'underline', textUnderlineOffset: '[2px]', _hover: { color: 'text.default' } })}
                  href={`/admin/tasks/${group.taskId}`}
                >
                  {group.refId}
                </a>
              {:else}
                {group.refId}
              {/if}
              <span class={css({ color: 'text.faint' })}>· {group.items.length}건</span>
            </p>

            <div class={flex({ direction: 'column' })}>
              {#each group.items as item (item.key)}
                {@const open = openIds.includes(item.key)}
                <article class={entryClass}>
                  <!-- 펼치기를 헤더 줄에 붙인다. 따로 한 줄을 내주면 항목마다 같은 문구가 반복돼
                       시선을 끌고, 한 화면에 들어오는 코멘트 수가 그만큼 줄어든다. -->
                  <div class={flex({ align: 'center', gap: '8px', flexWrap: 'wrap' })}>
                    <button
                      class={flex({
                        align: 'center',
                        gap: '4px',
                        fontSize: '12px',
                        color: 'text.subtle',
                        cursor: 'pointer',
                        _hover: { color: 'text.default' },
                      })}
                      aria-expanded={open}
                      onclick={() => toggle(item.key)}
                      title={open ? '오라클이 짚은 내용 닫기' : '오라클이 짚은 내용 펼치기'}
                      type="button"
                    >
                      <Icon style={css.raw({ color: 'text.faint' })} icon={open ? IconChevronDown : IconChevronRight} size={12} />
                      {#if item.number !== null}
                        <span class={css({ fontVariantNumeric: 'tabular-nums' })}>#{item.number}</span>
                      {:else}
                        <span>{kindLabels[item.kind] ?? item.kind}</span>
                      {/if}
                      {#if item.axis}
                        <span class={css({ color: 'text.faint' })}>{item.axis}</span>
                      {/if}
                    </button>
                    <span class={flex({ align: 'center', gap: '10px', marginLeft: 'auto' })}>
                      {#if item.taskId}
                        <a class={linkClass} href={`/admin/tasks/${item.taskId}?item=${item.itemId}`}>원문</a>
                      {/if}
                      <span class={css({ fontSize: '11px', color: 'text.faint' })}>{item.evaluator}</span>
                      <AxisMark slots={item.slots} />
                    </span>
                  </div>

                  {#if item.reasonKind.length > 0}
                    <p class={css({ marginTop: '5px', fontSize: '11px', fontWeight: 'medium', color: 'text.danger' })}>
                      {item.reasonKind.map((k) => noReasonLabels[k] ?? k).join(' · ')}
                    </p>
                  {/if}
                  {#if item.note}
                    <p class={noteTextClass}>{item.note}</p>
                  {:else if item.reasonKind.length === 0}
                    <p class={css({ marginTop: '5px', fontSize: '13px', lineHeight: '[1.75]', color: 'text.disabled' })}>
                      사유를 남기지 않았습니다
                    </p>
                  {/if}

                  {#if open}
                    <p
                      class={css({
                        marginTop: '6px',
                        paddingLeft: '10px',
                        borderLeftWidth: '2px',
                        borderColor: 'border.default',
                        fontSize: '13px',
                        lineHeight: '[1.75]',
                        color: 'text.subtle',
                        whiteSpace: 'pre-wrap',
                      })}
                    >
                      {item.body}
                    </p>
                  {/if}
                </article>
              {/each}
            </div>
          </section>
        {/each}
      </div>
    {/if}
  </section>

  <!-- 총평은 작품 하나에 대한 판단이라 지적보다 수가 적다 — 접지 않고 그대로 편다. -->
  <section class={css({ marginTop: '20px' })}>
    <div class={flex({ align: 'baseline', gap: '8px' })}>
      <h3 class={css({ fontSize: '13px', fontWeight: 'bold' })}>작품 총평</h3>
      <span class={css({ fontSize: '12px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>{reviewNotes.length}건</span>
    </div>

    {#if reviewNotes.length === 0}
      <p class={css({ marginTop: '10px', fontSize: '13px', color: 'text.faint' })}>아직 총평에 남긴 말이 없습니다.</p>
    {:else}
      <div class={flex({ direction: 'column', marginTop: '10px' })}>
        {#each reviewNotes as entry (entry.key)}
          <article class={entryClass}>
            <div class={flex({ align: 'center', gap: '8px', flexWrap: 'wrap' })}>
              {#if entry.taskId}
                <a class={[linkClass, css({ fontSize: '12px', color: 'text.subtle' })]} href={`/admin/tasks/${entry.taskId}`}>
                  {entry.refId}
                </a>
              {:else}
                <span class={css({ fontSize: '12px', color: 'text.subtle', fontVariantNumeric: 'tabular-nums' })}>{entry.refId}</span>
              {/if}
              <span class={flex({ align: 'center', gap: '10px', marginLeft: 'auto' })}>
                <span class={css({ fontSize: '11px', color: 'text.faint' })}>{entry.evaluator}</span>
                <AxisMark slots={runFields.map((f) => ({ label: renderOf(f).short, failed: entry.failed[f.key] }))} />
              </span>
            </div>

            {#if entry.note}
              <p class={noteTextClass}>{entry.note}</p>
            {/if}

            {#if entry.missed}
              <p class={css({ marginTop: entry.note ? '6px' : '5px', fontSize: '11px', color: 'text.faint' })}>놓친 것</p>
              <p class={css({ marginTop: '1px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' })}>{entry.missed}</p>
            {/if}

            {#if entry.consistentNote}
              <p class={css({ marginTop: '6px', fontSize: '11px', color: 'text.faint' })}>충돌 사유</p>
              <p class={css({ marginTop: '1px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' })}>
                {entry.consistentNote}
              </p>
            {/if}

            {#if entry.researchNote}
              <p class={css({ marginTop: '6px', fontSize: '11px', color: 'text.faint' })}>리서치 사유</p>
              <p class={css({ marginTop: '1px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' })}>{entry.researchNote}</p>
            {/if}

            {#if entry.planNote}
              <p class={css({ marginTop: '6px', fontSize: '11px', color: 'text.faint' })}>계획 사유</p>
              <p class={css({ marginTop: '1px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' })}>{entry.planNote}</p>
            {/if}

            {#if entry.revisit}
              <p class={css({ marginTop: '6px', fontSize: '11px', color: 'text.faint' })}>바꾸고 싶어진 판정</p>
              <p class={css({ marginTop: '1px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' })}>{entry.revisit}</p>
            {/if}

            {#if entry.artifactComment}
              <p class={css({ marginTop: '6px', fontSize: '11px', color: 'text.faint' })}>리서치·계획 코멘트</p>
              <p class={css({ marginTop: '1px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' })}>
                {entry.artifactComment}
              </p>
            {/if}

            {#if entry.comment}
              <!-- 판정 전체에 남긴 말은 총평 사유와 다른 층위다 — 라벨로 구분해 섞이지 않게 한다. -->
              <p class={css({ marginTop: entry.note ? '6px' : '5px', fontSize: '11px', color: 'text.faint' })}>이 글 전체에 대해</p>
              <p class={css({ marginTop: '1px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' })}>{entry.comment}</p>
            {/if}
          </article>
        {/each}
      </div>
    {/if}
  </section>

  <details class={css({ marginTop: '20px' })}>
    <summary class={detailsSummaryClass}>집계 지표</summary>

    {#if confirmed.length < MIN_JUDGMENTS}
      <p class={css({ marginTop: '10px', fontSize: '12px', color: 'accent.warning.default' })}>
        확정 {confirmed.length}건 — 표본이 적어 아래 비율은 아직 흔들립니다.
      </p>
    {/if}

    <div class={grid({ columns: { base: 2, md: 3 }, gap: '8px', marginTop: '12px' })}>
      {#each itemTallies as row (row.key)}
        <div class={statCellClass}>
          <p class={statLabelClass}>{row.question}</p>
          <p class={statValueClass}>{percent(rate(row.tally))}</p>
          <p class={statNoteClass}>
            예 {row.tally.yes} · 아니오 {row.tally.no}{#if row.tally.unknown > 0}&nbsp;· 모름 {row.tally.unknown}{/if}
          </p>
        </div>
      {/each}
    </div>

    <div class={grid({ columns: { base: 2, md: 3 }, gap: '8px', marginTop: '8px' })}>
      {#each runTallies as row (row.key)}
        <div class={statCellClass}>
          <p class={statLabelClass}>{row.question}</p>
          <p class={statValueClass}>{percent(rate(row.tally))}</p>
          <p class={statNoteClass}>예 {row.tally.yes} · 아니오 {row.tally.no}</p>
        </div>
      {/each}
      <div class={statCellClass}>
        <p class={statLabelClass}>도움이 되었을까</p>
        <p class={statValueClass}>
          {helpfulness.length === 0 ? '—' : fixed(helpfulness.reduce((x, y) => x + y, 0) / helpfulness.length)}
        </p>
        <p class={statNoteClass}>
          {helpfulness.length === 0 ? '판정 없음' : [1, 2, 3, 4, 5].map((n) => helpfulness.filter((h) => h === n).length).join(' / ')}
        </p>
      </div>
      {#each choiceTallies as row (row.key)}
        <div class={statCellClass}>
          <p class={statLabelClass}>{row.question}</p>
          <p class={statValueClass}>{row.counts.reduce((x, c) => x + c.n, 0)}건</p>
          <p class={statNoteClass}>{row.counts.map((c) => `${c.label} ${c.n}`).join(' · ')}</p>
        </div>
      {/each}
      {#if reasonKindCounts.some((r) => r.n > 0)}
        <div class={statCellClass}>
          <p class={statLabelClass}>아니오 사유 분류</p>
          <p class={statValueClass}>{reasonTagged}건</p>
          <p class={statNoteClass}>
            {reasonKindCounts
              .filter((r) => r.n > 0)
              .map((r) => `${r.label} ${r.n}`)
              .join(' · ')}
          </p>
        </div>
      {/if}
    </div>

    {#if documents.length > 0}
      <div class={css({ marginTop: '16px' })}>
        <p class={statLabelClass}>판정이 갈린 문서</p>
        <div class={flex({ direction: 'column', gap: '4px', marginTop: '6px' })}>
          {#each documents.slice(0, 5) as document (document.refId)}
            <p class={flex({ align: 'baseline', gap: '8px', fontSize: '13px' })}>
              <span class={css({ color: 'text.subtle', fontVariantNumeric: 'tabular-nums' })}>{document.refId}</span>
              <span class={css({ color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
                {document.characterCount.toLocaleString()}자 · 지적 {document.findings}건
              </span>
              <span class={css({ marginLeft: 'auto', fontWeight: 'bold', fontVariantNumeric: 'tabular-nums' })}>
                아니오 {document.no} / {document.judged}
              </span>
            </p>
          {/each}
        </div>
      </div>
    {/if}
  </details>
{/if}
