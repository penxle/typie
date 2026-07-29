<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { deserialize, enhance } from '$app/forms';
  import { invalidateAll } from '$app/navigation';
  import {
    checkboxClass,
    emptyClass,
    formInputClass,
    formLabelClass,
    formNoticeClass,
    formSubmitClass,
    pageClass,
    pageDescClass,
    pageTitleClass,
    panelClass,
    quietButtonClass,
    rowLinkClass,
    tableClass,
    tableHeadClass,
    tableRowClass,
  } from '$lib/styles.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData; form: { message?: string; roundId?: string } | null };
  const { data, form }: Props = $props();

  const selected = new SvelteSet<string>();
  let evaluationId = $state(untrack(() => data.evaluations[0]?.id ?? ''));
  let creating = $state(false);

  const generationOf = $derived(data.evaluations.find((e) => e.id === evaluationId)?.generationId ?? null);
  const eligible = $derived(generationOf ? data.candidates.filter((c) => c.generationId === generationOf) : data.candidates);

  // 목록이 짧거나 비면 실행이 실패한 것처럼 읽힌다 — 무엇을 왜 뺐는지 밝힌다.
  const excludedNote = $derived.by(() => {
    const parts = [
      data.excluded.used > 0 ? `이미 라운드에 쓰인 문서 ${data.excluded.used}건` : null,
      data.excluded.intake > 0 ? `반입 문서 ${data.excluded.intake}건` : null,
    ].filter((p) => p !== null);
    return parts.length > 0 ? `${parts.join(', ')}은 후보에서 제외했습니다.` : null;
  });

  const requestInvalidate = (roundId: string) => {
    Dialog.confirm({
      title: '라운드를 무효화할까요?',
      message: '이 라운드의 태스크가 모두 삭제됩니다. 아직 판정이 없는 라운드만 무효화할 수 있습니다.',
      action: 'danger',
      actionLabel: '무효화',
      cancelLabel: '되돌아가기',
      actionHandler: async () => {
        const body = new FormData();
        body.set('roundId', roundId);
        const response = await fetch('?/invalidate', { method: 'POST', body });
        const result = deserialize(await response.text());
        if (result.type === 'failure') {
          Toast.error((result.data as { message?: string } | undefined)?.message ?? '무효화에 실패했습니다.');
          return false;
        }
        if (result.type === 'error') {
          Toast.error(result.error instanceof Error ? result.error.message : '무효화에 실패했습니다.');
          return false;
        }
        await invalidateAll();
      },
    });
  };

  const toggle = (id: string, on: boolean) => {
    if (on) selected.add(id);
    else selected.delete(id);
  };

  // 열고 닫는 것은 평가자 전원에게 즉시 보이는 스위치다 — 실수로 누른 클릭 한 번이 라운드를
  // 여닫지 않게 확인을 끼운다.
  const requestToggle = (round: { id: string; label: string; active: boolean }) => {
    Dialog.confirm({
      title: round.active ? '라운드를 닫을까요?' : '라운드를 열까요?',
      message: round.active
        ? `‘${round.label}’이 비활성화됩니다. 평가자는 새로 배정받을 수 없고, 작성 중인 평가도 저장·제출이 막힙니다.`
        : `‘${round.label}’이 활성화됩니다. 평가자 홈에 바로 노출되고 배정이 시작됩니다.`,
      actionLabel: round.active ? '닫기' : '열기',
      actionHandler: async () => {
        const body = new FormData();
        body.set('id', round.id);
        body.set('active', String(!round.active));
        const response = await fetch('?/toggle', { method: 'POST', body });
        const result = deserialize(await response.text());
        if (result.type === 'failure' || result.type === 'error') {
          Toast.error('상태를 바꾸지 못했습니다. 잠시 후 다시 시도해주세요.');
          return;
        }
        await invalidateAll();
      },
    });
  };
</script>

<Helmet title="라운드" trailing="타이피 평가" />

