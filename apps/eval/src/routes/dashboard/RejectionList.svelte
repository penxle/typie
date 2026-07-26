<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronRight from '~icons/lucide/chevron-right';
  import AxisMark from './AxisMark.svelte';
  import type { Rejection } from '$lib/domain/analysis-summary.ts';

  // 평가자가 남긴 말이 본문이다. 오라클이 짚은 내용은 대조할 때만 필요하므로 접어 둔다 —
  // 펼쳐 두면 기계의 문장이 지면을 차지해 정작 읽어야 할 사람의 말이 묻힌다.
  type Props = { rejections: Rejection[] };
  const { rejections }: Props = $props();

  type AxisFilter = 'all' | 'correct' | 'needed' | 'useful';
  let axisFilter = $state<AxisFilter>('all');
  let evaluatorFilter = $state<string>('all');
  let openIds = $state<string[]>([]);

  const keyOf = (r: Rejection) => `${r.feedbackId}:${r.evaluator}`;
  const toggle = (key: string) => {
    openIds = openIds.includes(key) ? openIds.filter((k) => k !== key) : [...openIds, key];
  };

  const counts = $derived({
    all: rejections.length,
    correct: rejections.filter((r) => r.failed.correct).length,
    needed: rejections.filter((r) => r.failed.needed).length,
    useful: rejections.filter((r) => r.failed.useful).length,
  });

  const evaluators = $derived([...new Set(rejections.map((r) => r.evaluator))].toSorted((a, b) => a.localeCompare(b)));

  const shown = $derived(
    rejections
      .filter((r) => axisFilter === 'all' || r.failed[axisFilter])
      .filter((r) => evaluatorFilter === 'all' || r.evaluator === evaluatorFilter),
  );

  // 같은 문서의 반대를 붙여 읽어야 패턴이 보인다. 목록이 이미 문서별로 정렬돼 있어 순서대로 묶는다.
  const groups = $derived.by(() => {
    const out: { refId: string; items: Rejection[] }[] = [];
    for (const r of shown) {
      const last = out.at(-1);
      if (last?.refId === r.refId) last.items.push(r);
      else out.push({ refId: r.refId, items: [r] });
    }
    return out;
  });

  const AXIS_FILTERS: { key: AxisFilter; label: string }[] = [
    { key: 'all', label: '전체' },
    { key: 'correct', label: '정확' },
    { key: 'needed', label: '가치' },
    { key: 'useful', label: '실행' },
  ];

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
</script>

<div class={flex({ align: 'baseline', gap: '8px', flexWrap: 'wrap' })}>
  <h3 class={css({ fontSize: '13px', fontWeight: 'bold' })}>아니오로 갈린 판정</h3>
  <span class={css({ fontSize: '12px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
    {shown.length === rejections.length ? `${rejections.length}건` : `${shown.length} / ${rejections.length}건`}
  </span>
</div>

<div class={flex({ direction: 'column', gap: '6px', marginTop: '10px' })}>
  <div class={filterRowClass}>
    <span class={filterLabelClass}>아니오 축</span>
    {#each AXIS_FILTERS as f (f.key)}
      <button class={axisFilter === f.key ? chipOn : chipOff} onclick={() => (axisFilter = f.key)} type="button">
        {f.label}
        <span class={css({ fontVariantNumeric: 'tabular-nums' })}>{counts[f.key]}</span>
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
          {#if group.items[0].taskId}
            <a
              class={css({ textDecoration: 'underline', textUnderlineOffset: '[2px]', _hover: { color: 'text.default' } })}
              href={`/admin/tasks/${group.items[0].taskId}`}
            >
              {group.refId}
            </a>
          {:else}
            {group.refId}
          {/if}
          <span class={css({ color: 'text.faint' })}>· {group.items.length}건</span>
        </p>

        <div class={flex({ direction: 'column' })}>
          {#each group.items as item (keyOf(item))}
            {@const open = openIds.includes(keyOf(item))}
            <article
              class={css({
                paddingY: '12px',
                borderBottomWidth: '1px',
                borderColor: 'border.subtle',
                ['&:last-child']: { borderBottomWidth: '0' },
              })}
            >
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
                  onclick={() => toggle(keyOf(item))}
                  title={open ? '오라클이 짚은 내용 닫기' : '오라클이 짚은 내용 펼치기'}
                  type="button"
                >
                  <Icon style={css.raw({ color: 'text.faint' })} icon={open ? IconChevronDown : IconChevronRight} size={12} />
                  <span class={css({ fontVariantNumeric: 'tabular-nums' })}>#{item.number}</span>
                  {#if item.category}
                    <span class={css({ color: 'text.faint' })}>{item.category}</span>
                  {/if}
                </button>
                <span class={flex({ align: 'center', gap: '10px', marginLeft: 'auto' })}>
                  {#if item.taskId}
                    <a
                      class={css({
                        fontSize: '11px',
                        color: 'text.faint',
                        textDecoration: 'underline',
                        textUnderlineOffset: '[2px]',
                        _hover: { color: 'text.default' },
                      })}
                      href={`/admin/tasks/${item.taskId}?feedback=${item.feedbackId}`}
                    >
                      원문
                    </a>
                  {/if}
                  <span class={css({ fontSize: '11px', color: 'text.faint' })}>{item.evaluator}</span>
                  <AxisMark
                    slots={[
                      { label: '정확', failed: item.failed.correct },
                      { label: '가치', failed: item.failed.needed },
                      { label: '실행', failed: item.failed.useful },
                    ]}
                  />
                </span>
              </div>

              {#if item.note}
                <p class={css({ marginTop: '5px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' })}>{item.note}</p>
              {:else}
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
