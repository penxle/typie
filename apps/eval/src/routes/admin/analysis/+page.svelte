<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { goto } from '$app/navigation';
  import AnalysisRunModal from './AnalysisRunModal.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  // 단계 이름은 그 단계가 하는 일로 쓴다 — '통람', '편성' 같은 말은 뜻이 전달되지 않고
  // '검토'와 '검증'은 서로 구별되지 않는다.
  const STAGE_LABELS: Record<string, string> = {
    survey: '작품 파악',
    review: '짚을 곳 찾기',
    dedupe: '중복 묶기',
    verify: '근거 확인',
    compose: '피드백 다듬기',
    composeReview: '총평 쓰기',
  };

  let openSetId = $state<string | null>(null);
  let running = $state(false);
  let runError = $state<string | null>(null);

  const startRun = async (promptSetId: string, corpusVersion: string, documentIds: string[] | undefined) => {
    running = true;
    runError = null;
    try {
      const response = await fetch('/admin/api/runs', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ kind: 'analysis', promptSetId, corpusVersion, documentIds }),
      });
      if (!response.ok) {
        runError = `실행 시작에 실패했습니다 (${response.status}).`;
        return;
      }
      const { runId } = (await response.json()) as { runId: string };
      await goto(`/admin/runs/${runId}`);
    } finally {
      running = false;
    }
  };

  const cardClass = css({
    backgroundColor: 'surface.default',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '12px',
    padding: '20px',
    boxShadow: 'small',
    marginBottom: '12px',
  });
</script>

<Helmet title="분석 파이프라인" trailing="타이피 평가" />

<div class={css({ maxWidth: '880px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>분석 파이프라인</h1>
    <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>
      작품 파악 → 짚을 곳 찾기 → 중복 묶기 → 근거 확인 → 피드백 다듬기 순으로 도는 재설계 파이프라인입니다. 기존 후보와 별도로 관리됩니다.
    </p>
  </header>

  {#if data.sets.length === 0}
    <p class={css({ fontSize: '14px', color: 'text.faint' })}>등록된 프롬프트 세트가 없습니다.</p>
  {/if}

  {#each data.sets as set (set.id)}
    <section class={cardClass}>
      <div class={flex({ align: 'flex-start', justify: 'space-between', gap: '16px' })}>
        <div class={css({ minWidth: '0' })}>
          <h2 class={css({ fontSize: '16px', fontWeight: 'bold' })}>{set.label}</h2>
          {#if set.note}
            <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.subtle' })}>{set.note}</p>
          {/if}
          <p class={css({ marginTop: '2px', fontSize: '12px', color: 'text.faint', fontFamily: 'mono' })}>{set.id}</p>
        </div>
        <button
          class={css({
            flexShrink: '0',
            paddingX: '14px',
            paddingY: '9px',
            borderRadius: '8px',
            backgroundColor: 'accent.brand.default',
            color: 'text.bright',
            fontSize: '13px',
            fontWeight: 'bold',
            cursor: 'pointer',
            transition: '[background-color 0.15s ease]',
            _hover: { backgroundColor: 'accent.brand.hover' },
          })}
          onclick={() => {
            runError = null;
            openSetId = set.id;
          }}
          type="button"
        >
          이 세트로 실행
        </button>
      </div>

      <div class={css({ marginTop: '14px', overflowX: 'auto' })}>
        <table
          class={css({ width: 'full', fontSize: '13px', borderCollapse: 'collapse', '& td, & th': { paddingY: '6px', textAlign: 'left' } })}
        >
          <thead>
            <tr
              class={css({
                '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
              })}
            >
              <th>단계</th>
              <th>모델</th>
              <th>effort</th>
              <th>프롬프트</th>
            </tr>
          </thead>
          <tbody>
            {#each set.stages as stage (stage.stage)}
              <tr class={css({ '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' } })}>
                <td>{STAGE_LABELS[stage.stage] ?? stage.stage}</td>
                <td class={css({ fontFamily: 'mono', fontSize: '12px' })}>{stage.model}</td>
                <td class={css({ color: 'text.subtle' })}>{stage.effort ?? '—'}</td>
                <td class={css({ color: 'text.subtle', fontVariantNumeric: 'tabular-nums' })}>{stage.length.toLocaleString()}자</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>

    {#if openSetId === set.id}
      <AnalysisRunModal
        corpusVersions={data.corpusVersions}
        error={runError}
        onCancel={() => (openSetId = null)}
        onConfirm={(corpusVersion, documentIds) => startRun(set.id, corpusVersion, documentIds)}
        {running}
        setLabel={set.label}
      />
    {/if}
  {/each}
</div>
