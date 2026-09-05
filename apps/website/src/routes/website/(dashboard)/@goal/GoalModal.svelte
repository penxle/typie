<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Calendar, Modal, Popover, ProgressRing, TextInput } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { untrack } from 'svelte';
  import { dDayLabel, goalColorState, requiredToday, timeFraction } from '$lib/goal';
  import { formatCommaInput, parseCommaInput } from '$lib/number-input';
  import { graphql } from '$mearie';
  import { getDayClock } from '../day-clock.svelte';
  import GoalHistoryTable from './GoalHistoryTable.svelte';
  import GoalTrendChart from './GoalTrendChart.svelte';

  const app = getAppContext();
  const dayClock = getDayClock();

  const query = createQuery(
    graphql(`
      query DashboardLayout_GoalModal_Query($entityIds: [ID!]!) {
        entities(entityIds: $entityIds) {
          id

          goal {
            id
            targetCharacterCount
            dueAt
            createdAt
          }

          node {
            __typename
            ... on Document {
              id
              title
              characterCount
            }
            ... on Folder {
              id
              name
              characterCount
            }
          }

          ...DashboardLayout_GoalTrendChart_entity
          ...DashboardLayout_GoalHistoryTable_entity
        }
      }
    `),
    () => ({ entityIds: app.state.goalOpen }),
    () => ({ skip: app.state.goalOpen.length === 0 }),
  );

  const entity = $derived(query.data?.entities[0]);
  const goal = $derived(entity?.goal);
  const node = $derived(entity?.node);
  const currentCount = $derived(node?.__typename === 'Document' || node?.__typename === 'Folder' ? node.characterCount : 0);
  const targetName = $derived(node?.__typename === 'Document' ? node.title : node?.__typename === 'Folder' ? node.name : '');
  const loaded = $derived(app.state.goalOpen.length > 0 && !!query.data);

  let targetInput = $state('');
  let dueDate = $state<Date | undefined>();
  let seededEntityId = $state<string>();
  let editing = $state(false);
  let tab = $state<'trend' | 'history'>('trend');

  const seedForm = () => {
    targetInput = goal ? comma(goal.targetCharacterCount) : '';
    dueDate = goal?.dueAt ? dayjs(dayjs(goal.dueAt).kst().format('YYYY-MM-DD')).toDate() : undefined;
  };

  $effect(() => {
    if (app.state.goalOpen.length === 0) {
      seededEntityId = undefined;
      return;
    }

    if (!loaded || !entity || untrack(() => seededEntityId) === entity.id) {
      return;
    }

    seededEntityId = entity.id;
    editing = false;
    tab = 'trend';
    seedForm();
  });

  const [updateEntityGoal] = createMutation(
    graphql(`
      mutation DashboardLayout_GoalModal_UpdateEntityGoal_Mutation($input: UpdateEntityGoalInput!) {
        updateEntityGoal(input: $input) {
          id
          goal {
            id
            targetCharacterCount
            dueAt
            createdAt
          }
        }
      }
    `),
  );

  const [deleteEntityGoal] = createMutation(
    graphql(`
      mutation DashboardLayout_GoalModal_DeleteEntityGoal_Mutation($input: DeleteEntityGoalInput!) {
        deleteEntityGoal(input: $input) {
          id
          goal {
            id
            targetCharacterCount
            dueAt
            createdAt
          }
        }
      }
    `),
  );

  const save = async () => {
    if (!entity) {
      return;
    }

    const target = parseCommaInput(targetInput);
    if (!Number.isSafeInteger(target) || target <= 0) {
      Toast.error('목표 글자 수를 올바르게 입력해 주세요.');
      return;
    }

    await updateEntityGoal({
      input: {
        entityId: entity.id,
        targetCharacterCount: target,
        dueAt: dueDate ? dayjs.kst(dayjs(dueDate).format('YYYY-MM-DD')).format() : undefined,
      },
    });

    editing = false;
    Toast.success('목표를 저장했어요.');
  };

  const cancel = () => {
    seedForm();
    editing = false;
  };

  const remove = () => {
    const entityId = entity?.id;
    if (!entityId) {
      return;
    }

    Dialog.confirm({
      title: '목표를 삭제하시겠어요?',
      message: '설정한 목표 글자 수와 마감일이 사라져요.',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        await deleteEntityGoal({ input: { entityId } });
        targetInput = '';
        dueDate = undefined;
        Toast.success('목표를 삭제했어요.');
      },
    });
  };

  const percentColorByState = {
    under: css.raw({ color: 'text.default' }),
    achieved: css.raw({ color: 'success.default' }),
    over: css.raw({ color: 'warning.default' }),
    excess: css.raw({ color: 'danger.default' }),
  };

  const fieldStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    width: 'full',
    height: '32px',
    paddingX: '12px',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '4px',
    backgroundColor: 'surface.default',
    transition: 'common',
    cursor: 'pointer',
    _hover: { borderColor: 'border.emphasis' },
    _focusVisible: { borderColor: 'accent.default' },
  });
