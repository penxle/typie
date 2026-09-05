<script lang="ts">
  import { createFragment, createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, ProgressBar, ProgressRing } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import TargetIcon from '~icons/lucide/target';
  import { dueStatus, goalColorState, pickGoalSource } from '$lib/goal';
  import { dailyGoalStatus } from '$lib/user-stats';
  import { graphql } from '$mearie';
  import { getDayClock } from '../../day-clock.svelte';
  import Widget from '../Widget.svelte';
  import { getWidgetContext } from '../widget-context.svelte';

  type Props = {
    widgetId: string;
    data?: Record<string, unknown>;
  };

  let { widgetId, data = {} }: Props = $props();

  const app = getAppContext();
  const dayClock = getDayClock();
  const widgetContext = getWidgetContext();
  const { document$key, editor } = $derived(widgetContext.env);
  let isCollapsed = $state((data.isCollapsed as boolean) ?? false);

  const toggleCollapse = () => {
    isCollapsed = !isCollapsed;
    widgetContext.updateWidget?.(widgetId, { ...data, isCollapsed });
  };

  const document = createFragment(
    graphql(`
      fragment Editor_Widget_GoalWidget_document on Document {
        id
        characterCount

        entity {
          id

          goal {
            id
            targetCharacterCount
            dueAt
            createdAt
          }

          ancestors {
            id

            goal {
              id
              targetCharacterCount
              dueAt
              createdAt
            }

            node {
              __typename
              ... on Folder {
                id
                characterCount
              }
            }
          }
        }
      }
    `),
    () => document$key,
  );

  const meQuery = createQuery(
    graphql(`
      query Editor_Widget_GoalWidget_Query {
        me @required {
          id

          goal {
            id
            targetCharacterCount
          }

          goalHistory {
            date
            targetCharacterCount
            additions
            achieved
          }

          todayCharacterCountChange {
            date
            additions
          }
        }
      }
    `),
  );

  $effect(() => {
    if (!editor) return;
    void editor.characterCountsVersion;
    editor.updateCharacterCounts();
  });

  const localCount = $derived(editor?.characterCounts.docWithWhitespace);

  const entityGoal = $derived.by(() => {
    const doc = document.data;
    if (!doc) return null;

    return pickGoalSource(doc.entity, localCount ?? doc.characterCount);
  });

  const userGoal = $derived.by(() => {
    const me = meQuery.data?.me;
    if (!me?.goal) return null;
    return {
      target: me.goal.targetCharacterCount,
      ...dailyGoalStatus(me.goalHistory, me.goal.targetCharacterCount, me.todayCharacterCountChange, dayClock.now),
    };
  });

  const collapsedSummary = $derived.by(() => {
    if (entityGoal) return Math.round((entityGoal.current / entityGoal.goal.targetCharacterCount) * 100);
    if (userGoal) return Math.round((userGoal.additions / userGoal.target) * 100);
    return null;
  });
</script>

<Widget collapsed={isCollapsed} icon={TargetIcon} title="목표">
  {#snippet headerActions()}
    <button
      class={cx(
        'group',
        flex({
          alignItems: 'center',
          height: '26px',
          borderRadius: '6px',
          paddingX: '6px',
          gap: '2px',
          color: 'text.muted',
          cursor: 'pointer',
          _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
        }),
      )}
      onclick={toggleCollapse}
      type="button"
    >
      {#if isCollapsed && collapsedSummary !== null}
        <span class={css({ fontSize: '13px', fontWeight: 'normal' })}>
          {collapsedSummary}%
        </span>
      {/if}
      <Icon icon={isCollapsed ? ChevronDownIcon : ChevronUpIcon} size={14} />
    </button>
  {/snippet}

  <div class={flex({ flexDirection: 'column', gap: '10px' })}>
    {#if entityGoal}
      {@const target = entityGoal.goal.targetCharacterCount}
      {@const state = goalColorState(entityGoal.current, target)}
      {@const today = dayClock.now}
      {@const due = entityGoal.goal.dueAt ? dayjs(entityGoal.goal.dueAt).kst() : null}
      <div class={flex({ flexDirection: 'column', gap: '4px' })}>
        <div class={flex({ justifyContent: 'space-between', flexWrap: 'wrap', columnGap: '8px', rowGap: '2px', fontSize: '13px' })}>
          <span class={css({ color: 'text.muted', whiteSpace: 'nowrap' })}>{entityGoal.isFolder ? '폴더 목표' : '문서 목표'}</span>

          <div class={flex({ alignItems: 'center', justifyContent: 'flex-end', flexWrap: 'wrap', columnGap: '6px', marginLeft: 'auto' })}>
            <span class={css({ color: 'text.muted', whiteSpace: 'nowrap' })}>{comma(entityGoal.current)} / {comma(target)}자</span>

            {#if due}
              {@const status = dueStatus(entityGoal.current, target, due, today, 'compact')}

              {#if status}
                <span class={css(status.warning ? { color: 'danger.default' } : { color: 'text.hint' }, { whiteSpace: 'nowrap' })}>
                  {status.label}
                </span>
              {/if}
            {/if}
          </div>
        </div>
        <ProgressBar progress={entityGoal.current / target} {state} />
      </div>
    {/if}

    {#if userGoal}
      <button
        class={flex({ alignItems: 'center', gap: '8px', cursor: 'pointer' })}
        onclick={() => {
          app.state.userGoalOpen = true;
          mixpanel.track('open_user_goal_modal', { via: 'goal_widget' });
        }}
        type="button"
      >
        <ProgressRing progress={userGoal.additions / userGoal.target} size={20} state={userGoal.achieved ? 'achieved' : 'under'} />
        <span class={css({ fontSize: '13px', color: 'text.muted', whiteSpace: 'nowrap' })}>
          오늘 {comma(userGoal.additions)} / {comma(userGoal.target)}자
        </span>
      </button>
    {/if}

    {#if !entityGoal && !userGoal && !meQuery.loading}
      <span class={css({ fontSize: '13px', color: 'text.hint' })}>설정된 목표가 없어요</span>
    {/if}
  </div>
</Widget>
