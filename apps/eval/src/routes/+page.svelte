<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { enhance } from '$app/forms';
  import { page } from '$app/state';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData; form: { message?: string } | null };
  const { data, form }: Props = $props();

  let claiming = $state<string | null>(null);

  const finished = $derived(page.url.searchParams.has('finished'));
  const empty = $derived(page.url.searchParams.has('empty'));
  const claimable = $derived(data.rounds.reduce((sum, round) => sum + round.claimable, 0));

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
          <a class={headerLinkClass} href="/admin">어드민</a>
        {/if}
        <a class={headerLinkClass} data-sveltekit-reload href="/cdn-cgi/access/logout">로그아웃</a>
      </div>
    </header>

    {#if (finished || empty) && claimable === 0 && data.drafts.length === 0}
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
        <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.subtle' })}>새 평가가 배정되면 이 화면에 다시 나타납니다.</p>
      </section>
    {/if}

    {#if form?.message}
      <p
        class={css({
          marginBottom: '16px',
          paddingX: '14px',
          paddingY: '12px',
          borderRadius: '10px',
          backgroundColor: 'accent.danger.subtle',
          fontSize: '13px',
          color: 'text.danger',
        })}
      >
        {form.message}
      </p>
    {/if}

    {#if !data.evaluating}
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
        <p class={css({ fontSize: '14px', color: 'text.subtle' })}>
          동의는 접수됐습니다. 관리자가 명단에 올리면 이 화면에서 평가를 시작할 수 있습니다.
        </p>
      </section>
    {:else if data.rounds.length === 0}
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
        <p class={css({ fontSize: '14px', color: 'text.subtle' })}>열려 있는 라운드가 없습니다. 새 라운드가 열리면 여기에 표시됩니다.</p>
      </section>
    {:else}
      {#each data.rounds as round (round.id)}
        <section
          class={css({
            backgroundColor: 'surface.default',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '12px',
            padding: '24px',
            boxShadow: 'small',
            marginBottom: '16px',
          })}
        >
          <h2 class={css({ fontSize: '16px', fontWeight: 'bold', marginBottom: '14px' })}>{round.label}</h2>
          <div class={flex({ align: 'baseline', gap: '8px' })}>
            <span class={css({ fontSize: '32px', fontWeight: 'bold', fontVariantNumeric: 'tabular-nums' })}>{round.mine}</span>
            <span class={css({ fontSize: '14px', color: 'text.subtle' })}>건 평가 완료</span>
            <span class={css({ marginLeft: 'auto', fontSize: '13px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
              {#if round.claimable > 0}
                새로 시작할 수 있는 평가 {round.claimable}건
              {:else}
                새로 받을 평가 없음
              {/if}
            </span>
          </div>
          <div
            class={css({ marginTop: '12px', height: '6px', borderRadius: 'full', backgroundColor: 'surface.muted', overflow: 'hidden' })}
          >
            <div
              style:width={`${round.total === 0 ? 0 : Math.round((round.done / round.total) * 100)}%`}
              class={css({ height: 'full', backgroundColor: 'accent.brand.default' })}
            ></div>
          </div>
          <p class={css({ marginTop: '6px', fontSize: '12px', color: 'text.faint' })}>
            전체 진행 {round.done} / {round.total} — 평가자 전원의 평가를 합한 라운드 전체 수로, 내 할당량이 아닙니다.
          </p>

          <form
            action="?/claim"
            method="post"
            use:enhance={() => {
              claiming = round.id;
              return async ({ update }) => {
                await update();
                claiming = null;
              };
            }}
          >
            <input name="roundId" type="hidden" value={round.id} />
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
              disabled={claiming !== null || round.claimable === 0 || data.drafts.length > 0}
              type="submit"
            >
              {#if data.drafts.length > 0}
                작성 중인 평가를 먼저 마무리해 주세요
              {:else if round.claimable === 0}
                시작할 새 평가가 없습니다
              {:else if claiming === round.id}
                배정 중…
              {:else}
                다음 평가 시작
              {/if}
            </button>
          </form>
          <p class={css({ marginTop: '10px', fontSize: '12px', color: 'text.faint', textAlign: 'center' })}>
            {#if round.claimable > 0}
              {#if round.manuscript}
                이 라운드의 원고는 {round.manuscript.min.toLocaleString()}~{round.manuscript.max.toLocaleString()}자(평균 {round.manuscript.avg.toLocaleString()}자)입니다.
              {/if}
            {:else if data.drafts.length > 0}
              새로 배정받을 평가는 없습니다. 아래 작성 중인 평가를 마무리해 주세요.
            {:else}
              내 몫의 평가를 모두 마쳤습니다. 남은 평가는 다른 평가자에게 배정되어 있으며, 새 평가가 열리면 여기에 다시 표시됩니다.
            {/if}
          </p>
        </section>
      {/each}
    {/if}

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
