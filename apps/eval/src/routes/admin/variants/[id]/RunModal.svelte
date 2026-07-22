<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { untrack } from 'svelte';

  type Props = {
    corpusVersions: string[];
    running: boolean;
    error: string | null;
    onConfirm: (corpusVersion: string) => void;
    onCancel: () => void;
  };
  const { corpusVersions, running, error, onConfirm, onCancel }: Props = $props();

  // 이 컴포넌트는 {#if showRunModal} 블록 안에서만 렌더링되어 열릴 때마다 새로 마운트되므로
  // corpusVersions의 초깃값만 한 번 캡처해도 안전하다.
  let selected = $state(untrack(() => corpusVersions[0] ?? ''));

  // select 등 다이얼로그 내부 엘리먼트에 포커스가 있으면 바깥 div의 onkeydown까지 이벤트가 버블링되지 않아
  // Escape가 죽는다 — 포커스 위치와 무관하게 항상 잡히도록 window 레벨에서 처리한다.
  const onWindowKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') onCancel();
  };
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
    class={css({
      width: '400px',
      backgroundColor: 'surface.default',
      borderRadius: '12px',
      boxShadow: 'modal',
      padding: '24px',
    })}
    aria-modal="true"
    onclick={(e) => e.stopPropagation()}
    onkeydown={onWindowKeydown}
    role="dialog"
    tabindex="-1"
  >
    <h2 class={css({ fontSize: '16px', fontWeight: 'bold', marginBottom: '4px' })}>이 후보로 실행</h2>
    <p class={css({ fontSize: '13px', color: 'text.subtle', marginBottom: '16px' })}>실행할 코퍼스 버전을 선택하세요.</p>

    {#if corpusVersions.length === 0}
      <p class={css({ fontSize: '13px', color: 'text.faint' })}>적재된 코퍼스가 없습니다. 먼저 코퍼스를 적재하세요.</p>
    {:else}
      <label class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' })} for="run-modal-corpus-version">
        코퍼스 버전
      </label>
      <select
        id="run-modal-corpus-version"
        class={css({
          width: 'full',
          paddingX: '10px',
          paddingY: '8px',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '8px',
          backgroundColor: 'surface.default',
          fontSize: '14px',
          cursor: 'pointer',
          transition: '[border-color 0.15s ease]',
          _hover: { borderColor: 'border.strong' },
        })}
        bind:value={selected}
      >
        {#each corpusVersions as version (version)}
          <option value={version}>{version}</option>
        {/each}
      </select>
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
          transition: '[background-color 0.15s ease]',
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
          transition: '[background-color 0.15s ease]',
          _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
          ['&:hover:not(:disabled)']: { backgroundColor: 'accent.brand.hover' },
        })}
        disabled={running || corpusVersions.length === 0 || !selected}
        onclick={() => onConfirm(selected)}
        type="button"
      >
        {running ? '실행 시작 중…' : '실행 시작'}
      </button>
    </div>
  </div>
</div>
