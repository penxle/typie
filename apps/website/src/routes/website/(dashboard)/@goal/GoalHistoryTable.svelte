<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { graphql } from '$mearie';
  import type { DashboardLayout_GoalHistoryTable_entity$key } from '$mearie';

  type Props = {
    entity$key: DashboardLayout_GoalHistoryTable_entity$key;
  };

  let { entity$key }: Props = $props();

  const entity = createFragment(
    graphql(`
      fragment DashboardLayout_GoalHistoryTable_entity on Entity {
        id

        characterCountHistory {
          date
          characterCount
        }
      }
    `),
    () => entity$key,
  );

  const history = $derived(entity.data.characterCountHistory);

  const recentRows = $derived.by(() => {
    const rows = history
      .slice(-15)
      .toReversed()
      .map((p, i, arr) => ({ ...p, diff: i < arr.length - 1 ? p.characterCount - arr[i + 1].characterCount : null }));

    return history.length > 1 ? rows.slice(0, -1) : rows;
  });
</script>

{#if recentRows.length > 0}
  <div class={flex({ flexDirection: 'column', height: 'full' })}>
    <div class={flex({ paddingY: '4px', fontSize: '11px', color: 'text.muted', borderBottomWidth: '1px', borderColor: 'border.hairline' })}>
      <span class={css({ flex: '1' })}>날짜</span>
      <span class={css({ flex: '1', textAlign: 'center' })}>글자 수</span>
      <span class={css({ flex: '1', textAlign: 'right' })}>증감</span>
    </div>

    <div class={flex({ flexDirection: 'column', flex: '1', minHeight: '0', overflowY: 'auto' })}>
      {#each recentRows as row (row.date)}
        <div
          class={flex({
            paddingY: '4px',
            fontSize: '12px',
            fontVariantNumeric: 'tabular-nums',
            borderBottomWidth: '1px',
            borderColor: 'border.hairline',
          })}
        >
          <span class={css({ flex: '1', color: 'text.hint' })}>{dayjs(row.date).kst().format('M월 D일')}</span>
          <span class={css({ flex: '1', textAlign: 'center', color: 'text.muted' })}>{comma(row.characterCount)}자</span>
          <span class={css({ flex: '1', textAlign: 'right', color: row.diff !== null && row.diff < 0 ? 'danger.default' : 'text.hint' })}>
            {row.diff === null ? '—' : `${row.diff >= 0 ? '+' : ''}${comma(row.diff)}`}
          </span>
        </div>
      {/each}
    </div>
  </div>
{:else}
  <div class={css({ paddingY: '24px', textAlign: 'center', fontSize: '13px', color: 'text.hint' })}>
    아직 기록이 없어요. 글을 쓰면 하루하루의 글자 수가 쌓여요.
  </div>
{/if}