</script>

{#snippet goalForm()}
  <div class={flex({ flexDirection: 'column', gap: '8px', width: 'full' })}>
    {#if !goal}
      <div
        class={css({
          marginBottom: '4px',
          padding: '12px',
          borderRadius: '8px',
          backgroundColor: 'surface.canvas',
          fontSize: '13px',
          lineHeight: '[1.6]',
          color: 'text.muted',
        })}
      >
        완성까지 쓸 글자 수를 목표로 정해 보세요. 마감일을 함께 정하면 오늘 써야 할 분량도 알려드려요.
      </div>
    {/if}

    <div class={flex({ alignItems: 'center', gap: '6px' })}>
      <TextInput
        style={css.raw({ flex: '1', minWidth: '0' })}
        inputmode="numeric"
        oninput={(e) => (targetInput = formatCommaInput(e))}
        placeholder="목표 글자 수"
        size="sm"
        value={targetInput}
      />
      <span class={css({ fontSize: '13px', color: 'text.muted' })}>자</span>
    </div>

    <Popover style={fieldStyle} placement="bottom-start">
      {#snippet trigger()}
        <span class={css({ fontSize: '13px', color: dueDate ? 'text.default' : 'text.hint' })}>
          {dueDate ? dayjs(dueDate).format('YYYY. M. D. 마감') : '마감일 없음 (선택)'}
        </span>
      {/snippet}

      {#snippet children({ close })}
        <Calendar
          onchange={(date) => {
            dueDate = date;
            close();
          }}
          value={dueDate}
        />

        {#if dueDate}
          <Button
            onclick={() => {
              dueDate = undefined;
              close();
            }}
            size="sm"
            variant="secondary"
          >
            마감일 제거
          </Button>
        {/if}
      {/snippet}
    </Popover>

    <div class={flex({ gap: '6px' })}>
      <Button style={css.raw({ flex: '1' })} onclick={save} size="sm">저장</Button>
      {#if goal}
        <Button style={css.raw({ flex: '1' })} onclick={cancel} size="sm" variant="secondary">취소</Button>
      {/if}
    </div>
  </div>
{/snippet}

{#snippet tabButton(id: 'trend' | 'history', label: string)}
  <button
    class={css({
      marginBottom: '-1px',
      paddingBottom: '6px',
      fontSize: '13px',
      fontWeight: tab === id ? 'semibold' : 'medium',
      color: tab === id ? 'text.default' : 'text.muted',
      borderBottomWidth: '2px',
      borderColor: tab === id ? 'accent.default' : 'transparent',
      transition: 'common',
      cursor: 'pointer',
    })}
    aria-selected={tab === id}
    onclick={() => (tab = id)}
    role="tab"
    type="button"
  >
    {label}
  </button>
{/snippet}

<Modal
  style={css.raw({ gap: '20px', maxWidth: '640px', padding: '24px' })}
  loading={!loaded}
  onclose={() => {
    app.state.goalOpen = [];
  }}
  open={app.state.goalOpen.length > 0}
>
  <div class={flex({ alignItems: 'center', gap: '8px' })}>
    <div class={css({ fontSize: '17px', fontWeight: 'semibold', color: 'text.default' })}>목표</div>
    {#if targetName}
      <div class={css({ fontSize: '13px', color: 'text.muted', minWidth: '0', truncate: true })}>{targetName}</div>
    {/if}
  </div>

  {#if entity}
    <div class={flex({ alignItems: 'stretch' })}>
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '12px',
          width: '200px',
          minHeight: '304px',
          flexShrink: '0',
          paddingRight: '24px',
          borderRightWidth: '1px',
          borderColor: 'border.hairline',
        })}
      >
        {#if goal && !editing}
          {@const state = goalColorState(currentCount, goal.targetCharacterCount)}
          {@const percent = Math.floor((currentCount / goal.targetCharacterCount) * 100)}
          {@const today = dayClock.now}
          {@const due = goal.dueAt ? dayjs(goal.dueAt).kst() : null}
          {@const overdue = !!due && due.isBefore(today, 'day')}
          {@const overdueUnder = overdue && state === 'under'}
          {@const pie = due && state === 'under' ? timeFraction(dayjs(goal.createdAt).kst(), due, today) : null}
          {@const required = due ? requiredToday(currentCount, goal.targetCharacterCount, due, today) : 0}

          <ProgressRing {pie} pieWarning={overdueUnder} progress={currentCount / goal.targetCharacterCount} size={96} {state} />

          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <div class={css({ fontSize: '32px', fontWeight: 'bold', fontVariantNumeric: 'tabular-nums' }, percentColorByState[state])}>
              {percent}%
            </div>

            <div class={css({ fontSize: '13px', color: 'text.muted', fontVariantNumeric: 'tabular-nums' })}>
              {comma(currentCount)} / {comma(goal.targetCharacterCount)}자
            </div>

            {#if state === 'under' && !overdue}
              <div class={css({ fontSize: '12px', color: 'text.hint' })}>
                {comma(goal.targetCharacterCount - currentCount)}자 남음
              </div>
            {:else if state === 'over' || state === 'excess'}
              <div class={css({ fontSize: '12px', color: state === 'excess' ? 'danger.default' : 'text.hint' })}>
                목표보다 {comma(currentCount - goal.targetCharacterCount)}자 초과
              </div>
            {/if}
          </div>

          {#if due && (state === 'under' || !overdue)}
            <div
              class={flex({
                flexDirection: 'column',
                alignItems: 'center',
                gap: '2px',
                width: 'full',
                paddingX: '12px',
                paddingY: '8px',
                borderRadius: '8px',
                backgroundColor: 'surface.canvas',
              })}
            >
              <span class={css({ fontSize: '14px', fontWeight: 'semibold', color: overdueUnder ? 'danger.default' : 'text.default' })}>
                {dDayLabel(due, today)}
              </span>

              {#if required > 0}
                <span class={css({ fontSize: '12px', color: 'text.hint' })}>
                  {overdueUnder ? `${comma(required)}자 남음` : `오늘 ${comma(required)}자 필요`}
                </span>
              {/if}
            </div>
          {/if}

          <div class={flex({ gap: '6px', width: 'full' })}>
            <Button style={css.raw({ flex: '1' })} onclick={() => (editing = true)} size="sm" variant="secondary">수정</Button>
            <Button style={css.raw({ flex: '1', color: 'danger.default' })} onclick={remove} size="sm" variant="secondary">삭제</Button>
          </div>
        {:else}
          {@render goalForm()}
        {/if}
      </div>

      <div class={flex({ flexDirection: 'column', flex: '1', minWidth: '0', gap: '12px', paddingLeft: '24px' })}>
        <div class={flex({ gap: '16px', borderBottomWidth: '1px', borderColor: 'border.hairline' })} role="tablist">
          {@render tabButton('trend', '추세')}
          {@render tabButton('history', '기록')}
        </div>

        <div class={css({ position: 'relative', flex: '1', minHeight: '0' })}>
          <div class={css({ position: 'absolute', inset: '0' })}>
            {#if tab === 'trend'}
              <GoalTrendChart current={currentCount} entity$key={entity} />
            {:else}
              <GoalHistoryTable entity$key={entity} />
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}
</Modal>
