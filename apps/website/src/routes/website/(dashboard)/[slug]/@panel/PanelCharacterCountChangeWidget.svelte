<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import { fly } from 'svelte/transition';
  import IconChevronRight from '~icons/lucide/chevron-right';
  import GoalIcon from '~icons/lucide/goal';
  import TrendingDownIcon from '~icons/lucide/trending-down';
  import TrendingUpIcon from '~icons/lucide/trending-up';
  import { fragment, graphql } from '$graphql';
  import type { Editor_Panel_PanelInfo_CharacterCountChangeWidget_post } from '$graphql';

  type Props = {
    $post: Editor_Panel_PanelInfo_CharacterCountChangeWidget_post;
  };

  let { $post: _post }: Props = $props();

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_Panel_PanelInfo_CharacterCountChangeWidget_post on Post {
        id

        characterCountChange {
          additions
          deletions
        }
      }
    `),
  );

  let open = $state(false);
  const difference = $derived($post.characterCountChange.additions - $post.characterCountChange.deletions);
</script>

<details class={flex({ flexDirection: 'column', marginBottom: open ? '12px' : '0' })} bind:open>
  <summary class={flex({ alignItems: 'center', gap: '4px', cursor: 'pointer', marginBottom: open ? '8px' : '0', userSelect: 'none' })}>
    <Icon style={{ color: 'text.faint' }} icon={GoalIcon} size={12} />
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
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>{comma($post.characterCountChange.additions)}자</dd>
      </dl>

      <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
        <dt class={css({ color: 'text.faint' })}>지운 글자</dt>
        <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>{comma($post.characterCountChange.deletions)}자</dd>
      </dl>
    </div>
  {/if}
</details>

<!-- 
<button
  class={flex({ alignItems: 'center', gap: '6px' })}
  onclick={() => {
    app.preference.current.characterCountChangeMode =
      app.preference.current.characterCountChangeMode === 'additions' ? 'difference' : 'additions';
  }}
  onmouseenter={() => (open = true)}
  onmouseleave={() => (open = false)}
  type="button"
  use:anchor
>
  <Icon style={{ color: 'text.faint' }} icon={IconTarget} size={14} />
</button> -->
