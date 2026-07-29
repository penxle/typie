<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Switch } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { deserialize } from '$app/forms';
  import { invalidateAll } from '$app/navigation';
  import {
    adminChipClass,
    attentionChipClass,
    chipClass,
    numericTableClass,
    pageClass,
    pageDescClass,
    pageTitleClass,
    sectionCardClass,
    tableHeadClass,
    tableRowClass,
  } from '$lib/styles.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  let pending = $state<string | null>(null);

  const setParticipation = async (email: string, evaluating: boolean) => {
    pending = email;
    try {
      const formData = new FormData();
      formData.set('email', email);
      formData.set('evaluating', String(evaluating));
      const response = await fetch('?/participation', { method: 'POST', body: formData });
      const result = deserialize(await response.text());

      if (result.type === 'failure') {
        Toast.error((result.data as { error?: string } | undefined)?.error ?? '변경에 실패했습니다.');
        return;
      }
      if (result.type === 'error') {
        Toast.error(result.error instanceof Error ? result.error.message : '변경에 실패했습니다.');
        return;
      }

      await invalidateAll();
    } finally {
      pending = null;
    }
  };

  const formatLastAt = (iso: string | null) => {
    if (!iso) return '—';
    return new Date(iso).toLocaleString('ko', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  };
</script>

<Helmet title="평가자" trailing="타이피 평가" />

<div class={pageClass}>
  <header class={css({ marginBottom: '24px' })}>
    <h1 class={pageTitleClass}>평가자</h1>
    <p class={pageDescClass}>라운드별 평가자 참여 현황입니다.</p>
  </header>

  <section class={sectionCardClass}>
    <div class={flex({ align: 'center', gap: '10px' })}>
      <h2 class={css({ fontSize: '15px', fontWeight: 'bold' })}>참여 명단</h2>
      <span class={css({ marginLeft: 'auto', fontSize: '13px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
        참여 {data.roster.filter((r) => r.evaluating).length}명 / 동의 {data.roster.length}명
      </span>
    </div>
    <p class={css({ marginTop: '4px', fontSize: '12px', color: 'text.faint' })}>
      동의만으로는 평가가 시작되지 않습니다 — 여기서 켜야 태스크가 배정됩니다. 어드민 권한과는 무관한 축이라 어드민도 켜면 평가자가 됩니다.
    </p>

    <div class={css({ marginTop: '12px', overflowX: 'auto' })}>
      <table class={numericTableClass}>
        <thead>
          <tr class={tableHeadClass}>
            <th>이메일</th>
            <th>평가 참여</th>
          </tr>
        </thead>
        <tbody>
          {#each data.roster as person (person.email)}
            <tr class={tableRowClass}>
              <td>
                <span class={flex({ align: 'center', gap: '6px' })}>
                  <span class={css({ fontWeight: 'medium', wordBreak: 'break-all' })}>{person.email}</span>
                  {#if person.admin}
                    <span class={adminChipClass}>어드민</span>
                  {/if}
                </span>
              </td>
              <td>
                <span class={flex({ justify: 'flex-end' })}>
                  <Switch
                    name={`evaluating-${person.email}`}
                    checked={person.evaluating}
                    disabled={pending !== null}
                    onchange={(event) => setParticipation(person.email, event.currentTarget.checked)}
                  />
                </span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>

  {#if data.summaries.length === 0}
    <section class={sectionCardClass}>
      <p class={css({ fontSize: '14px', color: 'text.subtle', textAlign: 'center', paddingY: '12px' })}>아직 라운드가 없습니다.</p>
    </section>
  {/if}

  {#each data.summaries as summary (summary.roundId)}
    <section class={sectionCardClass}>
      <div class={flex({ align: 'center', gap: '10px' })}>
        <h2 class={css({ fontSize: '15px', fontWeight: 'bold' })}>{summary.label}</h2>
        <span class={chipClass}>{summary.evaluationLabel}</span>
        {#if !summary.active}
          <span class={chipClass}>비활성</span>
        {/if}
        <span class={css({ marginLeft: 'auto', fontSize: '13px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
          확정 판정 {summary.confirmedTotal} / {summary.taskTotal}건
        </span>
      </div>

      <div class={css({ marginTop: '12px', overflowX: 'auto' })}>
        <table class={numericTableClass}>
          <thead>
            <tr class={tableHeadClass}>
              <th>평가자</th>
              <th>확정 판정</th>
              <th>상태</th>
              <th>마지막 활동</th>
            </tr>
          </thead>
          <tbody>
            {#each summary.evaluators as evaluator (evaluator.email)}
              <tr class={tableRowClass}>
                <td class={css({ fontWeight: 'medium', wordBreak: 'break-all' })}>{evaluator.email}</td>
                <td>{evaluator.confirmed}</td>
                <td>
                  {#if evaluator.hasDraft}
                    <span class={chipClass}>작성 중</span>
                  {:else if evaluator.confirmed === 0}
                    <span class={attentionChipClass}>미참여</span>
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
