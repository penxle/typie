<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { untrack } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import CostCell from '../lib/CostCell.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  type Outcome = {
    accepted: { id: string; refId: string; characterCount: number }[];
    rejected: { refId: string; reason: string }[];
    run: { runId: string; spawnedCount: number } | null;
  };

  let raw = $state('');
  // 목록이 다시 불려도 고른 값을 되돌리지 않는다 — 초깃값만 한 번 잡는다.
  let promptSetId = $state(untrack(() => data.promptSets[0]?.id ?? ''));
  let submitting = $state(false);
  let submitError = $state<string | null>(null);
  let outcome = $state<Outcome | null>(null);
  let copied = $state<string | null>(null);

  const inputStyle = css.raw({
    width: 'full',
    paddingX: '10px',
    paddingY: '8px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    fontSize: '14px',
    backgroundColor: 'surface.default',
    transition: '[border-color 0.15s ease]',
    _hover: { borderColor: 'border.strong' },
  });
  const inputClass = css(inputStyle);
  const labelClass = css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' });

  const submit = async () => {
    submitError = null;
    outcome = null;

    const documentIds = [...new Set(raw.split(/[\s,]+/).filter((s) => s.length > 0))];
    if (documentIds.length === 0) {
      submitError = '문서 ID를 입력하세요.';
      return;
    }
    if (!promptSetId) {
      submitError = '프롬프트 세트를 고르세요.';
      return;
    }

    submitting = true;
    try {
      const response = await fetch('/admin/api/reads', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ documentIds, promptSetId }),
      });
      if (!response.ok) {
        const body = await response.text();
        submitError = `들이기에 실패했습니다 (${response.status}). ${body.slice(0, 300)}`;
        return;
      }
      outcome = (await response.json()) as Outcome;
      raw = '';
      await invalidateAll();
    } finally {
      submitting = false;
    }
  };

  const copy = async (setId: string) => {
    await navigator.clipboard.writeText(`${location.origin}/reads/${setId}`);
    copied = setId;
    setTimeout(() => (copied = null), 1500);
  };
</script>

<Helmet title="개인 열람" trailing="타이피 평가 어드민" />

<div class={css({ maxWidth: '960px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>개인 열람</h1>
    <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>
      본인 글의 피드백을 읽어볼 수 있게 따로 들이는 자리입니다. 여기 들인 글은 평가 대상 코퍼스에 섞이지 않고, 실행·라운드의 코퍼스 목록에도
      나오지 않습니다.
    </p>
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
    <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '12px' })}>글 들이고 피드백 만들기</h2>

    <div class={css({ marginBottom: '12px' })}>
      <label class={labelClass} for="read-document-ids">문서 ID</label>
      <textarea
        id="read-document-ids"
        class={css(inputStyle, { height: '96px', resize: 'vertical', fontFamily: 'mono' })}
        placeholder="타이피 문서 ID를 줄바꿈·쉼표·공백으로 구분해 넣으세요"
        bind:value={raw}></textarea>
      <p class={css({ marginTop: '4px', fontSize: '12px', color: 'text.faint' })}>
        공개로 설정된 글만 들일 수 있습니다 — 표집 코퍼스와 같은 관문을 씁니다.
      </p>
    </div>

    <div class={flex({ gap: '12px', align: 'end' })}>
      <div class={css({ flex: '1' })}>
        <label class={labelClass} for="read-prompt-set">프롬프트 세트</label>
        <select id="read-prompt-set" class={inputClass} bind:value={promptSetId}>
          {#each data.promptSets as set (set.id)}
            <option value={set.id}>{set.label}</option>
          {/each}
        </select>
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
          transition: '[background-color 0.15s ease]',
          _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
          ['&:hover:not(:disabled)']: { backgroundColor: 'accent.brand.hover' },
        })}
        disabled={submitting || data.promptSets.length === 0}
        onclick={submit}
        type="button"
      >
        {submitting ? '들이는 중…' : '들이고 실행'}
      </button>
    </div>

    <p class={css({ marginTop: '8px', minHeight: '16px', fontSize: '12px', color: 'text.danger' })}>{submitError ?? ''}</p>

    {#if outcome}
      <div class={css({ marginTop: '4px', fontSize: '13px' })}>
        {#if outcome.run}
          <p>
            {outcome.accepted.length}편을 들여 실행을 걸었습니다.
            <a class={css({ color: 'text.link', textDecoration: 'underline' })} href={`/admin/runs/${outcome.run.runId}`}>실행 보기 →</a>
          </p>
        {/if}
        {#each outcome.rejected as item (item.refId)}
          <p class={css({ color: 'text.danger' })}>{item.refId} — {item.reason}</p>
        {/each}
      </div>
    {/if}
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
    {#if data.reads.length === 0}
      <p class={css({ paddingY: '48px', textAlign: 'center', fontSize: '14px', color: 'text.faint' })}>
        아직 만들어진 열람 링크가 없습니다.
      </p>
    {:else}
      <table class={css({ width: 'full', fontSize: '13px', borderCollapse: 'collapse' })}>
        <thead>
          <tr
            class={css({
              textAlign: 'left',
              '& th': {
                paddingY: '10px',
                paddingX: '16px',
                color: 'text.faint',
                fontWeight: 'medium',
                borderBottomWidth: '1px',
                borderColor: 'border.default',
              },
            })}
          >
            <th>문서 ID</th>
            <th>글자수</th>
            <th>실행</th>
            <th>비용</th>
            <th>만든 시각</th>
            <th>열람 링크</th>
          </tr>
        </thead>
        <tbody>
          {#each data.reads as read (read.setId)}
            <tr class={css({ '& td': { paddingY: '10px', paddingX: '16px', borderBottomWidth: '1px', borderColor: 'border.subtle' } })}>
              <td class={css({ fontFamily: 'mono' })}>{read.refId}</td>
              <td>{read.characterCount.toLocaleString()}자</td>
              <td>
                <a class={css({ color: 'text.link', textDecoration: 'underline' })} href={`/admin/runs/${read.runId}`}>
                  {read.runStatus}
                </a>
              </td>
              <td><CostCell cost={read.cost} tokens={read.tokens} /></td>
              <td class={css({ color: 'text.faint' })}>{new Date(read.createdAt).toLocaleString('ko')}</td>
              <td>
                <div class={flex({ align: 'center', gap: '8px' })}>
                  <a class={css({ color: 'text.link', textDecoration: 'underline' })} href={`/reads/${read.setId}`}>열어보기</a>
                  <button
                    class={css({ fontSize: '12px', color: 'text.subtle', cursor: 'pointer', _hover: { color: 'text.default' } })}
                    onclick={() => copy(read.setId)}
                    type="button"
                  >
                    {copied === read.setId ? '복사됨' : '링크 복사'}
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
</div>
