<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import {
    emptyClass,
    pageClass,
    pageDescClass,
    pageTitleClass,
    panelClass,
    rowLinkClass,
    tableClass,
    tableHeadClass,
    tableRowClass,
  } from '$lib/styles.ts';
  import CostCell from '../lib/CostCell.svelte';
  import { usePolling } from '../lib/poll.svelte.ts';
  import RunStatusBadge from './RunStatusBadge.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  // 문서 화면에서 걸고 넘어온 직후의 알림. 방금 만든 것이 목록 맨 위에 있다.
  const spawned = $derived(Number(page.url.searchParams.get('spawned') ?? 0));
  const failed = $derived(Number(page.url.searchParams.get('failed') ?? 0));

  // 실행 상태는 로드가 인스턴스에 물어 갱신한다 — 목록을 다시 태우는 것이 곧 폴링이다.
  usePolling(() => invalidateAll(), 3000, { enabled: () => data.runs.some((r) => r.status === 'running' || r.status === 'pending') });
</script>

<Helmet title="실행" trailing="타이피 평가" />

<div class={pageClass}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={pageTitleClass}>실행</h1>
    <p class={pageDescClass}>원고 한 편에 프롬프트 묶음 하나를 돌린 결과입니다. 3초마다 갱신됩니다.</p>
  </header>

  {#if spawned > 0 || failed > 0}
    <p
      class={css({
        marginBottom: '16px',
        paddingX: '14px',
        paddingY: '12px',
        borderRadius: '10px',
        fontSize: '13px',
        backgroundColor: failed > 0 ? 'accent.danger.subtle' : 'accent.success.subtle',
        color: failed > 0 ? 'text.danger' : 'text.success',
      })}
    >
      실행 {spawned}건을 시작했습니다{failed > 0 ? `, ${failed}건은 걸지 못했습니다` : ''}.
    </p>
  {/if}

  <section class={panelClass}>
    {#if data.runs.length === 0}
      <p class={emptyClass}>아직 실행된 작업이 없습니다.</p>
    {:else}
      <table class={tableClass}>
        <thead>
          <tr class={tableHeadClass}>
            <th>문서</th>
            <th>묶음</th>
            <th>상태</th>
            <th>진행</th>
            <th>비용</th>
            <th>생성 시각</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each data.runs as run (run.id)}
            <tr class={tableRowClass}>
              <td>{run.refId ?? run.id}</td>
              <td>{run.promptSetLabel ?? '—'}</td>
              <td><RunStatusBadge status={run.status} /></td>
              <td>{run.phaseLabel ?? '—'}</td>
              <td><CostCell cost={run.cost} tokens={run.tokens} total={run.stageTotal} /></td>
              <td class={css({ color: 'text.faint' })}>{new Date(run.createdAt).toLocaleString('ko')}</td>
              <td><a class={rowLinkClass} href="/admin/runs/{run.id}">보기 →</a></td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
</div>
