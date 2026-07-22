<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Dialog } from '@typie/ui/notification';
  import { invalidateAll } from '$app/navigation';
  import { diffLines } from '$lib/domain/line-diff.ts';
  import LineDiff from './LineDiff.svelte';
  import type { StageKey, StagePrompt, VariantContent } from '$lib/domain/admin-types.ts';

  type Props = {
    variant: { id: string; label: string; content: VariantContent };
    currentPrompts: Record<StageKey, StagePrompt>;
  };
  const { variant, currentPrompts }: Props = $props();

  const STAGES: StageKey[] = ['summarize', 'meta', 'analyze'];
  const STAGE_LABELS: Record<StageKey, string> = { summarize: '요약', meta: '메타', analyze: '분석' };

  const stageDiffs = STAGES.map((stage) => {
    const current = currentPrompts[stage];
    const candidate = variant.content[stage];
    const systemDiff = diffLines(current.system, candidate.system);
    const toolsDiff = diffLines(JSON.stringify(current.tools, null, 2), JSON.stringify(candidate.tools, null, 2));
    const modelChanged = current.model !== candidate.model;
    const effortChanged = current.effort !== candidate.effort;
    const changed = systemDiff.some((e) => e.type !== 'same') || toolsDiff.some((e) => e.type !== 'same') || modelChanged || effortChanged;
    return { stage, current, candidate, systemDiff, toolsDiff, modelChanged, effortChanged, changed };
  });

  // 이 컴포넌트는 부모가 {#key variant.id}로 감싸 후보가 바뀔 때마다 새로 마운트하므로 마운트 시점 기본 체크값만 캡처하면 된다.
  let selected = $state<Record<StageKey, boolean>>(
    Object.fromEntries(stageDiffs.map((d) => [d.stage, d.changed])) as Record<StageKey, boolean>,
  );

  const selectedCount = $derived(STAGES.filter((s) => selected[s]).length);

  let applying = $state(false);
  let applyResults = $state<{ stage: StageKey; ok: boolean; error?: string }[] | null>(null);

  const requestApply = () => {
    Dialog.confirm({
      title: `${selectedCount}개 단계를 프로덕션에 적용할까요?`,
      message: '선택한 단계의 프롬프트가 즉시 프로덕션에 반영됩니다. 적용 이력이 기록되며 롤백할 수 있습니다.',
      actionLabel: '적용',
      actionHandler: () => runApply(),
    });
  };

  const runApply = async () => {
    applying = true;
    const results: { stage: StageKey; ok: boolean; error?: string }[] = [];

    for (const stage of STAGES) {
      if (!selected[stage]) continue;
      try {
        const response = await fetch('/admin/api/apply', {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ promptVariantId: variant.id, stage }),
        });
        if (!response.ok) {
          results.push({ stage, ok: false, error: `HTTP ${response.status}` });
          continue;
        }
        const body = (await response.json()) as { ok: boolean };
        results.push({ stage, ok: body.ok, error: body.ok ? undefined : '내부 API가 적용을 거부했습니다 (이력에 실패로 기록됨).' });
      } catch (err) {
        results.push({ stage, ok: false, error: String(err).slice(0, 200) });
      }
    }

    applyResults = results;
    applying = false;
    await invalidateAll();
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
</script>

<section class={cardClass}>
  <div class={flex({ align: 'center', justify: 'space-between', marginBottom: '16px' })}>
    <div>
      <h2 class={css({ fontSize: '15px', fontWeight: 'bold' })}>{variant.label} 적용</h2>
      <p class={css({ marginTop: '2px', fontSize: '12px', color: 'text.faint' })}>
        현재 프로덕션 프롬프트 대비 라인 diff입니다. 내용이 다른 단계가 기본으로 체크됩니다.
      </p>
    </div>
    <button
      class={css({
        paddingX: '16px',
        paddingY: '9px',
        borderRadius: '8px',
        backgroundColor: 'accent.brand.default',
        color: 'text.bright',
        fontSize: '13px',
        fontWeight: 'bold',
        cursor: 'pointer',
        flexShrink: '0',
        transition: '[background-color 0.15s ease]',
        _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
        ['&:hover:not(:disabled)']: { backgroundColor: 'accent.brand.hover' },
      })}
      disabled={selectedCount === 0 || applying}
      onclick={requestApply}
      type="button"
    >
      {selectedCount}개 단계 적용
    </button>
  </div>

  <div class={flex({ direction: 'column', gap: '16px' })}>
    {#each stageDiffs as diff (diff.stage)}
      <div
        class={css({
          borderTopWidth: '1px',
          borderColor: 'border.subtle',
          paddingTop: '16px',
          ['&:first-child']: { borderTopWidth: '0', paddingTop: '0' },
        })}
      >
        <label class={flex({ align: 'center', gap: '8px', fontSize: '14px', fontWeight: 'bold', cursor: 'pointer', marginBottom: '8px' })}>
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
            checked={selected[diff.stage]}
            onchange={() => (selected[diff.stage] = !selected[diff.stage])}
            type="checkbox"
          />
          {STAGE_LABELS[diff.stage]}
          {#if !diff.changed}
            <span class={css({ fontSize: '11px', fontWeight: 'normal', color: 'text.faint' })}>변경 없음</span>
          {/if}
        </label>

        {#if diff.modelChanged || diff.effortChanged}
          <p class={css({ marginBottom: '8px', fontSize: '12px', color: 'text.subtle' })}>
            model {diff.current.model || '(미지정)'} → {diff.candidate.model || '(미지정)'} · effort {diff.current.effort ?? '(미지정)'} → {diff
              .candidate.effort ?? '(미지정)'}
          </p>
        {/if}

        <p class={css({ fontSize: '11px', color: 'text.faint', marginBottom: '4px' })}>system</p>
        <LineDiff entries={diff.systemDiff} />
        <p class={css({ marginTop: '10px', fontSize: '11px', color: 'text.faint', marginBottom: '4px' })}>tools</p>
        <LineDiff entries={diff.toolsDiff} />
      </div>
    {/each}
  </div>

  {#if applyResults}
    <div class={css({ marginTop: '16px', padding: '12px', borderRadius: '8px', backgroundColor: 'surface.subtle' })}>
      <p class={css({ fontSize: '12px', fontWeight: 'bold', color: 'text.subtle', marginBottom: '6px' })}>적용 결과</p>
      <div class={flex({ direction: 'column', gap: '4px' })}>
        {#each applyResults as result (result.stage)}
          <p class={css({ fontSize: '13px', color: result.ok ? 'text.success' : 'text.danger' })}>
            {STAGE_LABELS[result.stage]}: {result.ok ? '성공' : `실패${result.error ? ` (${result.error})` : ''}`}
          </p>
        {/each}
      </div>
    </div>
  {/if}
</section>
