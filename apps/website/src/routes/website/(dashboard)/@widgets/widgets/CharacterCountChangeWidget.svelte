<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import GoalIcon from '~icons/lucide/goal';
  import TrendingDownIcon from '~icons/lucide/trending-down';
  import TrendingUpIcon from '~icons/lucide/trending-up';
  import { fragment, graphql } from '$graphql';
  import Widget from '../Widget.svelte';
  import { getWidgetContext } from '../widget-context.svelte';

  type Props = {
    widgetId: string;
    data?: Record<string, unknown>;
  };

  let { widgetId, data = {} }: Props = $props();

  const widgetContext = getWidgetContext();
  const { $post: _post } = $derived(widgetContext.env);
  let isCollapsed = $state((data.isCollapsed as boolean) ?? false);

  const toggleCollapse = () => {
    isCollapsed = !isCollapsed;
    widgetContext.updateWidget?.(widgetId, { ...data, isCollapsed });
  };

  const post = fragment(
    // eslint-disable-next-line svelte/no-unused-svelte-ignore
    // svelte-ignore state_referenced_locally
    _post,
    graphql(`
      fragment Editor_Widget_CharacterCountChangeWidget_post on Post {
        id

        characterCountChange {
          additions
          deletions
        }
      }
    `),
  );

  const difference = $derived($post ? $post.characterCountChange.additions - $post.characterCountChange.deletions : 0);
  const additions = $derived($post ? $post.characterCountChange.additions : 0);
  const deletions = $derived($post ? $post.characterCountChange.deletions : 0);
</script>

<Widget collapsed={isCollapsed} icon={GoalIcon} title="오늘의 기록">
  {#snippet headerActions()}
    <button
      class={flex({ alignItems: 'center', gap: '2px', color: 'text.subtle', cursor: 'pointer' })}
      onclick={toggleCollapse}
      type="button"
    >
      {#if isCollapsed}
        <span class={css({ fontSize: '13px', fontWeight: 'normal', color: 'text.subtle' })}>
          {#if difference === 0}
            없음
          {:else}
            {difference >= 0 ? '+' : ''}{comma(difference)}자
          {/if}
        </span>
      {/if}
      <Icon icon={isCollapsed ? ChevronDownIcon : ChevronUpIcon} size={14} />
    </button>
  {/snippet}

  <div class={flex({ flexDirection: 'column', gap: '8px' })}>
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
      <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>{comma(additions ?? 0)}자</dd>
    </dl>

    <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
      <dt class={css({ color: 'text.faint' })}>지운 글자</dt>
      <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>{comma(deletions ?? 0)}자</dd>
    </dl>
  </div>
</Widget>
