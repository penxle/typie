<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { untrack } from 'svelte';
  import { usePolling } from '../lib/poll.svelte.ts';
  import { formatProgressSummary, KIND_LABELS } from './progress.ts';
  import RunStatusBadge from './RunStatusBadge.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  type RunListItem = PageData['runs'][number];

  let runs = $state<RunListItem[]>(untrack(() => data.runs));

  const refresh = async () => {
    const response = await fetch('/admin/api/runs');
    if (!response.ok) return;
    const { runs: fresh } = (await response.json()) as { runs: Omit<RunListItem, 'variantLabel'>[] };

    const labelByVariantId = new Map(runs.filter((r) => r.variantId).map((r) => [r.variantId as string, r.variantLabel]));
    runs = fresh.map((run) => ({ ...run, variantLabel: run.variantId ? (labelByVariantId.get(run.variantId) ?? run.variantId) : null }));
  };

  usePolling(refresh, 3000);
</script>

<div class={css({ maxWidth: '1080px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>실행</h1>
    <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>파이프라인·샘플링 실행 목록입니다. 3초마다 갱신됩니다.</p>
  </header>

  <section
    class={css({
      backgroundColor: 'surface.default',
      borderWidth: '1px',
      borderColor: 'border.default',
      borderRadius: '12px',
      boxShadow: 'small',
      overflow: 'hidden',
    })}
  >
    {#if runs.length === 0}
      <p class={css({ paddingY: '48px', textAlign: 'center', fontSize: '14px', color: 'text.faint' })}>아직 실행된 작업이 없습니다.</p>
    {:else}
      <table class={css({ width: 'full', fontSize: '13px', '& td, & th': { paddingX: '16px', paddingY: '10px', textAlign: 'left' } })}>
        <thead>
          <tr
            class={css({
              '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
            })}
          >
            <th>종류</th>
            <th>후보</th>
            <th>코퍼스</th>
            <th>상태</th>
            <th>진행</th>
            <th>생성 시각</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each runs as run (run.id)}
            <tr class={css({ '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' } })}>
              <td>{KIND_LABELS[run.kind]}</td>
              <td>{run.variantLabel ?? '—'}</td>
              <td>{run.corpusVersion}</td>
              <td><RunStatusBadge status={run.status} /></td>
              <td>{formatProgressSummary(run)}</td>
              <td class={css({ color: 'text.faint' })}>{new Date(run.createdAt).toLocaleString('ko')}</td>
              <td>
                <a
                  class={css({ fontSize: '12px', color: 'text.subtle', _hover: { color: 'text.default' } })}
                  href={`/admin/runs/${run.id}`}
                >
                  보기 →
                </a>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
</div>
