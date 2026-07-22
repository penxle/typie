<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { invalidateAll } from '$app/navigation';
  import VariantStatusBadge from '../VariantStatusBadge.svelte';
  import ApplyPanel from './ApplyPanel.svelte';
  import RollbackDialog from './RollbackDialog.svelte';
  import type { StageKey } from '$lib/domain/admin-types.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const STAGE_LABELS: Record<StageKey, string> = { summarize: '요약', meta: '메타', analyze: '분석' };

  type HistoryRow = PageData['applies'][number];

  let rollbackTarget = $state<HistoryRow | null>(null);
  let rollingBack = $state(false);
  let rollbackError = $state<string | null>(null);
  let rollbackResult = $state<string | null>(null);

  const requestRollback = (row: HistoryRow) => {
    rollbackError = null;
    rollbackResult = null;
    rollbackTarget = row;
  };

  const confirmRollback = async () => {
    if (!rollbackTarget) return;
    rollingBack = true;
    rollbackError = null;
    try {
      const response = await fetch('/admin/api/rollback', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ applyId: rollbackTarget.id }),
      });
      if (!response.ok) {
        rollbackError = `롤백에 실패했습니다 (${response.status}).`;
        return;
      }
      const body = (await response.json()) as { ok: boolean };
      rollbackResult = body.ok ? '롤백이 적용되었습니다.' : '롤백 요청은 기록되었지만 내부 API 적용에 실패했습니다.';
      rollbackTarget = null;
      await invalidateAll();
    } finally {
      rollingBack = false;
    }
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

  // 롤백 비활성 사유: 서버가 계산한 rollbackBlockedReason(실패 이력·단계별 비최신 행) 우선, 그다음 current 로드 실패.
  const rollbackDisabledReason = (row: HistoryRow) =>
    row.rollbackBlockedReason ?? (data.currentPrompts ? null : '현재 프롬프트를 불러오지 못해 롤백 diff를 계산할 수 없습니다.');
</script>

