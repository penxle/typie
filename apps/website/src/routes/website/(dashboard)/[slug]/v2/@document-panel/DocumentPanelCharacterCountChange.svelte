<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import { fly } from 'svelte/transition';
  import IconChevronRight from '~icons/lucide/chevron-right';
  import GoalIcon from '~icons/lucide/goal';
  import TrendingDownIcon from '~icons/lucide/trending-down';
  import TrendingUpIcon from '~icons/lucide/trending-up';
  import { graphql } from '$mearie';
  import type { DocumentPanelV2_Info_CharacterCountChange_document$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelV2_Info_CharacterCountChange_document$key;
  };

  let { document$key }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DocumentPanelV2_Info_CharacterCountChange_document on Document {
        id

        characterCountChange {
          additions
          deletions
        }
      }
    `),
    () => document$key,
  );

  let open = $state(false);
  const difference = $derived(document.data.characterCountChange.additions - document.data.characterCountChange.deletions);
</script>

<details class={flex({ flexDirection: 'column', marginBottom: open ? '12px' : '0' })} bind:open>
  <summary class={flex({ alignItems: 'center', gap: '4px', cursor: 'pointer', marginBottom: open ? '8px' : '0', userSelect: 'none' })}>
    <Icon style={css.raw({ color: 'text.faint' })} icon={GoalIcon} size={12} />
    <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.subtle' })}>오늘의 기록</div>
    <Icon style={css.raw({ color: 'text.faint', transform: open ? 'rotate(90deg)' : 'rotate(0deg)' })} icon={IconChevronRight} size={14} />
    <div class={css({ flexGrow: '1' })}></div>
    {#if !open}
      <div
        class={flex({ alignItems: 'center', gap: '4px', fontSize: '13px', fontWeight: 'medium', color: 'text.subtle' })}
        in:fly={{ y: 2, duration: 150 }}
      >
        {#if difference === 0}
          없음
        {:else}
          <Icon style={css.raw({ color: 'text.faint' })} icon={difference >= 0 ? TrendingUpIcon : TrendingDownIcon} size={14} />
          <span>{difference >= 0 ? '+' : '-'}{comma(Math.abs(difference))}자</span>
        {/if}
      </div>
    {/if}
  </summary>

  {#if open}
    <div class={flex({ flexDirection: 'column', gap: '2px' })} in:fly={{ y: -2, duration: 150 }}>
      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>변화량</dt>
        <dd class={flex({ alignItems: 'center', gap: '4px', fontWeight: 'medium', color: 'text.subtle' })}>
          {#if difference === 0}
            없음
          {:else}
            <Icon style={css.raw({ color: 'text.faint' })} icon={difference >= 0 ? TrendingUpIcon : TrendingDownIcon} size={14} />
            <span>{difference >= 0 ? '+' : '-'}{comma(Math.abs(difference))}자</span>
          {/if}
        </dd>
      </dl>

      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>입력한 글자</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>{comma(document.data.characterCountChange.additions)}자</dd>
      </dl>

      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>지운 글자</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>{comma(document.data.characterCountChange.deletions)}자</dd>
      </dl>
    </div>
  {/if}
</details>
