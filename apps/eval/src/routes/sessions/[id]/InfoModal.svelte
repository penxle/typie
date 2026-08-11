<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Modal } from '@typie/ui/components';
  import { formatKrw } from '$lib/feedback/pricing.ts';
  import { summarizeAgentCosts } from '$lib/feedback/stage-cost.ts';
  import type { PriceTable } from '$lib/feedback/pricing.ts';
  import type { ModelConfig, TierName } from '$lib/feedback/tiers.ts';
  import type { UsageFold } from '$lib/feedback/types.ts';

  // folds의 원천은 상태가 가른다(호출처) — 실행 중은 턴 누적 합성, 종결은 원장 기록. null은 기록 부재라 비용 열을 세우지 않는다.
  // priceTable은 로드가 prism에서 걷어 내려보낸 정본 사영이다 — null(수신 실패)이면 금액이 전부 미상(—)으로 선다.
  type Props = {
    open: boolean;
    tier: TierName;
    workflowId: string | null;
    refId: string;
    modelConfig: ModelConfig | null;
    priceTable: PriceTable | null;
    folds: UsageFold[] | null;
    running: boolean;
    lowerBound: boolean;
  };
  let { open = $bindable(), tier, workflowId, refId, modelConfig, priceTable, folds, running, lowerBound }: Props = $props();

  // 구성 스냅샷의 에이전트가 표의 행이다 — fold는 base 이름으로 그 행에 접히고, 행 밖의 fold는 기타로 남는다.
  const costs = $derived(folds === null ? null : summarizeAgentCosts(folds, Object.keys(modelConfig ?? {}), priceTable));

  // 하한의 두 얼굴 — 실행 중은 계속 커질 값이고, 종결인데 하한이면 아직 안 접힌 집계가 남은 기록이다. 둘 다 아니면 확정값이라 각주가 없다.
  const accrualNote = $derived(
    running
      ? '진행 중 누적이에요 — 최종 비용은 이보다 커질 수 있어요.'
      : lowerBound
        ? '집계가 다 접히기 전 기록이에요 — 실제 비용은 이보다 클 수 있어요.'
        : null,
  );

  // 부재(아직 안 돎)와 미상(단가·구성 모름)은 화면에서 같은 —다 — 구분은 합계 계산만 안다(미상만 오염시킨다).
  const amount = (value: number | null | undefined): string => (value == null ? '—' : formatKrw(value));

  // 운영자 전용 표식이라 상태 배지보다 한 급 눌러 세운다 — 코드 명칭을 그대로 쓰는 자리라 mono다.
  const tierBadgeClass = css({
    flexShrink: '0',
    paddingX: '8px',
    paddingY: '2px',
    borderRadius: 'full',
    backgroundColor: 'surface.muted',
    fontFamily: 'mono',
    fontSize: '10px',
    letterSpacing: '0',
    fontWeight: 'semibold',
    color: 'text.faint',
  });

  const amountClass = css({ marginLeft: 'auto', fontFamily: 'mono', letterSpacing: '0' });
</script>

<Modal style={css.raw({ padding: '20px', width: '630px' })} bind:open>
  <div class={css({ marginBottom: '14px' })}>
    <div class={flex({ align: 'center', gap: '8px' })}>
      <h2 class={css({ fontSize: '14px', fontWeight: 'semibold' })}>이 리뷰의 정보</h2>
      <span class={tierBadgeClass}>{tier}</span>
    </div>
    {#if workflowId}
      <!-- 원장·CF 포렌식·typie 조회의 열쇠들이라 id마다 통짜 선택되게 둔다. -->
      <div
        class={flex({
          align: 'center',
          gap: '6px',
          marginTop: '4px',
          fontFamily: 'mono',
          letterSpacing: '0',
          fontSize: '11px',
          color: 'text.faint',
        })}
      >
        <span class={css({ userSelect: 'all' })}>{workflowId}</span>
        <span>·</span>
        <span class={css({ userSelect: 'all' })}>{refId}</span>
      </div>
    {/if}
  </div>

  {#if modelConfig}
    <div class={flex({ direction: 'column', gap: '6px' })}>
      <!-- 스냅샷 자체를 순회한다 — 티어 목록으로 거르면 구 이름으로 저장된 옛 리뷰가 빈 표가 된다. 신규 행은
           buildModelConfig가 카탈로그의 워크플로 구동 목록 순서로 넣으므로 표시 순서는 같다. -->
      {#each Object.entries(modelConfig) as [agent, entry] (agent)}
        {#if entry}
          <div class={flex({ align: 'center', gap: '8px', fontSize: '12px' })}>
            <span class={css({ width: '150px', flexShrink: '0', fontFamily: 'mono', letterSpacing: '0', color: 'text.subtle' })}>
              {agent}
            </span>
            <span class={css({ fontFamily: 'mono', letterSpacing: '0', fontWeight: entry.overridden ? 'semibold' : 'normal' })}>
              {entry.model} · {entry.effort}
            </span>
            {#if entry.overridden}
              <span class={css({ fontSize: '11px', color: 'text.brand' })}>변경됨</span>
            {/if}
            {#if costs !== null}
              <span class={amountClass}>{amount(costs.agents[agent])}</span>
            {/if}
          </div>
        {/if}
      {/each}
      {#if costs !== null && costs.etc !== undefined}
        <div class={flex({ align: 'center', gap: '8px', fontSize: '12px' })}>
          <span class={css({ color: 'text.subtle' })}>기타</span>
          <span class={amountClass}>{amount(costs.etc)}</span>
        </div>
      {/if}
    </div>
    {#if costs !== null}
      <div
        class={flex({
          align: 'center',
          justify: 'space-between',
          marginTop: '10px',
          paddingTop: '10px',
          borderTopWidth: '1px',
          borderColor: 'border.default',
          fontSize: '13px',
          fontWeight: 'semibold',
        })}
      >
        <span>합계</span>
        <span class={css({ fontFamily: 'mono', letterSpacing: '0' })}>{amount(costs.total)}</span>
      </div>
    {/if}
    <div class={flex({ direction: 'column', gap: '4px', marginTop: '10px', fontSize: '11px', color: 'text.faint' })}>
      {#if costs !== null && accrualNote !== null}
        <p>{accrualNote}</p>
      {/if}
      <p>기본값 표시는 리뷰 시작 시점 기준이에요.</p>
    </div>
  {:else}
    <p class={css({ fontSize: '12px', color: 'text.faint' })}>구성 기록이 없어요 — 이 기능이 생기기 전에 시작된 리뷰예요.</p>
  {/if}
</Modal>
