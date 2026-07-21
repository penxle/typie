<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { PlanPair, SUBSCRIPTION_GRACE_DAYS } from '@typie/lib/const';
  import { PlanAvailability, PlanInterval, SubscriptionState } from '@typie/lib/enums';
  import { TypieError } from '@typie/lib/errors';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import RedeemCreditCodeModal from './RedeemCreditCodeModal.svelte';
  import SubscriptionCancellationSurveyModal from './SubscriptionCancellationSurveyModal.svelte';
  import UpdatePaymentMethodModal from './UpdatePaymentMethodModal.svelte';
  import type { DashboardLayout_PreferenceModal_BillingTab_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_BillingTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
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
            availability
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
    () => user$key,
  );

  const isTrial = $derived(user.data.subscription?.plan.availability === PlanAvailability.TRIAL);
  const isBillingKey = $derived(user.data.subscription?.plan.availability === PlanAvailability.BILLING_KEY);
  const isInAppPurchase = $derived(user.data.subscription?.plan.availability === PlanAvailability.IN_APP_PURCHASE);

  const [scheduleSubscriptionCancellation] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_BillingTab_ScheduleSubscriptionCancellation_Mutation {
        scheduleSubscriptionCancellation {
          id
          state
          expiresAt
        }
      }
    `),
  );

  const [cancelSubscriptionCancellation] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_BillingTab_CancelSubscriptionCancellation_Mutation {
        cancelSubscriptionCancellation {
          id
          state
          expiresAt
        }
      }
    `),
  );

  const [schedulePlanChange] = createMutation(
    graphql(`
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
    `),
  );

  const [cancelPlanChange] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_BillingTab_CancelPlanChange_Mutation {
        cancelPlanChange {
          id
          state
          expiresAt
        }
      }
    `),
  );

  const [subscribePlanWithBillingKey] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_BillingTab_SubscribePlanWithBillingKey_Mutation($input: SubscribePlanWithBillingKeyInput!) {
        subscribePlanWithBillingKey(input: $input) {
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
    `),
  );

  const [recordSurvey] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_BillingTab_RecordSurvey_Mutation($input: RecordSurveyInput!) {
        recordSurvey(input: $input) {
          id
        }
      }
    `),
  );

  const [deleteBillingKey] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_BillingTab_DeleteBillingKey_Mutation {
        deleteBillingKey
      }
    `),
  );

  let updatePaymentMethodOpen = $state(false);
  let updatePaymentMethodMode = $state<'register' | 'subscribe'>('register');
  let redeemCreditCodeOpen = $state(false);
  let cancellationSurveyOpen = $state(false);

  async function handleCancellationSurveySubmit(surveyData: unknown) {
    await recordSurvey({
      input: {
        name: 'subscription_cancellation_202510',
        value: surveyData,
      },
    });

    await scheduleSubscriptionCancellation();

    mixpanel.track('cancel_plan', surveyData as Record<string, unknown>);
    Toast.success('구독이 해지되었어요');
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

    {#if !user.data.subscription}
      <SettingsCard>
        <SettingsRow>
          {#snippet label()}
            구독 없음
          {/snippet}
          {#snippet description()}
            읽기 전용 상태예요.
          {/snippet}
          {#snippet value()}
            <Button
              onclick={() => {
                updatePaymentMethodMode = 'subscribe';
                updatePaymentMethodOpen = true;
              }}
              size="sm"
              variant="primary"
            >
              구독 시작하기
            </Button>
          {/snippet}
        </SettingsRow>
      </SettingsCard>
    {:else}
      {@const subscription = user.data.subscription}
      <SettingsCard>
        <SettingsRow>
          {#snippet label()}
            {subscription.plan.name} 플랜
          {/snippet}
          {#snippet description()}
            {#if isTrial}
              <span>
                {#if user.data.nextSubscription}
                  무료 체험이 {dayjs(subscription.expiresAt).formatAsDate()}에 종료되고 {user.data.nextSubscription.plan.name} 플랜이 시작돼요.
                {:else}
                  무료 체험이 {dayjs(subscription.expiresAt).formatAsDate()}에 종료돼요.
                {/if}
              </span>
            {:else if subscription.state === SubscriptionState.ACTIVE}
              <span>
                {dayjs(subscription.expiresAt).formatAsDate()}에 {comma(subscription.plan.fee)}원 결제 예정
              </span>
            {:else if subscription.state === SubscriptionState.IN_GRACE_PERIOD}
              <span class={css({ color: 'text.danger' })}>
                결제에 실패했어요. {dayjs(subscription.expiresAt).add(SUBSCRIPTION_GRACE_DAYS, 'day').formatAsDate()}까지 결제 수단을 확인해
                주세요.
              </span>
            {:else if subscription.state === SubscriptionState.WILL_EXPIRE && user.data.nextSubscription}
              <span>
                {dayjs(subscription.expiresAt).formatAsDate()}에 다음 플랜으로 전환 예정
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

        {#if subscription.state === SubscriptionState.ACTIVE && isBillingKey && !user.data.nextSubscription && PlanPair[subscription.plan.id as keyof typeof PlanPair]}
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
                      ? `다음 결제일(${dayjs(subscription.expiresAt).formatAsDate()})부터 연간 플랜(29,000원/년)이 적용돼요.`
                      : `다음 결제일(${dayjs(subscription.expiresAt).formatAsDate()})부터 월간 플랜(2,900원/월)이 적용돼요.`,
                    actionLabel: '전환하기',
                    actionHandler: async () => {
                      await schedulePlanChange({ input: { planId: targetPlanId } });
                      cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'subscription' });
                      cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'nextSubscription' });
                      mixpanel.track('change_plan', {
                        from: isMonthly ? 'monthly' : 'yearly',
                        to: isMonthly ? 'yearly' : 'monthly',
                      });
                      Toast.success(isMonthly ? '연간 플랜으로 전환되었어요' : '월간 플랜으로 전환되었어요');
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

        {#if subscription.state === SubscriptionState.WILL_EXPIRE && !user.data.nextSubscription && (isTrial || isBillingKey)}
          <SettingsDivider />

          {#if isTrial}
            <SettingsRow>
              {#snippet label()}
                업그레이드
              {/snippet}
              {#snippet description()}
                결제 수단을 등록하고 유료 플랜으로 업그레이드하세요.
              {/snippet}
              {#snippet value()}
                <Button
                  onclick={() => {
                    updatePaymentMethodMode = 'subscribe';
                    updatePaymentMethodOpen = true;
                  }}
                  size="sm"
                  variant="primary"
                >
                  지금 업그레이드
                </Button>
              {/snippet}
            </SettingsRow>
          {:else}
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
                        try {
                          await cancelSubscriptionCancellation();
                          mixpanel.track('resume_subscription');
                          Toast.success('구독 해지가 취소되었어요');
                        } catch (err) {
                          if (err instanceof TypieError && err.code === 'subscription_already_expired') {
                            Toast.error('구독이 이미 만료되었어요. 새로 구독해 주세요.');
                          } else {
                            throw err;
                          }
                        } finally {
                          cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'subscription' });
                        }
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
        {/if}
      </SettingsCard>

      {#if isInAppPurchase}
        <p class={css({ marginTop: '12px', fontSize: '13px', color: 'text.faint' })}>
          이 구독은 앱에서 결제되었어요. 플랜 변경 및 해지는 App Store 또는 Google Play에서 관리할 수 있어요.
        </p>
      {/if}

      {#if user.data.nextSubscription}
        {@const nextSubscription = user.data.nextSubscription}
        <div class={css({ marginTop: '16px' })}>
          <p class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', marginBottom: '12px' })}>다음 플랜 (예정)</p>
          <SettingsCard>
            <SettingsRow>
              {#snippet label()}
                {nextSubscription.plan.name} 플랜
              {/snippet}
              {#snippet description()}
                {dayjs(nextSubscription.startsAt).formatAsDate()}에 {comma(nextSubscription.plan.fee)}원이 결제될 예정이에요. 크레딧이
                있으면 차감된 금액으로 결제돼요.
              {/snippet}
              {#snippet value()}
                <div class={flex({ gap: '8px' })}>
                  {#if isTrial && PlanPair[nextSubscription.plan.id as keyof typeof PlanPair]}
                    {@const targetPlanId = PlanPair[nextSubscription.plan.id as keyof typeof PlanPair]}
                    {@const isMonthly = nextSubscription.plan.interval === PlanInterval.MONTHLY}
                    <Button
                      onclick={() => {
                        Dialog.confirm({
                          title: isMonthly ? '연간 플랜으로 변경하시겠어요?' : '월간 플랜으로 변경하시겠어요?',
                          message: isMonthly
                            ? '무료 체험이 끝나면 연간 플랜(29,000원/년)으로 시작해요.'
                            : '무료 체험이 끝나면 월간 플랜(2,900원/월)으로 시작해요.',
                          actionLabel: '변경하기',
                          actionHandler: async () => {
                            try {
                              const result = await subscribePlanWithBillingKey({ input: { planId: targetPlanId } });
                              const scheduled = result.subscribePlanWithBillingKey.state === SubscriptionState.WILL_ACTIVATE;
                              mixpanel.track('enroll_plan', { planId: targetPlanId, scheduled });
                              Toast.success(
                                scheduled
                                  ? isMonthly
                                    ? '연간 플랜으로 변경되었어요'
                                    : '월간 플랜으로 변경되었어요'
                                  : '플랜이 시작되었어요',
                              );
                            } catch (err) {
                              if (err instanceof TypieError && err.code === 'subscription_already_exists') {
                                Toast.error('이미 처리된 예약이에요. 새로고침 후 확인해 주세요.');
                              } else {
                                throw err;
                              }
                            } finally {
                              cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'subscription' });
                              cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'nextSubscription' });
                            }
                          },
                        });
                      }}
                      size="sm"
                      variant="secondary"
                    >
                      {isMonthly ? '연간 플랜으로 변경' : '월간 플랜으로 변경'}
                    </Button>
                  {/if}
                  <Button
                    onclick={() => {
                      Dialog.confirm({
                        title: isTrial ? '플랜 예약을 취소하시겠어요?' : '플랜 전환을 취소하시겠어요?',
                        message: isTrial ? '무료 체험은 그대로 유지되고, 종료 후 결제되지 않아요.' : '현재 플랜이 계속 유지돼요.',
                        actionLabel: isTrial ? '예약 취소' : '전환 취소',
                        actionHandler: async () => {
                          try {
                            await cancelPlanChange();
                            mixpanel.track('cancel_plan_change');
                            Toast.success(isTrial ? '플랜 예약이 취소되었어요' : '플랜 전환이 취소되었어요');
                          } catch (err) {
                            if (err instanceof TypieError && err.code === 'plan_change_already_processed') {
                              Toast.error('이미 처리된 예약이에요. 새로고침 후 확인해 주세요.');
                            } else {
                              throw err;
                            }
                          } finally {
                            cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'subscription' });
                            cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'nextSubscription' });
                          }
                        },
                      });
                    }}
                    size="sm"
                    variant="secondary"
                  >
                    {isTrial ? '예약 취소' : '전환 취소'}
                  </Button>
                </div>
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
          {#if user.data.billingKey}
            {user.data.billingKey.name}
          {:else}
            등록된 카드가 없어요.
          {/if}
        {/snippet}
        {#snippet value()}
          <div class={flex({ gap: '8px' })}>
            <Button
              onclick={() => {
                updatePaymentMethodMode = 'register';
                updatePaymentMethodOpen = true;
              }}
              size="sm"
              variant="secondary"
            >
              {user.data.billingKey ? '변경' : '카드 등록'}
            </Button>
            {#if user.data.billingKey && (!user.data.subscription || isTrial) && !user.data.nextSubscription}
              <Button
                onclick={() => {
                  Dialog.confirm({
                    title: '결제 카드를 삭제하시겠어요?',
                    message: '등록된 카드 정보가 삭제돼요. 유료 플랜을 구독하려면 다시 등록해야 해요.',
                    action: 'danger',
                    actionLabel: '삭제',
                    actionHandler: async () => {
                      await deleteBillingKey();
                      cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'billingKey' });
                      mixpanel.track('delete_billing_key');
                      Toast.success('카드가 삭제되었어요');
                    },
                  });
                }}
                size="sm"
                variant="secondary"
              >
                삭제
              </Button>
            {/if}
          </div>
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
            <span>{comma(user.data.credit)}원</span>
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

  {#if isBillingKey && (user.data.subscription?.state === SubscriptionState.ACTIVE || user.data.subscription?.state === SubscriptionState.IN_GRACE_PERIOD)}
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

<UpdatePaymentMethodModal mode={updatePaymentMethodMode} user$key={user.data} bind:open={updatePaymentMethodOpen} />
<RedeemCreditCodeModal bind:open={redeemCreditCodeOpen} />
<SubscriptionCancellationSurveyModal onSubmit={handleCancellationSurveySubmit} user$key={user.data} bind:open={cancellationSurveyOpen} />
