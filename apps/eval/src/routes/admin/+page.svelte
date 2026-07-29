<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { cardClass, cardTitleClass, pageClass, pageDescClass, pageTitleClass, rowLinkClass } from '$lib/styles.ts';
  import PromptSetStatusBadge from './PromptSetStatusBadge.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const ctaClass = css({
    display: 'inline-block',
    marginTop: '10px',
    paddingX: '14px',
    paddingY: '8px',
    borderRadius: '8px',
    backgroundColor: 'accent.brand.default',
    color: 'text.bright',
    fontSize: '13px',
    fontWeight: 'bold',
    transition: '[background-color 0.15s ease]',
    _hover: { backgroundColor: 'accent.brand.hover' },
  });

  const quietCtaClass = css({
    display: 'inline-block',
    marginTop: '10px',
    paddingX: '14px',
    paddingY: '8px',
    borderRadius: '8px',
    borderWidth: '1px',
    borderColor: 'border.strong',
    color: 'text.default',
    fontSize: '13px',
    fontWeight: 'bold',
    transition: '[background-color 0.15s ease]',
    _hover: { backgroundColor: 'surface.muted' },
  });
</script>

<Helmet title="관리자 홈" trailing="타이피 평가" />

<div class={pageClass}>
  <header class={css({ marginBottom: '24px' })}>
    <h1 class={pageTitleClass}>관리자 홈</h1>
    <p class={pageDescClass}>문서 · 프롬프트 묶음 · 실행 현황을 한눈에 확인합니다.</p>
  </header>

  <section
    class={css({
      marginBottom: '24px',
      padding: '20px',
      borderRadius: '12px',
      borderWidth: '1px',
      backgroundColor: 'accent.brand.subtle',
      borderColor: 'border.brand',
    })}
  >
    <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '6px' })}>다음 행동</h2>
    {#if data.nextAction.kind === 'create-prompt-set'}
      <p class={css({ fontSize: '14px', color: 'text.default' })}>아직 프롬프트 묶음이 없습니다. 새 묶음을 만들어 시작하세요.</p>
      <a class={ctaClass} href="/admin/prompt-sets">묶음 만들기</a>
    {:else if data.nextAction.kind === 'run'}
      <p class={css({ fontSize: '14px', color: 'text.default' })}>묶음은 있지만 아직 실행한 적이 없습니다. 문서를 골라 실행해보세요.</p>
      <a class={ctaClass} href="/admin/documents">문서 목록에서 실행</a>
    {:else if data.nextAction.kind === 'view-run'}
      <p class={css({ fontSize: '14px', color: 'text.default' })}>실행이 진행 중입니다.</p>
      <a class={ctaClass} href={`/admin/runs/${data.nextAction.runId}`}>실행 보기</a>
    {:else}
      <p class={css({ fontSize: '14px', color: 'text.default' })}>진행 중인 실행이 없습니다. 결과를 검토하거나 새 묶음을 만들어보세요.</p>
      <a class={quietCtaClass} href="/admin/runs">실행 목록 보기</a>
    {/if}
  </section>

  <div class={grid({ columns: 2, gap: '16px' })}>
    <div class={cardClass}>
      <h2 class={cardTitleClass}>들여온 문서</h2>
      <p class={css({ fontSize: '20px', fontWeight: 'bold' })}>{data.documentCount}편</p>
    </div>

    <div class={cardClass}>
      <h2 class={cardTitleClass}>진행 중 실행</h2>
      <p class={css({ fontSize: '20px', fontWeight: 'bold' })}>{data.runningCount}개</p>
    </div>
  </div>

  <section class={css({ marginTop: '16px' })}>
    <div class={cardClass}>
      <div class={flex({ align: 'center', justify: 'space-between', marginBottom: '12px' })}>
        <h2 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle' })}>묶음별 상태</h2>
        <a class={rowLinkClass} href="/admin/prompt-sets">전체 보기 →</a>
      </div>

      {#if data.promptSetSummaries.length === 0}
        <p class={css({ paddingY: '20px', textAlign: 'center', fontSize: '13px', color: 'text.faint' })}>아직 만들어진 묶음이 없습니다.</p>
      {:else}
        <div class={flex({ direction: 'column', gap: '6px' })}>
          {#each data.promptSetSummaries as set (set.id)}
            <a
              class={flex({
                align: 'center',
                justify: 'space-between',
                gap: '8px',
                paddingX: '10px',
                paddingY: '8px',
                borderRadius: '8px',
                transition: '[background-color 0.15s ease]',
                _hover: { backgroundColor: 'surface.subtle' },
              })}
              href={`/admin/prompt-sets/${set.id}`}
            >
              <span class={flex({ align: 'center', gap: '8px' })}>
                <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>{set.label}</span>
                <span class={css({ fontSize: '12px', color: 'text.faint' })}>{set.generationLabel}</span>
              </span>
              <PromptSetStatusBadge status={set.status} />
            </a>
          {/each}
        </div>
      {/if}
    </div>
  </section>
</div>
