<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import IconCheck from '~icons/lucide/check';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const STAGE_LABELS: Record<string, string> = { screening: '스크리닝', confirmation: '확정' };

  const formatLastAt = (iso: string | null) => {
    if (!iso) return '—';
    return new Date(iso).toLocaleString('ko', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  };

  const cardClass = css({
    backgroundColor: 'surface.default',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '12px',
    padding: '20px',
    boxShadow: 'small',
    marginBottom: '16px',
  });

  const chipClass = css({
    paddingX: '8px',
    paddingY: '2px',
    borderRadius: 'full',
    fontSize: '12px',
    fontWeight: 'medium',
    backgroundColor: 'surface.muted',
    color: 'text.subtle',
  });

  const attentionChipClass = css({
    paddingX: '8px',
    paddingY: '2px',
    borderRadius: 'full',
    fontSize: '12px',
    fontWeight: 'medium',
    backgroundColor: 'accent.warning.subtle',
    color: 'accent.warning.default',
  });

  const tableClass = css({
    width: 'full',
    fontSize: '13px',
    fontVariantNumeric: 'tabular-nums',
    '& td, & th': { paddingX: '10px', paddingY: '8px', textAlign: 'left' },
    '& td:not(:first-child), & th:not(:first-child)': { textAlign: 'right' },
  });
</script>

<Helmet title="평가자" trailing="타이피 평가" />

<div class={css({ maxWidth: '1080px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <header class={css({ marginBottom: '24px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>평가자</h1>
    <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>라운드별 평가자 참여 현황과 남은 할당량입니다.</p>
  </header>

  {#if data.summaries.length === 0}
    <section class={cardClass}>
      <p class={css({ fontSize: '14px', color: 'text.subtle', textAlign: 'center', paddingY: '12px' })}>아직 라운드가 없습니다.</p>
    </section>
  {/if}

  {#each data.summaries as summary (summary.roundId)}
    <section class={cardClass}>
      <div class={flex({ align: 'center', gap: '10px' })}>
        <h2 class={css({ fontSize: '15px', fontWeight: 'bold' })}>{summary.roundId}</h2>
        <span class={chipClass}>{STAGE_LABELS[summary.stage] ?? summary.stage}</span>
        <span class={css({ marginLeft: 'auto', fontSize: '13px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
          유효 판정 {summary.effectiveTotal} / {summary.requiredTotal} · 확정 {summary.confirmedTotal}건
        </span>
      </div>
      {#if summary.cap !== null}
        <p class={css({ marginTop: '4px', fontSize: '12px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
          1인당 배정 한도 {summary.cap}건 (예상 평가자 {summary.expected}명 기준) — 한도는 잉여 판정으로 소모된 용량만큼 자동 확대됩니다.
          남은 몫은 한도 기준이며, 라운드 잔여 태스크에 따라 실제 배정 가능 수는 더 적을 수 있습니다.
        </p>
      {/if}

      <div class={css({ marginTop: '12px', overflowX: 'auto' })}>
        <table class={tableClass}>
          <thead>
            <tr
              class={css({
                '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
              })}
            >
              <th>평가자</th>
              <th>확정 판정</th>
              <th>진행</th>
              <th>남은 몫</th>
              <th>상태</th>
              <th>마지막 활동</th>
            </tr>
          </thead>
          <tbody>
            {#each summary.evaluators as evaluator (evaluator.email)}
              <tr class={css({ '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' } })}>
                <td class={css({ fontWeight: 'medium', wordBreak: 'break-all' })}>{evaluator.email}</td>
                <td>{summary.cap === null ? evaluator.confirmed : `${evaluator.confirmed} / ${summary.cap}`}</td>
                <td>
                  {#if summary.cap !== null}
                    <div
                      class={css({
                        display: 'inline-block',
                        width: '96px',
                        height: '6px',
                        borderRadius: 'full',
                        backgroundColor: 'surface.muted',
                        overflow: 'hidden',
                        verticalAlign: 'middle',
                      })}
                    >
                      <div
                        style:width={`${Math.min(100, Math.round((evaluator.confirmed / summary.cap) * 100))}%`}
                        class={css({ height: 'full', backgroundColor: 'accent.brand.default' })}
                      ></div>
                    </div>
                  {:else}
                    —
                  {/if}
                </td>
                <td>
                  {#if evaluator.quotaLeft === null}
                    —
                  {:else if evaluator.quotaLeft === 0}
                    <span class={flex({ align: 'center', justify: 'flex-end', gap: '3px', color: 'text.success' })}>
                      <Icon icon={IconCheck} size={12} />
                      완료
                    </span>
                  {:else}
                    {evaluator.quotaLeft}건
                  {/if}
                </td>
                <td>
                  {#if evaluator.hasDraft}
                    <span class={chipClass}>작성 중</span>
                  {:else if evaluator.confirmed === 0}
                    <span class={attentionChipClass}>미참여</span>
                  {:else if evaluator.confirmed > evaluator.effective}
                    <span class={chipClass}>잉여 {evaluator.confirmed - evaluator.effective}건</span>
                  {:else}
                    —
                  {/if}
                </td>
                <td>{formatLastAt(evaluator.lastAt)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  {/each}
</div>
