<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { SvelteSet } from 'svelte/reactivity';
  import { enhance } from '$app/forms';
  import { invalidateAll } from '$app/navigation';
  import {
    attentionChipClass,
    checkboxClass,
    chipClass,
    dangerButtonClass,
    emptyClass,
    formInputClass,
    formLabelClass,
    formNoticeClass,
    formSubmitClass,
    pageClass,
    pageDescClass,
    pageTitleClass,
    panelClass,
    tableClass,
    tableHeadClass,
    tableRowClass,
  } from '$lib/styles.ts';
  import { usePolling } from '../lib/poll.svelte.ts';
  import RunStatusBadge from '../runs/RunStatusBadge.svelte';
  import type { PageData } from './$types';

  type Props = {
    data: PageData;
    form: {
      message?: string;
      intake?: { accepted: number; reused: number; rejected: string[] };
      sampled?: boolean;
    } | null;
  };
  const { data, form }: Props = $props();

  const selected = new SvelteSet<string>();
  let promptSetId = $state('');

  const notice = $derived.by(() => {
    if (form?.message) return { tone: 'danger' as const, text: form.message };
    if (form?.intake) {
      const rejected = form.intake.rejected.length > 0 ? `, 본문 없음 ${form.intake.rejected.length}건` : '';
      return { tone: 'success' as const, text: `들여옴 ${form.intake.accepted}건, 이미 있음 ${form.intake.reused}건${rejected}` };
    }
    if (form?.sampled) return { tone: 'success' as const, text: '표집을 시작했습니다.' };
    return null;
  });

  const SAMPLING_PHASES: Record<string, string> = {
    candidates: '후보 수집',
    classify: '분류',
    extract: '추출',
    freeze: '동결',
  };

  const running = $derived(data.samplings.some((s) => s.status === 'running' || s.status === 'pending'));
  usePolling(() => invalidateAll(), 3000, { enabled: () => running });

  // running은 로드 시점 데이터라 요청이 날아가는 동안의 연타를 못 막는다 — 제출 중 상태를 따로 든다.
  let samplingBusy = $state(false);
  let runBusy = $state(false);
</script>

<Helmet title="문서" trailing="타이피 평가" />

