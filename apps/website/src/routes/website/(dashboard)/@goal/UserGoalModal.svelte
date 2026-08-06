<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal, ProgressRing, TextInput } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { untrack } from 'svelte';
  import { streaks, todayProgress } from '$lib/goal';
  import { formatCommaInput, parseCommaInput } from '$lib/number-input';
  import { graphql } from '$mearie';
  import UserGoalDots from './UserGoalDots.svelte';
  import UserGoalHistoryTable from './UserGoalHistoryTable.svelte';
  import UserGoalTrendChart from './UserGoalTrendChart.svelte';

  const app = getAppContext();

  const query = createQuery(
    graphql(`
      query DashboardLayout_UserGoalModal_Query {
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

          ...DashboardLayout_UserGoalDots_user
          ...DashboardLayout_UserGoalTrendChart_user
          ...DashboardLayout_UserGoalHistoryTable_user
        }
      }
    `),
    undefined,
    () => ({ skip: !app.state.userGoalOpen }),
  );

  const me = $derived(query.data?.me);
  const goal = $derived(me?.goal);
  const loaded = $derived(app.state.userGoalOpen && !!query.data);

  let input = $state('');
  let editing = $state(false);
  let seeded = $state(false);

  const seedForm = () => {
    input = goal ? comma(goal.targetCharacterCount) : '';
  };

  $effect(() => {
    if (!app.state.userGoalOpen) {
      seeded = false;
      return;
    }

    if (!loaded || untrack(() => seeded)) {
      return;
    }

    seeded = true;
    editing = false;
    seedForm();
  });

  const [updateUserGoal] = createMutation(
    graphql(`
      mutation DashboardLayout_UserGoalModal_UpdateUserGoal_Mutation($input: UpdateUserGoalInput!) {
        updateUserGoal(input: $input) {
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
        }
      }
    `),
  );

  const [deleteUserGoal] = createMutation(
    graphql(`
      mutation DashboardLayout_UserGoalModal_DeleteUserGoal_Mutation {
        deleteUserGoal {
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
        }
      }
    `),
  );

  const save = async () => {
    const target = parseCommaInput(input);
    if (!Number.isSafeInteger(target) || target <= 0) {
      Toast.error('목표 글자 수를 올바르게 입력해 주세요.');
      return;
    }

    await updateUserGoal({ input: { targetCharacterCount: target } });

    editing = false;
    Toast.success('일일 목표를 저장했어요.');
  };

  const cancel = () => {
    seedForm();
    editing = false;
  };

  const remove = () => {
    Dialog.confirm({
      title: '일일 목표를 해제하시겠어요?',
      message: '설정한 하루 목표 글자 수가 사라져요.',
      action: 'danger',
      actionLabel: '해제',
      actionHandler: async () => {
        await deleteUserGoal();
        input = '';
        Toast.success('일일 목표를 해제했어요.');
      },
    });
  };
</script>

{#snippet goalForm()}
  <div class={flex({ flexDirection: 'column', gap: '8px', width: 'full' })}>
    {#if !goal}
      <div
        class={css({
          marginBottom: '4px',
          padding: '12px',
          borderRadius: '8px',
          backgroundColor: 'surface.muted',
          fontSize: '13px',
          lineHeight: '[1.6]',
          color: 'text.subtle',
        })}
      >
        매일 쓸 글자 수를 정해 보세요. 달성한 날이 기록으로 쌓이고, 연속 달성 일수도 볼 수 있어요.
      </div>
    {/if}

    <div class={flex({ alignItems: 'center', gap: '6px' })}>
      <TextInput
        style={css.raw({ flex: '1', minWidth: '0' })}
        inputmode="numeric"
        oninput={(e) => (input = formatCommaInput(e))}
        placeholder="하루 글자 수"
        size="sm"
        value={input}
      />
      <span class={css({ fontSize: '13px', color: 'text.faint' })}>자</span>
    </div>

    <div class={flex({ gap: '6px' })}>
      <Button style={css.raw({ flex: '1' })} onclick={save} size="sm">저장</Button>
      {#if goal}
        <Button style={css.raw({ flex: '1' })} onclick={cancel} size="sm" variant="secondary">취소</Button>
      {/if}
    </div>
  </div>
{/snippet}

<Modal
  style={css.raw({ gap: '20px', maxWidth: '640px', padding: '24px' })}
  loading={!loaded}
  onclose={() => {
    app.state.userGoalOpen = false;
  }}
  open={app.state.userGoalOpen}
>
  <div class={css({ fontSize: '17px', fontWeight: 'semibold', color: 'text.default' })}>일일 목표</div>

  {#if me}
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
          borderColor: 'border.subtle',
        })}
      >
        {#if goal && !editing}
          {@const progress = todayProgress(me.goalHistory, dayjs.kst())}
          {@const streak = streaks(me.goalHistory, dayjs.kst())}

          <ProgressRing
            progress={progress.additions / goal.targetCharacterCount}
            size={96}
            state={progress.achieved ? 'achieved' : 'under'}
          />

          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <div
              class={css({
                fontSize: '32px',
                fontWeight: 'bold',
                fontVariantNumeric: 'tabular-nums',
                color: progress.achieved ? 'accent.success.default' : 'text.default',
              })}
            >
              {comma(progress.additions)}
            </div>

            <div class={css({ fontSize: '13px', color: 'text.subtle', fontVariantNumeric: 'tabular-nums' })}>
              / {comma(goal.targetCharacterCount)}자
            </div>
          </div>

          <div
            class={flex({
              flexDirection: 'column',
              alignItems: 'center',
              gap: '2px',
              width: 'full',
              paddingX: '12px',
              paddingY: '8px',
              borderRadius: '8px',
              backgroundColor: 'surface.muted',
            })}
          >
            <span class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default' })}>달성 연속 {streak.current}일</span>
            <span class={css({ fontSize: '12px', color: 'text.faint' })}>최고 기록 {streak.best}일</span>
          </div>

          <div class={flex({ gap: '6px', width: 'full' })}>
            <Button style={css.raw({ flex: '1' })} onclick={() => (editing = true)} size="sm" variant="secondary">수정</Button>
            <Button style={css.raw({ flex: '1', color: 'text.danger' })} onclick={remove} size="sm" variant="secondary">해제</Button>
          </div>
        {:else}
          {@render goalForm()}
        {/if}
      </div>

      <div class={flex({ flexDirection: 'column', flex: '1', minWidth: '0', paddingLeft: '24px' })}>
        <div class={css({ position: 'relative', flex: '1', minHeight: '0' })}>
          <div class={flex({ position: 'absolute', inset: '0', flexDirection: 'column', gap: '16px', overflowY: 'auto' })}>
            <div class={flex({ flexDirection: 'column', gap: '8px' })}>
              <div class={css({ fontSize: '11px', color: 'text.faint' })}>달성 · 최근 16주</div>
              <UserGoalDots user$key={me} />
            </div>

            <div class={flex({ flexDirection: 'column', gap: '8px' })}>
              <div class={css({ fontSize: '11px', color: 'text.faint' })}>일별 글자 수 · 최근 4주</div>
              <UserGoalTrendChart user$key={me} />
            </div>

            <div class={flex({ flexDirection: 'column', gap: '8px' })}>
              <div class={css({ fontSize: '11px', color: 'text.faint' })}>일별 기록</div>
              <UserGoalHistoryTable user$key={me} />
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}
</Modal>
