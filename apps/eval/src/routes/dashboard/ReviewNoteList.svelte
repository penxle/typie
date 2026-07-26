<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import AxisMark from './AxisMark.svelte';
  import type { ReviewNote } from '$lib/domain/analysis-summary.ts';

  // 총평은 작품 하나에 대한 판단이라 피드백보다 수가 적다 — 접지 않고 그대로 편다.
  type Props = { notes: ReviewNote[] };
  const { notes }: Props = $props();

  const linkClass = css({
    fontSize: '11px',
    color: 'text.faint',
    textDecoration: 'underline',
    textUnderlineOffset: '[2px]',
    _hover: { color: 'text.default' },
  });
</script>

<div class={flex({ align: 'baseline', gap: '8px' })}>
  <h3 class={css({ fontSize: '13px', fontWeight: 'bold' })}>작품 총평</h3>
  <span class={css({ fontSize: '12px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>{notes.length}건</span>
</div>

{#if notes.length === 0}
  <p class={css({ marginTop: '10px', fontSize: '13px', color: 'text.faint' })}>아직 총평에 남긴 말이 없습니다.</p>
{:else}
  <div class={flex({ direction: 'column', marginTop: '10px' })}>
    {#each notes as note (note.setId + note.evaluator)}
      <article
        class={css({
          paddingY: '12px',
          borderBottomWidth: '1px',
          borderColor: 'border.subtle',
          ['&:last-child']: { borderBottomWidth: '0' },
        })}
      >
        <div class={flex({ align: 'center', gap: '8px', flexWrap: 'wrap' })}>
          {#if note.taskId}
            <a class={`${linkClass} ${css({ fontSize: '12px', color: 'text.subtle' })}`} href={`/admin/tasks/${note.taskId}`}>
              {note.refId}
            </a>
          {:else}
            <span class={css({ fontSize: '12px', color: 'text.subtle', fontVariantNumeric: 'tabular-nums' })}>{note.refId}</span>
          {/if}
          <span class={flex({ align: 'center', gap: '10px', marginLeft: 'auto' })}>
            <span class={css({ fontSize: '11px', color: 'text.faint' })}>{note.evaluator}</span>
            <AxisMark
              slots={[
                { label: '파악', failed: note.failed.readCorrectly },
                { label: '순서', failed: note.failed.priorityUseful },
              ]}
            />
          </span>
        </div>

        {#if note.note}
          <p class={css({ marginTop: '5px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' })}>{note.note}</p>
        {/if}

        {#if note.comment}
          <!-- 판정 전체에 남긴 말은 총평 사유와 다른 층위다 — 라벨로 구분해 섞이지 않게 한다. -->
          <p class={css({ marginTop: note.note ? '6px' : '5px', fontSize: '11px', color: 'text.faint' })}>이 글 전체에 대해</p>
          <p class={css({ marginTop: '1px', fontSize: '13px', lineHeight: '[1.75]', whiteSpace: 'pre-wrap' })}>{note.comment}</p>
        {/if}
      </article>
    {/each}
  </div>
{/if}
