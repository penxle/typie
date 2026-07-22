<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const percent = (value: number) => (Number.isNaN(value) ? '—' : `${(value * 100).toFixed(1)}%`);
  const fixed = (value: number) => (Number.isNaN(value) ? '—' : value.toFixed(3));
</script>

<main class={css({ minHeight: '[100dvh]', backgroundColor: 'surface.subtle' })}>
  <div class={css({ maxWidth: '960px', marginX: 'auto', paddingY: '48px', paddingX: '20px' })}>
    <header class={flex({ align: 'baseline', gap: '12px' })}>
      <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>대시보드</h1>
      <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/">평가 큐 →</a>
    </header>

    {#each data.summaries as summary (summary.roundId)}
      <section
        class={css({
          marginTop: '24px',
          backgroundColor: 'surface.default',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '12px',
          padding: '24px',
          boxShadow: 'small',
        })}
      >
        <div class={flex({ align: 'center', gap: '10px' })}>
          <h2 class={css({ fontSize: '16px', fontWeight: 'bold' })}>{summary.roundId}</h2>
          <span
            class={css({
              paddingX: '8px',
              paddingY: '2px',
              borderRadius: 'full',
              backgroundColor: 'surface.muted',
              fontSize: '12px',
              color: 'text.subtle',
            })}
          >
            {summary.stage === 'screening' ? '스크리닝' : '확정'}
          </span>
        </div>

        <div class={grid({ columns: 4, gap: '10px', marginTop: '16px' })}>
          {#each [{ label: '판정 진행', value: `${summary.confirmedCount} / ${summary.taskCount}` }, { label: '평가자 일치도 (kappa)', value: fixed(summary.kappa) }, { label: 'sanity 통과', value: percent(summary.sanityPass) }, { label: '카테고리 준수', value: percent(summary.categoryCompliance) }] as stat (stat.label)}
            <div class={css({ backgroundColor: 'surface.subtle', borderRadius: '10px', padding: '12px' })}>
              <p class={css({ fontSize: '12px', color: 'text.faint' })}>{stat.label}</p>
              <p class={css({ marginTop: '2px', fontSize: '18px', fontWeight: 'bold' })}>{stat.value}</p>
            </div>
          {/each}
        </div>

        <div class={css({ marginTop: '16px', overflowX: 'auto' })}>
          <table class={css({ width: 'full', fontSize: '13px', '& td, & th': { paddingX: '10px', paddingY: '8px', textAlign: 'left' } })}>
            <thead>
              <tr
                class={css({
                  '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
                })}
              >
                <th>variant</th>
                <th>승</th>
                <th>무</th>
                <th>패</th>
                <th>오탐율</th>
                <th>앵커 매칭률</th>
                <th>0건</th>
                <th>10건 초과</th>
                <th>토큰</th>
              </tr>
            </thead>
            <tbody>
              {#each summary.variants as variant (variant.label)}
                <tr
                  class={css({
                    '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' },
                    backgroundColor: variant.isBaseline ? 'surface.subtle' : 'surface.default',
                  })}
                >
                  <td class={css({ fontWeight: 'bold' })}>
                    {variant.label}
                    {#if variant.isBaseline}
                      <span class={css({ marginLeft: '4px', fontSize: '11px', fontWeight: 'normal', color: 'text.faint' })}>기준선</span>
                    {/if}
                  </td>
                  <td>{variant.isBaseline ? '—' : variant.win}</td>
                  <td>{variant.isBaseline ? '—' : variant.tie}</td>
                  <td>{variant.isBaseline ? '—' : variant.loss}</td>
                  <td>{percent(variant.falsePositive)}</td>
                  <td>{percent(variant.anchorMatch)}</td>
                  <td>{variant.zeroCount}</td>
                  <td>{variant.over10Count}</td>
                  <td>{variant.tokens.toLocaleString()}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {/each}
  </div>
</main>