<div class={pageClass}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={pageTitleClass}>문서</h1>
    <p class={pageDescClass}>실서비스에서 들여온 원고 목록입니다.</p>
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
    <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '12px' })}>새 문서 들여오기</h2>
    <form action="?/intake" method="post" use:enhance>
      <div class={grid({ columns: 3, gap: '12px', alignItems: 'end' })}>
        <div class={css({ gridColumn: '[span 2]' })}>
          <label class={formLabelClass} for="new-document-refs">문서 식별자 (공백·쉼표 구분)</label>
          <input id="new-document-refs" name="refIds" class={formInputClass} required type="text" />
        </div>
        <button class={formSubmitClass} type="submit">들여오기</button>
      </div>
    </form>
    <p class={css({ marginTop: '8px', fontSize: '12px', color: 'text.faint' })}>
      식별자로 지목해 곧장 가져옵니다. 표집이 거치는 공개 관문을 지나지 않으므로 비공개 글도 들어옵니다 — 반입한 문서는 실행해서 열람할 수는
      있지만 라운드에는 넣을 수 없습니다.
    </p>
    <p class={[formNoticeClass, css({ color: notice?.tone === 'success' ? 'text.success' : 'text.danger' })]}>{notice?.text ?? ''}</p>
  </section>

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
    <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '12px' })}>새 표집</h2>
    <form
      action="?/sample"
      method="post"
      use:enhance={() => {
        samplingBusy = true;
        return async ({ update }) => {
          await update();
          samplingBusy = false;
        };
      }}
    >
      <div class={grid({ columns: 3, gap: '12px', alignItems: 'end' })}>
        <div class={css({ gridColumn: '[span 2]' })}>
          <label class={formLabelClass} for="sampling-size">크기 (문서 수)</label>
          <input id="sampling-size" name="size" class={formInputClass} min="1" required type="number" value="20" />
        </div>
        <button class={formSubmitClass} disabled={running || samplingBusy} type="submit">
          {running || samplingBusy ? '표집 중…' : '표집 시작'}
        </button>
      </div>
    </form>
    <p class={css({ marginTop: '8px', fontSize: '12px', color: 'text.faint' })}>
      실서비스의 공개 원고 중에서 무작위로 골라 문서로 들입니다. 이미 들인 글은 건너뜁니다.
    </p>

    {#if data.samplings.length > 0}
      <div class={flex({ direction: 'column', gap: '4px', marginTop: '12px' })}>
        {#each data.samplings as sampling (sampling.id)}
          <div class={flex({ align: 'center', gap: '8px', fontSize: '12px', color: 'text.subtle' })}>
            <RunStatusBadge status={sampling.status} />
            <span class={css({ fontVariantNumeric: 'tabular-nums' })}>{sampling.size}편</span>
            {#if sampling.phase}
              <span>{SAMPLING_PHASES[sampling.phase] ?? sampling.phase}</span>
            {/if}
            <span class={css({ color: 'text.faint' })}>{new Date(sampling.createdAt).toLocaleString('ko')}</span>
            {#if sampling.error}
              <span class={css({ color: 'text.danger', wordBreak: 'break-all' })}>{sampling.error}</span>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>

  {#if data.genres.length > 0}
    {@const total = data.genres.reduce((sum, g) => sum + g.count, 0)}
    <section
      class={css({
        marginBottom: '24px',
        backgroundColor: 'surface.default',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '12px',
        boxShadow: 'small',
        padding: '16px',
      })}
    >
      <h2 class={css({ fontSize: '13px', fontWeight: 'bold', marginBottom: '10px' })}>장르 분포 (표집분)</h2>
      <div class={flex({ direction: 'column', gap: '4px' })}>
        {#each data.genres as genre (genre.key)}
          <div
            class={flex({
              justify: 'space-between',
              fontSize: '13px',
              color: genre.key === 'unclassified' ? 'text.faint' : 'text.default',
            })}
          >
            <span>{genre.name}</span>
            <span class={css({ color: 'text.subtle' })}>
              {genre.count.toLocaleString()}건 ({total > 0 ? Math.round((genre.count / total) * 100) : 0}%)
            </span>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <form
    action="?/run"
    method="post"
    use:enhance={() => {
      runBusy = true;
      return async ({ update }) => {
        await update();
        runBusy = false;
      };
    }}
  >
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
      <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '12px' })}>선택한 문서 실행</h2>
      <div class={grid({ columns: 3, gap: '12px', alignItems: 'end' })}>
        <div class={css({ gridColumn: '[span 2]' })}>
          <label class={formLabelClass} for="run-prompt-set">프롬프트 묶음</label>
          <select id="run-prompt-set" name="promptSetId" class={formInputClass} bind:value={promptSetId}>
            <option value="">선택하세요</option>
            {#each data.promptSets as set (set.id)}
              <option value={set.id}>{set.label}</option>
            {/each}
          </select>
        </div>
        <button class={formSubmitClass} disabled={!promptSetId || selected.size === 0 || runBusy} type="submit">
          {runBusy ? '실행 거는 중…' : `선택한 ${selected.size}편 실행`}
        </button>
      </div>
    </section>

    <section class={panelClass}>
      {#if data.documents.length === 0}
        <p class={emptyClass}>들여온 문서가 없습니다.</p>
      {:else}
        <table class={tableClass}>
          <thead>
            <tr class={tableHeadClass}>
              <th></th>
              <th>문서</th>
              <th>출처</th>
              <th>글자 수</th>
              <th>개행 수</th>
              <th>실행</th>
              <th>라운드</th>
              <th>들어온 날</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each data.documents as document (document.id)}
              <tr class={tableRowClass}>
                <td>
                  <input
                    name="documentIds"
                    class={checkboxClass}
                    checked={selected.has(document.id)}
                    onchange={(e) => (e.currentTarget.checked ? selected.add(document.id) : selected.delete(document.id))}
                    type="checkbox"
                    value={document.id}
                  />
                </td>
                <td>
                  <a
                    class={css({
                      fontWeight: 'bold',
                      color: 'text.link',
                      transition: '[color 0.15s ease]',
                      _hover: { color: 'text.brand' },
                    })}
                    href="/admin/documents/{document.id}"
                  >
                    {document.refId}
                  </a>
                </td>
                <td>
                  <span class={document.kind === 'sampled' ? chipClass : attentionChipClass}>
                    {document.kind === 'sampled' ? '표집' : '반입'}
                  </span>
                </td>
                <td>{document.characterCount.toLocaleString()}</td>
                <td>{document.lineBreakCount.toLocaleString()}</td>
                <td>{document.runs}</td>
                <td class={css({ color: document.rounds.length > 0 ? 'text.default' : 'text.faint' })}>
                  {document.rounds.length > 0 ? document.rounds.join(', ') : '—'}
                </td>
                <td class={css({ color: 'text.faint' })}>{new Date(document.createdAt).toLocaleString('ko')}</td>
                <td>
                  {#if document.runs === 0}
                    <button name="id" class={dangerButtonClass} formaction="?/remove" type="submit" value={document.id}>삭제</button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </section>
  </form>
</div>