<div class={pageClass}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={pageTitleClass}>라운드</h1>
    <p class={pageDescClass}>평가 라운드를 만들고 진행 현황을 확인합니다.</p>
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

    <form
      action="?/create"
      method="post"
      use:enhance={() => {
        creating = true;
        return async ({ result, update }) => {
          // 만들어진 실행은 후보에서 빠지는데 선택 상태는 남는다 — 버튼이 없는 것을 세게 된다.
          // 실패했을 때는 다시 누를 수 있도록 선택을 지우지 않는다.
          if (result.type === 'success') selected.clear();
          await update();
          creating = false;
        };
      }}
    >
      <div
        style:grid-template-columns={`repeat(${Math.max(1, data.evaluations.length)}, 1fr)`}
        class={css({ display: 'grid', gap: '6px', marginBottom: '16px' })}
      >
        {#each data.evaluations as evaluation (evaluation.id)}
          <button
            class={css({
              paddingY: '8px',
              borderRadius: '8px',
              borderWidth: '1px',
              borderColor: evaluationId === evaluation.id ? 'border.strong' : 'border.default',
              backgroundColor: evaluationId === evaluation.id ? 'surface.dark' : 'surface.default',
              color: evaluationId === evaluation.id ? 'text.bright' : 'text.default',
              fontSize: '14px',
              fontWeight: evaluationId === evaluation.id ? 'bold' : 'normal',
              cursor: 'pointer',
              transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
            })}
            onclick={() => (evaluationId = evaluation.id)}
            type="button"
          >
            {evaluation.label}
          </button>
        {/each}
      </div>
      <input name="evaluationId" type="hidden" value={evaluationId} />

      <div class={css({ marginBottom: '16px' })}>
        <label class={formLabelClass} for="round-label">라벨</label>
        <input id="round-label" name="label" class={formInputClass} placeholder="예: 3라운드" required type="text" />
      </div>

      <div class={css({ marginBottom: '4px' })}>
        <span class={formLabelClass}>대상 실행</span>
        {#if eligible.length === 0}
          <p class={css({ fontSize: '13px', color: 'text.faint' })}>
            고를 수 있는 실행이 없습니다. 아직 어느 라운드에도 쓰이지 않은 표집 문서의 완료 실행만 고를 수 있습니다.
          </p>
        {:else}
          <div class={flex({ direction: 'column', gap: '6px', maxHeight: '240px', overflowY: 'auto' })}>
            {#each eligible as candidate (candidate.id)}
              <label class={flex({ align: 'center', gap: '8px', fontSize: '14px', cursor: 'pointer' })}>
                <input
                  name="runIds"
                  class={checkboxClass}
                  checked={selected.has(candidate.id)}
                  onchange={(e) => toggle(candidate.id, e.currentTarget.checked)}
                  type="checkbox"
                  value={candidate.id}
                />
                {candidate.refId ?? candidate.id}
                {#if candidate.promptSetLabel}
                  <span class={css({ fontSize: '13px', color: 'text.faint' })}>{candidate.promptSetLabel}</span>
                {/if}
              </label>
            {/each}
          </div>
        {/if}
        {#if excludedNote}
          <p class={css({ marginTop: '6px', fontSize: '12px', color: 'text.faint' })}>{excludedNote}</p>
        {/if}
      </div>

      <button
        class={[formSubmitClass, css({ marginTop: '16px' })]}
        disabled={creating || !evaluationId || selected.size === 0}
        type="submit"
      >
        {creating ? '생성 중…' : `라운드 생성 (${selected.size}건)`}
      </button>
      <p class={[formNoticeClass, css({ color: form?.roundId ? 'text.success' : 'text.danger' })]}>
        {form?.message ?? (form?.roundId ? '라운드가 생성되었습니다.' : '')}
      </p>
    </form>
  </section>

  <section class={panelClass}>
    {#if data.rounds.length === 0}
      <p class={emptyClass}>아직 만들어진 라운드가 없습니다.</p>
    {:else}
      <table class={tableClass}>
        <thead>
          <tr class={tableHeadClass}>
            <th>라운드</th>
            <th>평가 방식</th>
            <th>태스크</th>
            <th>판정</th>
            <th>생성 시각</th>
            <th>상태</th>
            <th></th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each data.rounds as round (round.id)}
            <tr class={tableRowClass}>
              <td>{round.label}</td>
              <td class={css({ color: 'text.faint' })}>{round.evaluationLabel}</td>
              <td>{round.total.toLocaleString()}</td>
              <td>{round.done.toLocaleString()}</td>
              <td class={css({ color: 'text.faint' })}>{new Date(round.createdAt).toLocaleString('ko')}</td>
              <td>
                <button
                  class={[quietButtonClass, css({ color: round.active ? 'text.success' : 'text.faint' })]}
                  onclick={() => requestToggle(round)}
                  type="button"
                >
                  {round.active ? '활성 — 닫기' : '비활성 — 열기'}
                </button>
              </td>
              <td><a class={rowLinkClass} href="/admin/rounds/{round.id}">보기 →</a></td>
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
                  disabled={round.total === 0 || round.done > 0}
                  onclick={() => requestInvalidate(round.id)}
                  title={round.done > 0 ? '판정이 존재해 무효화할 수 없습니다.' : undefined}
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
