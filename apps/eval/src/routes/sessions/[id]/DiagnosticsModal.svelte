<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Modal } from '@typie/ui/components';
  import type { FeedbackDiagnostics } from '$lib/feedback/types.ts';

  type Props = { open: boolean; diagnostics: FeedbackDiagnostics | null };
  let { open = $bindable(), diagnostics }: Props = $props();

  // 처분 이름은 판정 계약의 값이다 — 화면에는 그 값이 무엇을 뜻하는지로 세운다.
  const DISPOSITION: Record<FeedbackDiagnostics['withheld'][number]['disposition'], string> = {
    explained: '작품 근거로 해소',
    withheld: '발화 조건 미충족',
  };

  const empty = $derived(
    diagnostics !== null && diagnostics.dropped.length === 0 && diagnostics.gaps.length === 0 && diagnostics.withheld.length === 0,
  );
</script>

{#snippet heading(text: string)}
  <h3 class={css({ marginBottom: '6px', fontSize: '12px', fontWeight: 'semibold', color: 'text.subtle' })}>{text}</h3>
{/snippet}

<Modal style={css.raw({ padding: '20px', width: '420px' })} bind:open>
  <h2 class={css({ fontSize: '14px', fontWeight: 'semibold' })}>이 리뷰의 진단</h2>
  <p class={css({ marginTop: '4px', marginBottom: '14px', fontSize: '11px', color: 'text.faint' })}>
    판정이 보았지만 작가에게 가지 않은 것들이에요.
  </p>

  {#if diagnostics === null}
    <p class={css({ fontSize: '12px', color: 'text.faint' })}>진단 기록이 없어요 — 이 기록이 생기기 전에 끝난 리뷰예요.</p>
  {:else if empty}
    <p class={css({ fontSize: '12px', color: 'text.faint' })}>지운 지적도, 기준 밖 걸림도 없었어요.</p>
  {:else}
    <div class={flex({ direction: 'column', gap: '16px', maxHeight: '[420px]', overflowY: 'auto' })}>
      {#if diagnostics.dropped.length > 0}
        <section>
          {@render heading('지우고 다시 낸 지적')}
          <p class={css({ marginBottom: '8px', fontSize: '11px', color: 'text.faint' })}>
            반려된 뒤 다시 낼 때 사라진 지적이에요 — 고치는 대신 지워서 통과한 자리예요.
          </p>
          <div class={flex({ direction: 'column', gap: '4px' })}>
            {#each diagnostics.dropped as item (item.trait)}
              <div class={flex({ align: 'center', justify: 'space-between', gap: '10px', fontSize: '12px' })}>
                <span class={css({ minWidth: '0', color: 'text.subtle' })}>{item.trait}</span>
                <span class={css({ flex: 'none', fontFamily: 'mono', letterSpacing: '0' })}>{item.count}</span>
              </div>
            {/each}
          </div>
        </section>
      {/if}

      {#if diagnostics.gaps.length > 0}
        <section>
          {@render heading('기준표가 덮지 못한 자리')}
          <div class={flex({ direction: 'column', gap: '6px' })}>
            {#each diagnostics.gaps as gap, index (index)}
              <p class={css({ fontSize: '12px', lineHeight: '[1.6]', color: 'text.subtle' })}>{gap ?? '—'}</p>
            {/each}
          </div>
        </section>
      {/if}

      {#if diagnostics.withheld.length > 0}
        <section>
          {@render heading('지적으로 서지 않은 걸림')}
          <div class={flex({ direction: 'column', gap: '10px' })}>
            {#each diagnostics.withheld as item, index (index)}
              <div
                class={css({
                  paddingX: '10px',
                  paddingY: '8px',
                  borderWidth: '1px',
                  borderColor: 'border.default',
                  borderRadius: '6px',
                  backgroundColor: 'surface.muted',
                })}
              >
                <div class={flex({ align: 'center', gap: '6px', marginBottom: '6px' })}>
                  <span class={css({ flex: 'none', fontSize: '11px', fontWeight: 'semibold', color: 'text.faint' })}>
                    {DISPOSITION[item.disposition]}
                  </span>
                  <span
                    class={css({
                      minWidth: '0',
                      fontSize: '11px',
                      color: 'text.faint',
                      whiteSpace: 'nowrap',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                    })}
                  >
                    {item.head}
                  </span>
                </div>
                {#if item.reading}
                  <p class={css({ fontSize: '12px', lineHeight: '[1.6]', color: 'text.subtle' })}>{item.reading}</p>
                {/if}
                {#if item.note}
                  <p class={css({ marginTop: '4px', fontSize: '11px', lineHeight: '[1.6]', color: 'text.faint' })}>{item.note}</p>
                {/if}
              </div>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  {/if}
</Modal>
