<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  let claiming = $state(false);

  const finished = $derived(page.url.searchParams.has('finished'));
  const progressPct = $derived(data.total === 0 ? 0 : Math.round((data.doneCount / data.total) * 100));

  const claim = async () => {
    claiming = true;
    try {
      const response = await fetch('/api/tasks/claim', { method: 'POST' });
      const { taskId } = (await response.json()) as { taskId: string | null };
      if (taskId) {
        await goto(`/tasks/${taskId}`);
      }
    } finally {
      claiming = false;
    }
  };
</script>

<main class={css({ minHeight: '[100dvh]', backgroundColor: 'surface.subtle' })}>
  <div class={css({ maxWidth: '560px', marginX: 'auto', paddingY: '64px', paddingX: '20px' })}>
    <header class={css({ marginBottom: '24px' })}>
      <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>문학 피드백 평가</h1>
      <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>{data.email}</p>
    </header>

    {#if finished && data.remaining === 0}
      <section
        class={css({
          backgroundColor: 'accent.success.subtle',
          borderRadius: '12px',
          padding: '24px',
          marginBottom: '16px',
          textAlign: 'center',
        })}
      >
        <p class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.success' })}>모든 평가를 마쳤습니다. 감사합니다!</p>
        <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.subtle' })}>새 태스크가 배정되면 이 화면에 다시 나타납니다.</p>
      </section>
    {/if}

    <section
      class={css({
        backgroundColor: 'surface.default',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '12px',
        padding: '24px',
        boxShadow: 'small',
      })}
    >
      <div class={flex({ align: 'baseline', gap: '8px' })}>
        <span class={css({ fontSize: '32px', fontWeight: 'bold' })}>{data.doneCount}</span>
        <span class={css({ fontSize: '14px', color: 'text.subtle' })}>/ {data.total} 판정 완료</span>
        {#if data.remaining > 0}
          <span class={css({ marginLeft: 'auto', fontSize: '13px', color: 'text.faint' })}>내가 할 수 있는 태스크 {data.remaining}개</span>
        {/if}
      </div>
      <div class={css({ marginTop: '12px', height: '6px', borderRadius: 'full', backgroundColor: 'surface.muted', overflow: 'hidden' })}>
        <div style:width={`${progressPct}%`} class={css({ height: 'full', backgroundColor: 'accent.brand.default' })}></div>
      </div>

      <button
        class={css({
          width: 'full',
          marginTop: '20px',
          paddingY: '12px',
          borderRadius: '10px',
          backgroundColor: 'accent.brand.default',
          color: 'text.bright',
          fontSize: '15px',
          fontWeight: 'bold',
          cursor: 'pointer',
          transition: '[background-color 0.15s ease]',
          _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
          ['&:hover:not(:disabled)']: { backgroundColor: 'accent.brand.hover' },
        })}
        disabled={claiming || data.remaining === 0}
        onclick={claim}
        type="button"
      >
        {data.remaining === 0 ? '남은 태스크가 없습니다' : claiming ? '배정 중…' : '다음 평가 시작'}
      </button>
      <p class={css({ marginTop: '10px', fontSize: '12px', color: 'text.faint', textAlign: 'center' })}>
        원문을 읽고 피드백 세트를 비교해 순위를 매기는 작업입니다. 한 편에 10–20분쯤 걸립니다.
      </p>
    </section>

    {#if data.drafts.length > 0}
      <section class={css({ marginTop: '16px' })}>
        <h2 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle', marginBottom: '8px' })}>작성 중인 평가</h2>
        <div class={flex({ direction: 'column', gap: '8px' })}>
          {#each data.drafts as draft (draft.taskId)}
            <a
              class={flex({
                align: 'center',
                justify: 'space-between',
                padding: '14px',
                borderWidth: '1px',
                borderColor: 'border.default',
                borderRadius: '10px',
                backgroundColor: 'surface.default',
                fontSize: '14px',
                transition: '[border-color 0.15s ease, box-shadow 0.15s ease]',
                _hover: { borderColor: 'border.strong', boxShadow: 'small' },
              })}
              href={`/tasks/${draft.taskId}`}
            >
              <span>임시 저장된 평가 이어서 하기</span>
              <span class={css({ fontSize: '12px', color: 'text.faint' })}>→</span>
            </a>
          {/each}
        </div>
      </section>
    {/if}
  </div>
</main>
