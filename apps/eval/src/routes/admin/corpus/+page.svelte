<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { grid } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { goto } from '$app/navigation';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  let corpusVersion = $state('');
  let size = $state(20);
  let submitting = $state(false);
  let submitError = $state<string | null>(null);

  const submit = async () => {
    submitError = null;

    if (!corpusVersion.trim()) {
      submitError = '버전을 입력하세요.';
      return;
    }
    if (!Number.isSafeInteger(size) || size < 1) {
      submitError = '크기는 1 이상의 정수여야 합니다.';
      return;
    }

    submitting = true;
    try {
      const response = await fetch('/admin/api/runs', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ kind: 'sampling', corpusVersion: corpusVersion.trim(), size }),
      });
      if (!response.ok) {
        submitError = `샘플링 시작에 실패했습니다 (${response.status}).`;
        return;
      }
      const { runId } = (await response.json()) as { runId: string };
      await goto(`/admin/runs/${runId}`);
    } finally {
      submitting = false;
    }
  };
</script>

<Helmet title="코퍼스" trailing="타이피 평가" />

<div class={css({ maxWidth: '960px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>코퍼스</h1>
    <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>동결된 코퍼스 버전 목록입니다.</p>
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
    <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '12px' })}>새 코퍼스 샘플링</h2>
    <div class={grid({ columns: 3, gap: '12px', alignItems: 'end' })}>
      <div>
        <label class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' })} for="new-corpus-version">
          버전
        </label>
        <input
          id="new-corpus-version"
          class={css({
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
          })}
          placeholder="예: v3"
          type="text"
          bind:value={corpusVersion}
        />
      </div>
      <div>
        <label class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' })} for="new-corpus-size">
          크기 (문서 수)
        </label>
        <input
          id="new-corpus-size"
          class={css({
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
          })}
          min="1"
          type="number"
          bind:value={size}
        />
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
        disabled={submitting}
        onclick={submit}
        type="button"
      >
        {submitting ? '시작 중…' : '샘플링 시작'}
      </button>
    </div>
    <p class={css({ marginTop: '8px', height: '16px', fontSize: '12px', color: 'text.danger' })}>{submitError ?? ''}</p>
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
    {#if data.versions.length === 0}
      <p class={css({ paddingY: '48px', textAlign: 'center', fontSize: '14px', color: 'text.faint' })}>적재된 코퍼스가 없습니다.</p>
    {:else}
      <table class={css({ width: 'full', fontSize: '13px', '& td, & th': { paddingX: '16px', paddingY: '10px', textAlign: 'left' } })}>
        <thead>
          <tr
            class={css({
              '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
            })}
          >
            <th>버전</th>
            <th>문서 수</th>
            <th>총 글자수</th>
            <th>최초 삽입 시각</th>
          </tr>
        </thead>
        <tbody>
          {#each data.versions as version (version.corpusVersion)}
            <tr class={css({ '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' } })}>
              <td>
                <a
                  class={css({ fontWeight: 'bold', color: 'text.link', transition: '[color 0.15s ease]', _hover: { color: 'text.brand' } })}
                  href={`/admin/corpus/${version.corpusVersion}`}
                >
                  {version.corpusVersion}
                </a>
              </td>
              <td>{version.docCount.toLocaleString()}</td>
              <td>{version.totalCharacters.toLocaleString()}자</td>
              <td>{new Date(version.firstInsertedAt).toLocaleString('ko')}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
</div>
