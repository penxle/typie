<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  let claiming = $state(false);

  const finished = $derived(page.url.searchParams.has('finished'));
  const progressPct = $derived(data.round.required === 0 ? 0 : Math.round((data.round.done / data.round.required) * 100));

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

  const headerLinkClass = css({
    flexShrink: '0',
    paddingX: '10px',
    paddingY: '6px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '6px',
    fontSize: '13px',
    color: 'text.faint',
    transition: '[background-color 0.15s ease, color 0.15s ease]',
    _hover: { backgroundColor: 'surface.default', color: 'text.default' },
  });
</script>

<Helmet title="평가 큐" trailing="타이피 평가" />

<main class={css({ minHeight: '[100dvh]', backgroundColor: 'surface.subtle' })}>
  <div class={css({ maxWidth: '560px', marginX: 'auto', paddingY: '64px', paddingX: '20px' })}>
    <header class={flex({ align: 'flex-start', justify: 'space-between', gap: '16px', marginBottom: '24px' })}>
      <div>
        <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>문학 피드백 평가</h1>
        <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>{data.email}</p>
      </div>
      <div class={flex({ align: 'center', gap: '8px', flexShrink: '0' })}>
        <ThemeToggle />
        {#if data.isAdmin}
          <a class={headerLinkClass} href="/dashboard">대시보드</a>
          <a class={headerLinkClass} href="/admin">어드민</a>
        {/if}
        <a class={headerLinkClass} data-sveltekit-reload href="/cdn-cgi/access/logout">로그아웃</a>
      </div>
    </header>

    {#if finished && data.remaining === 0 && data.drafts.length === 0}
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
        <span class={css({ fontSize: '14px', color: 'text.subtle' })}>건 판정 완료</span>
        <span class={css({ marginLeft: 'auto', fontSize: '13px', color: 'text.faint' })}>
          {#if data.remaining > 0}
            새로 시작할 수 있는 태스크 {data.remaining}개
          {:else if data.drafts.length > 0}
            작성 중인 평가 {data.drafts.length}건
          {/if}
        </span>
      </div>
      <div class={css({ marginTop: '12px', height: '6px', borderRadius: 'full', backgroundColor: 'surface.muted', overflow: 'hidden' })}>
        <div style:width={`${progressPct}%`} class={css({ height: 'full', backgroundColor: 'accent.brand.default' })}></div>
      </div>
      <p class={css({ marginTop: '6px', fontSize: '12px', color: 'text.faint' })}>
        라운드 전체 진행 {data.round.done} / {data.round.required} — 라운드에 필요한 판정 중 채워진 수입니다.
      </p>
      {#if data.quota}
        <p class={css({ marginTop: '2px', fontSize: '12px', color: 'text.faint' })}>
          작업이 한 사람에게 몰리지 않도록 1인당 최대 {data.quota.limit}건까지 배정됩니다 — 내 배정 {data.quota.used} / {data.quota
            .limit}건.
        </p>
      {/if}

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
        {data.remaining === 0 ? '시작할 새 태스크가 없습니다' : claiming ? '배정 중…' : '다음 평가 시작'}
      </button>
      <p class={css({ marginTop: '10px', fontSize: '12px', color: 'text.faint', textAlign: 'center' })}>
        {#if data.remaining > 0}
          원문을 읽고 피드백 세트를 비교해 점수를 매기는 작업입니다. 한 편에 10–20분쯤 걸립니다.
        {:else if data.drafts.length > 0}
          새로 배정받을 태스크는 없습니다. 아래 작성 중인 평가를 마무리해 주세요.
        {:else}
          내 몫의 평가를 모두 마쳤습니다. 남은 태스크는 다른 평가자에게 배정되어 있으며, 새 태스크가 열리면 여기에 다시 표시됩니다.
        {/if}
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
