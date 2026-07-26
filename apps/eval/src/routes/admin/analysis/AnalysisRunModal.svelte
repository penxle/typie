<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { untrack } from 'svelte';

  type Props = {
    setLabel: string;
    corpusVersions: { version: string; count: number; characters: number }[];
    running: boolean;
    error: string | null;
    onConfirm: (corpusVersion: string, documentIds: string[] | undefined) => void;
    onCancel: () => void;
  };
  const { setLabel, corpusVersions, running, error, onConfirm, onCancel }: Props = $props();

  let selected = $state(untrack(() => corpusVersions[0]?.version ?? ''));
  let documentIdsText = $state('');
  const documentIds = $derived(
    documentIdsText
      .split(/[\s,]+/)
      .map((id) => id.trim())
      .filter((id) => id.length > 0),
  );
  const selectedCorpus = $derived(corpusVersions.find((c) => c.version === selected));

  const onWindowKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') onCancel();
  };

  const inputClass = css({
    width: 'full',
    paddingX: '10px',
    paddingY: '8px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    backgroundColor: 'surface.default',
    fontSize: '14px',
    transition: '[border-color 0.15s ease]',
    _hover: { borderColor: 'border.strong' },
  });
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div
  class={css({
    position: 'fixed',
    inset: '0',
    backgroundColor: 'black/50',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 'modal',
  })}
  onclick={onCancel}
  onkeydown={onWindowKeydown}
  role="presentation"
>
  <div
    class={css({ width: '440px', backgroundColor: 'surface.default', borderRadius: '12px', boxShadow: 'modal', padding: '24px' })}
    aria-modal="true"
    onclick={(e) => e.stopPropagation()}
    onkeydown={onWindowKeydown}
    role="dialog"
    tabindex="-1"
  >
    <h2 class={css({ fontSize: '16px', fontWeight: 'bold', marginBottom: '4px' })}>{setLabel} 실행</h2>
    <p class={css({ fontSize: '13px', color: 'text.subtle', marginBottom: '16px' })}>문서마다 워크플로가 하나씩 떠서 병렬로 돕니다.</p>

    {#if corpusVersions.length === 0}
      <p class={css({ fontSize: '13px', color: 'text.faint' })}>적재된 코퍼스가 없습니다.</p>
    {:else}
      <label class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' })} for="analysis-corpus">
        코퍼스 버전
      </label>
      <select id="analysis-corpus" class={`${inputClass} ${css({ cursor: 'pointer' })}`} bind:value={selected}>
        {#each corpusVersions as corpus (corpus.version)}
          <option value={corpus.version}>{corpus.version} — {corpus.count}편 · {corpus.characters.toLocaleString()}자</option>
        {/each}
      </select>

      <label
        class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginTop: '12px', marginBottom: '4px' })}
        for="analysis-docs"
      >
        문서 ID — 비우면 코퍼스 전체
      </label>
      <textarea
        id="analysis-docs"
        class={`${inputClass} ${css({ minHeight: '76px', fontSize: '12px', fontFamily: 'mono' })}`}
        placeholder="쉼표 또는 줄바꿈으로 구분"
        bind:value={documentIdsText}></textarea>
      <p class={css({ marginTop: '4px', fontSize: '12px', color: 'text.subtle' })}>
        {documentIds.length > 0 ? `${documentIds.length}편만 실행합니다.` : `코퍼스 전체 ${selectedCorpus?.count ?? 0}편을 실행합니다.`}
      </p>
    {/if}

    <p class={css({ marginTop: '10px', height: '16px', fontSize: '12px', color: 'text.danger' })}>{error ?? ''}</p>

    <div class={flex({ gap: '8px', marginTop: '10px' })}>
      <button
        class={css({
          flex: '1',
          paddingY: '9px',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '8px',
          fontSize: '13px',
          color: 'text.subtle',
          cursor: 'pointer',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        onclick={onCancel}
        type="button"
      >
        취소
      </button>
      <button
        class={css({
          flex: '1',
          paddingY: '9px',
          borderRadius: '8px',
          backgroundColor: 'accent.brand.default',
          color: 'text.bright',
          fontSize: '13px',
          fontWeight: 'bold',
          cursor: 'pointer',
          _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
          ['&:hover:not(:disabled)']: { backgroundColor: 'accent.brand.hover' },
        })}
        disabled={running || corpusVersions.length === 0 || !selected}
        onclick={() => onConfirm(selected, documentIds.length > 0 ? documentIds : undefined)}
        type="button"
      >
        {running ? '실행 시작 중…' : '실행 시작'}
      </button>
    </div>
  </div>
</div>
