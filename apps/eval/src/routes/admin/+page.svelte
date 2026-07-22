<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import VariantStatusBadge from './VariantStatusBadge.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const STAGE_LABELS: Record<string, string> = { summarize: '요약', meta: '메타', analyze: '분석' };

  const cardClass = css({
    backgroundColor: 'surface.default',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '12px',
    padding: '20px',
    boxShadow: 'small',
  });

  const cardTitleClass = css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle', marginBottom: '12px' });
</script>

<Helmet title="관리자 홈" trailing="타이피 평가" />

<div class={css({ maxWidth: '1080px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <header class={css({ marginBottom: '24px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>관리자 홈</h1>
    <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>프롬프트 후보 · 실행 · 적용 현황을 한눈에 확인합니다.</p>
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
    {#if data.nextAction.kind === 'create-variant'}
      <p class={css({ fontSize: '14px', color: 'text.default' })}>아직 프롬프트 후보가 없습니다. 새 후보를 만들어 시작하세요.</p>
      <a
        class={css({
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
        })}
        href="/admin/variants/new"
      >
        후보 만들기
      </a>
    {:else if data.nextAction.kind === 'run'}
      <p class={css({ fontSize: '14px', color: 'text.default' })}>후보는 있지만 아직 실행한 적이 없습니다. 후보를 골라 실행해보세요.</p>
      <a
        class={css({
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
        })}
        href="/admin/variants"
      >
        후보 목록에서 실행
      </a>
    {:else if data.nextAction.kind === 'view-run'}
      <p class={css({ fontSize: '14px', color: 'text.default' })}>실행이 진행 중입니다.</p>
      <a
        class={css({
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
        })}
        href={`/admin/runs/${data.nextAction.runId}`}
      >
        실행 보기
      </a>
    {:else}
      <p class={css({ fontSize: '14px', color: 'text.default' })}>모든 후보가 실행되었습니다. 결과를 검토하거나 새 후보를 만들어보세요.</p>
      <a
        class={css({
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
        })}
        href="/admin/variants"
      >
        후보 목록 보기
      </a>
    {/if}
  </section>

  <div class={grid({ columns: 3, gap: '16px' })}>
    <div class={cardClass}>
      <h2 class={cardTitleClass}>현재 적용된 프롬프트</h2>
      <div class={flex({ direction: 'column', gap: '8px' })}>
        {#each data.currentByStage as stage (stage.stage)}
          <div class={flex({ align: 'center', justify: 'space-between', gap: '8px' })}>
            <span class={css({ fontSize: '13px', color: 'text.subtle' })}>{STAGE_LABELS[stage.stage] ?? stage.stage}</span>
            {#if stage.variantLabel}
              <span class={css({ fontSize: '13px', fontWeight: 'bold' })}>{stage.variantLabel}</span>
            {:else}
              <span class={css({ fontSize: '13px', color: 'text.faint' })}>미적용</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    <div class={cardClass}>
      <h2 class={cardTitleClass}>최신 코퍼스 버전</h2>
      {#if data.latestCorpusVersion}
        <p class={css({ fontSize: '20px', fontWeight: 'bold' })}>{data.latestCorpusVersion}</p>
      {:else}
        <p class={css({ fontSize: '14px', color: 'text.faint' })}>적재된 코퍼스가 없습니다.</p>
      {/if}
    </div>

    <div class={cardClass}>
      <h2 class={cardTitleClass}>진행 중 실행</h2>
      <p class={css({ fontSize: '20px', fontWeight: 'bold' })}>{data.runningCount}개</p>
    </div>
  </div>

  <section class={css({ marginTop: '16px' })}>
    <div class={cardClass}>
      <div class={flex({ align: 'center', justify: 'space-between', marginBottom: '12px' })}>
        <h2 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle' })}>후보별 상태</h2>
        <a class={css({ fontSize: '12px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/admin/variants">전체 보기 →</a>
      </div>

      {#if data.variantSummaries.length === 0}
        <p class={css({ paddingY: '20px', textAlign: 'center', fontSize: '13px', color: 'text.faint' })}>아직 만들어진 후보가 없습니다.</p>
      {:else}
        <div class={flex({ direction: 'column', gap: '6px' })}>
          {#each data.variantSummaries as variant (variant.id)}
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
              href={`/admin/variants/${variant.id}`}
            >
              <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>{variant.label}</span>
              <VariantStatusBadge status={variant.status} />
            </a>
          {/each}
        </div>
      {/if}
    </div>
  </section>
</div>
