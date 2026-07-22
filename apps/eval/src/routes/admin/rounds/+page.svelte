<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { nanoid } from 'nanoid';
  import { untrack } from 'svelte';
  import { deserialize } from '$app/forms';
  import { invalidateAll } from '$app/navigation';
  import ConfirmInvalidateDialog from './ConfirmInvalidateDialog.svelte';
  import type { RoundStage } from '$lib/domain/types.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const STAGE_LABELS: Record<RoundStage, string> = { screening: '스크리닝', confirmation: '확정' };

  let stage = $state<RoundStage>('screening');
  // 이 페이지는 목록·폼이 한 화면에 공존하는 단일 페이지라 {#key}로 다시 마운트되지 않는다 — 코퍼스 버전 select의
  // 초깃값은 최초 로드 시점만 참조하고, 이후 invalidateAll()로 data가 갱신되어도 사용자가 고른 값을 유지한다.
  let corpusVersion = $state(untrack(() => data.corpusVersions[0] ?? ''));
  let selectedLabels = $state<string[]>([]);
  let baselineLabel = $state('');
  let v0Label = $state('');
  let candidateLabel = $state('');
  let expectedEvaluators = $state('');

  // roundId는 제출마다 새로 만들지 않고 폼이 준비된 시점에 한 번만 발급한다 — 네트워크 오류 등으로 사용자가
  // 같은 입력으로 재시도해도 동일 roundId가 재사용되어 admin/api의 멱등 처리(이미 존재하면 created:false)가
  // 중복 라운드 생성을 막아준다. 성공적으로 라운드가 만들어진 뒤에만 다음 제출을 위해 새 id를 발급한다.
  let roundId = $state(nanoid());

  const availableLabels = $derived(data.labelsByCorpusVersion[corpusVersion] ?? []);

  // 코퍼스 버전이 바뀌면 더 이상 유효하지 않은 선택을 정리한다. availableLabels만 추적하고 정리 대상
  // 상태는 untrack으로 읽는다 — 추적 상태를 같은 effect에서 읽고 쓰면 무한 재실행된다.
  $effect(() => {
    const labels = availableLabels;
    untrack(() => {
      if (selectedLabels.some((label) => !labels.includes(label))) {
        selectedLabels = selectedLabels.filter((label) => labels.includes(label));
      }
      if (v0Label && !labels.includes(v0Label)) v0Label = '';
      if (candidateLabel && !labels.includes(candidateLabel)) candidateLabel = '';
      if (!selectedLabels.includes(baselineLabel)) {
        baselineLabel = selectedLabels.includes('현행') ? '현행' : (selectedLabels[0] ?? '');
      }
    });
  });

  const toggleLabel = (label: string) => {
    selectedLabels = selectedLabels.includes(label) ? selectedLabels.filter((l) => l !== label) : [...selectedLabels, label];
    if (!selectedLabels.includes(baselineLabel)) {
      baselineLabel = selectedLabels.includes('현행') ? '현행' : (selectedLabels[0] ?? '');
    }
  };

  let creating = $state(false);
  let createError = $state<string | null>(null);
  let createResult = $state<string | null>(null);

  const submitRound = async () => {
    createError = null;
    createResult = null;

    if (!corpusVersion) {
      createError = '코퍼스 버전을 선택하세요.';
      return;
    }
    if (stage === 'screening' && selectedLabels.length < 2) {
      createError = '대상 후보를 2개 이상 선택하세요.';
      return;
    }
    if (stage === 'screening' && !selectedLabels.includes(baselineLabel)) {
      createError = '기준선은 선택된 후보 중 하나여야 합니다.';
      return;
    }
    if (stage === 'confirmation' && (!v0Label || !candidateLabel)) {
      createError = 'v0과 후보 라벨을 모두 선택하세요.';
      return;
    }
    if (stage === 'confirmation' && v0Label === candidateLabel) {
      createError = 'v0과 후보는 서로 달라야 합니다.';
      return;
    }

    const parsedEvaluators = Math.floor(Number(expectedEvaluators));
    const payload =
      stage === 'screening'
        ? {
            roundId,
            stage: 'screening' as const,
            corpusVersion,
            variantLabels: selectedLabels,
            baselineLabel,
            ...(parsedEvaluators >= 1 && { expectedEvaluators: parsedEvaluators }),
          }
        : { roundId, stage: 'confirmation' as const, corpusVersion, v0Label, candidateLabel };

    creating = true;
    try {
      const response = await fetch('/admin/api/corpus/rounds', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (!response.ok) {
        const body = await response.text();
        createError = `라운드 생성에 실패했습니다 (${response.status}: ${body.slice(0, 200)}).`;
        return;
      }

      const body = (await response.json()) as { created: boolean; taskCount: number };
      createResult = body.created ? `${body.taskCount}개 태스크가 생성되었습니다.` : '이미 존재하는 라운드입니다.';
      selectedLabels = [];
      v0Label = '';
      candidateLabel = '';
      roundId = nanoid();
      await invalidateAll();
    } finally {
      creating = false;
    }
  };

  let pendingInvalidateId = $state<string | null>(null);
  let invalidating = $state(false);
  let invalidateError = $state<string | null>(null);

  const requestInvalidate = (roundId: string) => {
    invalidateError = null;
    pendingInvalidateId = roundId;
  };

  const confirmInvalidate = async () => {
    if (!pendingInvalidateId) return;
    invalidating = true;
    invalidateError = null;
    try {
      const formData = new FormData();
      formData.set('roundId', pendingInvalidateId);
      const response = await fetch('?/invalidate', { method: 'POST', body: formData });
      const result = deserialize(await response.text());

      if (result.type === 'failure') {
        invalidateError = (result.data as { error?: string } | undefined)?.error ?? '무효화에 실패했습니다.';
        return;
      }
      if (result.type === 'error') {
        invalidateError = result.error instanceof Error ? result.error.message : '무효화에 실패했습니다.';
        return;
      }

      pendingInvalidateId = null;
      await invalidateAll();
    } finally {
      invalidating = false;
    }
  };

  const formatConfig = (round: PageData['rounds'][number]) => {
    if (round.stage !== 'screening') return '—';
    const config = round.config as { overlapRatio?: number; sanityRatio?: number } | null;
    if (!config) return '—';
    return `중복 ${((config.overlapRatio ?? 0) * 100).toFixed(0)}% · 샌티 ${((config.sanityRatio ?? 0) * 100).toFixed(0)}%`;
  };

  const inputClass = css({
    width: 'full',
    paddingX: '10px',
    paddingY: '8px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    fontSize: '14px',
    backgroundColor: 'surface.default',
    cursor: 'pointer',
    transition: '[border-color 0.15s ease]',
    _hover: { borderColor: 'border.strong' },
  });

  const labelClass = css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' });
</script>

<div class={css({ maxWidth: '960px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>라운드</h1>
    <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>평가 라운드를 만들고 진행 현황을 확인합니다.</p>
  </header>

  <section
    class={css({
      marginBottom: '24px',
      backgroundColor: 'surface.default',
      borderWidth: '1px',
      borderColor: 'border.default',
      borderRadius: '12px',
      padding: '20px',
      boxShadow: 'small',
    })}
  >
    <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '12px' })}>새 라운드</h2>

    <div class={grid({ columns: 2, gap: '6px', marginBottom: '16px' })}>
      {#each Object.entries(STAGE_LABELS) as [value, label] (value)}
        <button
          class={css({
            paddingY: '8px',
            borderRadius: '8px',
            borderWidth: '1px',
            borderColor: stage === value ? 'border.strong' : 'border.default',
            backgroundColor: stage === value ? 'surface.dark' : 'surface.default',
            color: stage === value ? 'text.bright' : 'text.default',
            fontSize: '14px',
            fontWeight: stage === value ? 'bold' : 'normal',
            cursor: 'pointer',
            transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
          })}
          onclick={() => (stage = value as RoundStage)}
          type="button"
        >
          {label}
        </button>
      {/each}
    </div>

    <div class={css({ marginBottom: '16px' })}>
      <label class={labelClass} for="round-corpus-version">코퍼스 버전</label>
      {#if data.corpusVersions.length === 0}
        <p class={css({ fontSize: '13px', color: 'text.faint' })}>적재된 코퍼스가 없습니다.</p>
      {:else}
        <select id="round-corpus-version" class={inputClass} bind:value={corpusVersion}>
          {#each data.corpusVersions as version (version)}
            <option value={version}>{version}</option>
          {/each}
        </select>
      {/if}
    </div>

    {#if stage === 'screening'}
      <div class={css({ marginBottom: '16px' })}>
        <label class={labelClass} for="round-expected-evaluators">예상 평가자 수 (선택)</label>
        <input
          id="round-expected-evaluators"
          class={inputClass}
          min="1"
          placeholder="설정하면 평가자당 균등 몫 + 1건까지만 새 태스크가 배정됩니다"
          type="number"
          bind:value={expectedEvaluators}
        />
      </div>
    {/if}

    {#if availableLabels.length === 0}
      <p class={css({ fontSize: '13px', color: 'text.faint', marginBottom: '4px' })}>
        이 코퍼스 버전에 실행 완료된 후보가 없습니다. 먼저 후보를 실행하세요.
      </p>
    {:else if stage === 'screening'}
      <div class={css({ marginBottom: '4px' })}>
        <span class={labelClass}>대상 후보 (2개 이상)</span>
        <div class={flex({ direction: 'column', gap: '6px' })}>
          {#each availableLabels as label (label)}
            <label class={flex({ align: 'center', gap: '8px', fontSize: '14px', cursor: 'pointer' })}>
              <input
                class={css({
                  appearance: 'none',
                  width: '16px',
                  height: '16px',
                  borderWidth: '1px',
                  borderColor: 'border.strong',
                  borderRadius: '4px',
                  backgroundColor: 'surface.default',
                  cursor: 'pointer',
                  flexShrink: '0',
                  transition: '[background-color 0.15s ease, border-color 0.15s ease]',
                  _checked: { backgroundColor: 'accent.brand.default', borderColor: 'border.brand' },
                })}
                checked={selectedLabels.includes(label)}
                onchange={() => toggleLabel(label)}
                type="checkbox"
              />
              {label}
            </label>
          {/each}
        </div>
        {#if selectedLabels.length >= 2}
          <div class={css({ marginTop: '12px' })}>
            <label class={labelClass} for="round-baseline-label">기준선 (비교 기준이 되는 변형)</label>
            <select id="round-baseline-label" class={inputClass} bind:value={baselineLabel}>
              {#each selectedLabels as label (label)}
                <option value={label}>{label}</option>
              {/each}
            </select>
          </div>
        {/if}
      </div>
    {:else}
      <div class={grid({ columns: 2, gap: '16px', marginBottom: '4px' })}>
        <div>
          <label class={labelClass} for="round-v0-label">v0 라벨</label>
          <select id="round-v0-label" class={inputClass} bind:value={v0Label}>
            <option value="">선택하세요</option>
            {#each availableLabels as label (label)}
              <option value={label}>{label}</option>
            {/each}
          </select>
        </div>
        <div>
          <label class={labelClass} for="round-candidate-label">후보 라벨</label>
          <select id="round-candidate-label" class={inputClass} bind:value={candidateLabel}>
            <option value="">선택하세요</option>
            {#each availableLabels as label (label)}
              <option value={label}>{label}</option>
            {/each}
          </select>
        </div>
      </div>
    {/if}

    <button
      class={css({
        marginTop: '16px',
        paddingX: '16px',
        paddingY: '10px',
        borderRadius: '8px',
        backgroundColor: 'accent.brand.default',
        color: 'text.bright',
        fontSize: '13px',
        fontWeight: 'bold',
        cursor: 'pointer',
        transition: '[background-color 0.15s ease]',
        _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
        ['&:hover:not(:disabled)']: { backgroundColor: 'accent.brand.hover' },
      })}
      disabled={creating || data.corpusVersions.length === 0}
      onclick={submitRound}
      type="button"
    >
      {creating ? '생성 중…' : '라운드 생성'}
    </button>
    <p class={css({ marginTop: '8px', height: '16px', fontSize: '12px', color: createResult ? 'text.success' : 'text.danger' })}>
      {createError ?? createResult ?? ''}
    </p>
  </section>

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
    {#if data.rounds.length === 0}
      <p class={css({ paddingY: '48px', textAlign: 'center', fontSize: '14px', color: 'text.faint' })}>아직 만들어진 라운드가 없습니다.</p>
    {:else}
      <table class={css({ width: 'full', fontSize: '13px', '& td, & th': { paddingX: '16px', paddingY: '10px', textAlign: 'left' } })}>
        <thead>
          <tr
            class={css({
              '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
            })}
          >
            <th>스테이지</th>
            <th>설정</th>
            <th>태스크</th>
            <th>판정</th>
            <th>생성 시각</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each data.rounds as round (round.id)}
            <tr class={css({ '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' } })}>
              <td>{STAGE_LABELS[round.stage]}</td>
              <td class={css({ color: 'text.faint' })}>{formatConfig(round)}</td>
              <td>{round.taskCount.toLocaleString()}</td>
              <td>{round.judgmentCount.toLocaleString()}</td>
              <td class={css({ color: 'text.faint' })}>{new Date(round.createdAt).toLocaleString('ko')}</td>
              <td>
                <button
                  class={css({
                    paddingX: '10px',
                    paddingY: '6px',
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    borderRadius: '6px',
                    fontSize: '12px',
                    color: 'text.danger',
                    cursor: 'pointer',
                    transition: '[background-color 0.15s ease]',
                    _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
                    ['&:hover:not(:disabled)']: { backgroundColor: 'accent.danger.subtle' },
                  })}
                  disabled={round.taskCount === 0 || round.judgmentCount > 0}
                  onclick={() => requestInvalidate(round.id)}
                  title={round.judgmentCount > 0 ? '판정이 존재해 무효화할 수 없습니다.' : undefined}
                  type="button"
                >
                  무효화
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
</div>

{#if pendingInvalidateId}
  <ConfirmInvalidateDialog
    error={invalidateError}
    onCancel={() => (pendingInvalidateId = null)}
    onConfirm={confirmInvalidate}
    pending={invalidating}
  />
{/if}
