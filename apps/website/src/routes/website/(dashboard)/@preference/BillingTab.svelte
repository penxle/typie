<script lang="ts">
  import { cache } from '@typie/sark/internal';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { PlanPair } from '@/const';
  import { PlanInterval, SubscriptionState } from '@/enums';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import RedeemCreditCodeModal from './RedeemCreditCodeModal.svelte';
  import SubscriptionCancellationSurveyModal from './SubscriptionCancellationSurveyModal.svelte';
  import UpdatePaymentMethodModal from './UpdatePaymentMethodModal.svelte';
  import type { DashboardLayout_PreferenceModal_BillingTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_BillingTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_BillingTab_user on User {
        id
        credit
        ...DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user
        ...DashboardLayout_PreferenceModal_BillingTab_SubscriptionCancellationSurveyModal_user

        billingKey {
          id
          name
        }

        subscription {
          id
          state
          startsAt
          expiresAt

          plan {
            id
            name
            fee
            interval
          }
        }

        nextSubscription {
          id
          state
          startsAt
          expiresAt

          plan {
            id
            name
            fee
            interval
          }
        }
      }
    `),
  );

  const scheduleSubscriptionCancellation = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_ScheduleSubscriptionCancellation_Mutation {
      scheduleSubscriptionCancellation {
        id
        state
        expiresAt
      }
    }
  `);

  const cancelSubscriptionCancellation = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_CancelSubscriptionCancellation_Mutation {
      cancelSubscriptionCancellation {
        id
        state
        expiresAt
      }
    }
  `);

  const schedulePlanChange = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_SchedulePlanChange_Mutation($input: SchedulePlanChangeInput!) {
      schedulePlanChange(input: $input) {
        id
        state
        startsAt
        expiresAt
        plan {
          id
          name
          fee
        }
      }
    }
  `);

  const cancelPlanChange = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_CancelPlanChange_Mutation {
      cancelPlanChange {
        id
        state
        expiresAt
      }
    }
  `);

  const recordSurvey = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_RecordSurvey_Mutation($input: RecordSurveyInput!) {
      recordSurvey(input: $input) {
        id
      }
    }
  `);

  let updatePaymentMethodOpen = $state(false);
  let redeemCreditCodeOpen = $state(false);
  let cancellationSurveyOpen = $state(false);

  async function handleCancellationSurveySubmit(surveyData: unknown) {
    await recordSurvey({
      name: 'subscription_cancellation_202510',
      value: surveyData,
    });

    await scheduleSubscriptionCancellation();

    mixpanel.track('cancel_plan', surveyData as Record<string, unknown>);
  }
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>결제</h1>
  </div>

  <!-- Current Plan Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>현재 플랜</h2>

    {#if !$user.subscription}
      <SettingsCard>
        <SettingsRow>
          {#snippet label()}
            타이피 BASIC ACCESS
          {/snippet}
          {#snippet description()}
            타이피의 기본 기능을 무료로 이용할 수 있어요.
          {/snippet}
          {#snippet value()}
            <Button onclick={() => (updatePaymentMethodOpen = true)} size="sm" variant="secondary">업그레이드</Button>
          {/snippet}
        </SettingsRow>
      </SettingsCard>
    {:else}
      {@const subscription = $user.subscription}
      <SettingsCard>
        <SettingsRow>
          {#snippet label()}
            {subscription.plan.name} 플랜
          {/snippet}
          {#snippet description()}
            {#if subscription.state === SubscriptionState.ACTIVE}
              <span>
                {dayjs(subscription.expiresAt).formatAsDate()}에 {comma(subscription.plan.fee)}원 결제 예정
              </span>
            {:else if subscription.state === SubscriptionState.WILL_EXPIRE}
              <span class={css({ color: 'text.danger' })}>
                {dayjs(subscription.expiresAt).formatAsDate()} 해지 예정
              </span>
            {/if}
          {/snippet}
          {#snippet value()}
            이용 기간: {dayjs(subscription.startsAt).formatAsDate()} ~ {dayjs(subscription.expiresAt).formatAsDate()}
          {/snippet}
        </SettingsRow>

        {#if subscription.state === SubscriptionState.ACTIVE && !$user.nextSubscription && PlanPair[subscription.plan.id as keyof typeof PlanPair]}
          <SettingsDivider />

          <SettingsRow>
            {#snippet label()}
              플랜 전환
            {/snippet}
            {#snippet description()}
              {@const isMonthly = subscription.plan.interval === PlanInterval.MONTHLY}
              {isMonthly ? '1년 단위로 결제하면 2개월 무료 혜택을 받아요.' : '한 달 단위로 결제할 수 있어요.'}
            {/snippet}
            {#snippet value()}
              {@const targetPlanId = PlanPair[subscription.plan.id as keyof typeof PlanPair]}
              {@const isMonthly = subscription.plan.interval === PlanInterval.MONTHLY}
              <Button
                onclick={() => {
                  Dialog.confirm({
                    title: isMonthly ? '연간 플랜으로 전환하시겠어요?' : '월간 플랜으로 전환하시겠어요?',
                    message: isMonthly
                      ? `다음 결제일(${dayjs(subscription.expiresAt).formatAsDate()})부터 연간 플랜(49,000원/년)이 적용돼요.`
                      : `다음 결제일(${dayjs(subscription.expiresAt).formatAsDate()})부터 월간 플랜(4,900원/월)이 적용돼요.`,
                    actionLabel: '전환하기',
                    actionHandler: async () => {
                      await schedulePlanChange({ planId: targetPlanId });
                      cache.invalidate({ __typename: 'User', id: $user.id, field: 'subscription' });
                      cache.invalidate({ __typename: 'User', id: $user.id, field: 'nextSubscription' });
                      mixpanel.track('change_plan', {
                        from: isMonthly ? 'monthly' : 'yearly',
                        to: isMonthly ? 'yearly' : 'monthly',
                      });
                    },
                  });
                }}
                size="sm"
                variant="secondary"
              >
                {isMonthly ? '연간 플랜으로 전환' : '월간 플랜으로 전환'}
              </Button>
            {/snippet}
          </SettingsRow>
        {/if}

        {#if subscription.state === SubscriptionState.WILL_EXPIRE && !$user.nextSubscription}
          <SettingsDivider />

          <SettingsRow>
            {#snippet label()}
              구독 재개
            {/snippet}
            {#snippet description()}
              해지를 취소하고 다음 결제일부터 자동 갱신을 계속해요.
            {/snippet}
            {#snippet value()}
              <Button
                onclick={() => {
                  Dialog.confirm({
                    title: '구독 해지를 취소하시겠어요?',
                    message: '구독이 계속 유지되며, 다음 결제일에 자동으로 결제돼요.',
                    actionLabel: '해지 취소',
                    actionHandler: async () => {
                      await cancelSubscriptionCancellation();
                      mixpanel.track('resume_subscription');
                    },
                  });
                }}
                size="sm"
                variant="secondary"
              >
                해지 취소
              </Button>
            {/snippet}
          </SettingsRow>
        {/if}
      </SettingsCard>

      {#if $user.nextSubscription}
        {@const nextSubscription = $user.nextSubscription}
        <div class={css({ marginTop: '16px' })}>
          <p class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', marginBottom: '12px' })}>다음 플랜 (예정)</p>
          <SettingsCard>
            <SettingsRow>
              {#snippet label()}
                {nextSubscription.plan.name} 플랜
              {/snippet}
              {#snippet description()}
                {dayjs(nextSubscription.startsAt).formatAsDate()}부터 시작
              {/snippet}
              {#snippet value()}
                <Button
                  onclick={() => {
                    Dialog.confirm({
                      title: '플랜 전환을 취소하시겠어요?',
                      message: '현재 플랜이 계속 유지돼요.',
                      actionLabel: '전환 취소',
                      actionHandler: async () => {
                        await cancelPlanChange();
                        cache.invalidate({ __typename: 'User', id: $user.id, field: 'subscription' });
                        cache.invalidate({ __typename: 'User', id: $user.id, field: 'nextSubscription' });
                        mixpanel.track('cancel_plan_change');
                      },
                    });
                  }}
                  size="sm"
                  variant="secondary"
                >
                  전환 취소
                </Button>
              {/snippet}
            </SettingsRow>
          </SettingsCard>
        </div>
      {/if}
    {/if}
  </div>

  <!-- Payment Methods Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>결제 수단</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          결제 카드
        {/snippet}
        {#snippet description()}
          {#if $user.billingKey}
            {$user.billingKey.name}
          {:else}
            등록된 카드가 없어요.
          {/if}
        {/snippet}
        {#snippet value()}
          <Button onclick={() => (updatePaymentMethodOpen = true)} size="sm" variant="secondary">
            {$user.billingKey ? '카드 변경' : '카드 등록'}
          </Button>
        {/snippet}
      </SettingsRow>
    </SettingsCard>

    <div class={css({ marginTop: '16px' })}>
      <SettingsCard>
        <SettingsRow>
          {#snippet label()}
            현재 크레딧
          {/snippet}
          {#snippet description()}
            구독료 결제 시 크레딧이 있으면 우선 차감돼요.
          {/snippet}
          {#snippet value()}
            <span>{comma($user.credit)}원</span>
          {/snippet}
        </SettingsRow>

        <SettingsDivider />

        <SettingsRow>
          {#snippet label()}
            할인 코드
          {/snippet}
          {#snippet description()}
            이벤트나 프로모션 코드로 크레딧을 충전해요.
          {/snippet}
          {#snippet value()}
            <Button onclick={() => (redeemCreditCodeOpen = true)} size="sm" variant="secondary">코드 등록</Button>
          {/snippet}
        </SettingsRow>
      </SettingsCard>
    </div>
  </div>

  {#if $user.subscription?.state === SubscriptionState.ACTIVE || $user.subscription?.state === SubscriptionState.IN_GRACE_PERIOD}
    <!-- Subscription Cancellation Section -->
    <div>
      <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>구독 해지</h2>

      <SettingsCard>
        <SettingsRow>
          {#snippet label()}
            구독 해지
          {/snippet}
          {#snippet description()}
            해지 후에도 결제일까지는 유료 기능을 계속 사용할 수 있어요.
          {/snippet}
          {#snippet value()}
            <Button
              onclick={() => {
                cancellationSurveyOpen = true;
              }}
              size="sm"
              variant="ghost"
            >
              해지하기
            </Button>
          {/snippet}
        </SettingsRow>
      </SettingsCard>
    </div>
  {/if}
</div>

<UpdatePaymentMethodModal {$user} bind:open={updatePaymentMethodOpen} />
<RedeemCreditCodeModal bind:open={redeemCreditCodeOpen} />
<SubscriptionCancellationSurveyModal {$user} onSubmit={handleCancellationSurveySubmit} bind:open={cancellationSurveyOpen} />