<div class={css({ maxWidth: '960px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>적용</h1>
    <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>
      후보를 골라 현재 프로덕션 프롬프트와 비교하고, 단계별로 적용하거나 이전 값으로 롤백합니다.
    </p>
  </header>

  {#if data.currentError}
    <section
      class={css({
        marginBottom: '16px',
        paddingX: '16px',
        paddingY: '12px',
        borderRadius: '10px',
        backgroundColor: 'accent.warning.subtle',
        fontSize: '13px',
        color: 'accent.warning.default',
      })}
    >
      현재 프로덕션 프롬프트를 불러오지 못했습니다 ({data.currentError}). internal-api 서버가 켜져 있는지 확인하세요 — 대상이 없으면 diff를
      표시할 수 없습니다.
    </section>
  {/if}

  <section class={cardClass}>
    <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '12px' })}>후보 선택</h2>
    {#if data.variantSummaries.length === 0}
      <p class={css({ fontSize: '13px', color: 'text.faint' })}>아직 만들어진 후보가 없습니다.</p>
    {:else}
      <div class={flex({ direction: 'column', gap: '4px' })}>
        {#each data.variantSummaries as variant (variant.id)}
          <a
            class={flex({
              align: 'center',
              gap: '10px',
              paddingX: '10px',
              paddingY: '8px',
              borderRadius: '8px',
              borderWidth: '1px',
              borderColor: data.selectedVariant?.id === variant.id ? 'border.brand' : 'transparent',
              backgroundColor: data.selectedVariant?.id === variant.id ? 'accent.brand.subtle' : 'transparent',
              transition: '[background-color 0.15s ease, border-color 0.15s ease]',
              _hover: data.selectedVariant?.id === variant.id ? {} : { backgroundColor: 'surface.subtle' },
            })}
            href={`?variantId=${variant.id}`}
          >
            <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>{variant.label}</span>
            <VariantStatusBadge status={variant.status} />
          </a>
        {/each}
      </div>
    {/if}
  </section>

  {#if !data.selectedVariant}
    <section class={cardClass}>
      <p class={css({ minHeight: '60px', fontSize: '13px', color: 'text.faint' })}>적용할 후보를 선택하세요.</p>
    </section>
  {:else if data.currentPrompts}
    {#key data.selectedVariant.id}
      <ApplyPanel currentPrompts={data.currentPrompts} variant={data.selectedVariant} />
    {/key}
  {:else}
    <section class={cardClass}>
      <p class={css({ minHeight: '60px', fontSize: '13px', color: 'text.faint', lineHeight: '[1.5]' })}>
        "{data.selectedVariant.label}" 후보를 선택했지만 현재 프로덕션 프롬프트를 불러오지 못해 diff를 계산할 수 없습니다. 위 안내를
        확인하고 internal-api 서버 상태를 점검하세요.
      </p>
    </section>
  {/if}

  <section class={cardClass}>
    <div class={flex({ align: 'center', justify: 'space-between', marginBottom: '4px' })}>
      <h2 class={css({ fontSize: '14px', fontWeight: 'bold' })}>적용 이력</h2>
    </div>
    <p class={css({ marginBottom: '12px', fontSize: '12px', color: 'text.faint' })}>
      모든 적용은 적용 전 값(prev)을 함께 보존하며, 롤백으로 그 값으로 되돌릴 수 있습니다.
    </p>

    {#if data.applies.length === 0}
      <p class={css({ paddingY: '32px', textAlign: 'center', fontSize: '13px', color: 'text.faint' })}>적용 이력이 없습니다.</p>
    {:else}
      <table class={css({ width: 'full', fontSize: '13px', '& td, & th': { paddingX: '12px', paddingY: '8px', textAlign: 'left' } })}>
        <thead>
          <tr
            class={css({
              '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
            })}
          >
            <th>시각</th>
            <th>단계</th>
            <th>후보</th>
            <th>상태</th>
            <th>실행자</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each data.applies as row (row.id)}
            <tr class={css({ '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' } })}>
              <td class={css({ color: 'text.faint' })}>{new Date(row.createdAt).toLocaleString('ko')}</td>
              <td>{STAGE_LABELS[row.stage]}</td>
              <td>{row.variantLabel}</td>
              <td>
                <span class={css({ fontSize: '12px', color: row.status === 'applied' ? 'text.success' : 'text.danger' })}>
                  {row.status === 'applied' ? '적용됨' : '실패'}
                </span>
              </td>
              <td class={css({ color: 'text.faint', fontSize: '12px' })}>{row.appliedBy}</td>
              <td>
                <button
                  class={css({
                    paddingX: '10px',
                    paddingY: '6px',
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    borderRadius: '6px',
                    fontSize: '12px',
                    color: 'text.default',
                    cursor: 'pointer',
                    transition: '[background-color 0.15s ease]',
                    _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
                    ['&:hover:not(:disabled)']: { backgroundColor: 'surface.muted' },
                  })}
                  disabled={rollbackDisabledReason(row) !== null}
                  onclick={() => requestRollback(row)}
                  title={rollbackDisabledReason(row) ?? undefined}
                  type="button"
                >
                  롤백
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    <p class={css({ marginTop: '8px', height: '16px', fontSize: '12px', color: rollbackResult ? 'text.success' : 'text.danger' })}>
      {rollbackError ?? rollbackResult ?? ''}
    </p>
  </section>
</div>

{#if rollbackTarget && data.currentPrompts}
  <RollbackDialog
    current={data.currentPrompts[rollbackTarget.stage]}
    error={rollbackError}
    onCancel={() => (rollbackTarget = null)}
    onConfirm={confirmRollback}
    pending={rollingBack}
    prev={rollbackTarget.prev}
    stageLabel={STAGE_LABELS[rollbackTarget.stage]}
  />
{/if}
