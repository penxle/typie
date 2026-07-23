<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { PlanId } from '@typie/lib/const';
  import { PlanAvailability, PlanInterval, SubscriptionState } from '@typie/lib/enums';
  import { TypieError } from '@typie/lib/errors';
  import { cardSchema } from '@typie/lib/validation';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import { z } from 'zod';
  import { fb } from '$lib/analytics';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import BillingCardForm from '../@subscription/BillingCardForm.svelte';
  import SubscriptionCelebrationModal from '../SubscriptionCelebrationModal.svelte';
  import type { DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user$key } from '$mearie';

  type Props = {
    open: boolean;
    user$key: DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user$key;
    mode: 'register' | 'subscribe';
  };

  let { open = $bindable(), user$key, mode }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user on User {
        id
        credit

        billingKey {
          id
          name
        }

        subscription {
          id
          expiresAt

          plan {
            id
            availability
          }
        }
      }
    `),
    () => user$key,
  );

  const [updateBillingKey] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_UpdateBillingKey_Mutation(
        $input: UpdateBillingKeyInput!
      ) {
        updateBillingKey(input: $input) {
          id
          name
          createdAt
        }
      }
    `),
  );

  const [subscribePlanWithBillingKey] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_SubscribePlanWithBillingKey_Mutation(
        $input: SubscribePlanWithBillingKeyInput!
      ) {
        subscribePlanWithBillingKey(input: $input) {
          id
          state

          user {
            id
            ...DashboardLayout_PreferenceModal_BillingTab_user
            ...DashboardLayout_TrialWidget_user
            ...DashboardLayout_Profile_user
          }
        }
      }
    `),
  );

  let interval = $state<PlanInterval>(PlanInterval.MONTHLY);
  let submitError = $state<string | null>(null);
  let celebrationModalOpen = $state(false);
  let isEditingCard = $state(false);

  const isTrial = $derived(user.data.subscription?.plan.availability === PlanAvailability.TRIAL);
  let scheduledCelebration = $state(false);

  const form = createForm({
    schema: z.object({
      cardNumber: cardSchema.cardNumber.optional(),
      expiryDate: cardSchema.expiryDate.optional(),
      birthOrBusinessRegistrationNumber: cardSchema.birthOrBusinessRegistrationNumber.optional(),
      passwordTwoDigits: cardSchema.passwordTwoDigits.optional(),
      agreementsAccepted: z.boolean(),
    }),
    defaultValues: {
      agreementsAccepted: false,
    },
    onSubmit: async (data) => {
      submitError = null;

      if (!data.agreementsAccepted) {
        throw new FormError('agreementsAccepted', '약관에 동의해주세요.');
      }

      const needsCardRegistration = mode === 'register' || !user.data.billingKey || isEditingCard;

      if (needsCardRegistration) {
        if (!data.cardNumber) {
          throw new FormError('cardNumber', '카드 번호를 입력해 주세요');
        }
        if (!data.expiryDate) {
          throw new FormError('expiryDate', '만료일을 입력해 주세요');
        }
        if (!data.passwordTwoDigits) {
          throw new FormError('passwordTwoDigits', '카드 비밀번호를 입력해 주세요');
        }
        if (!data.birthOrBusinessRegistrationNumber) {
          throw new FormError('birthOrBusinessRegistrationNumber', '생년월일 또는 사업자 등록번호를 입력해 주세요');
        }

        await updateBillingKey({
          input: {
            birthOrBusinessRegistrationNumber: data.birthOrBusinessRegistrationNumber,
            cardNumber: data.cardNumber,
            expiryDate: data.expiryDate,
            passwordTwoDigits: data.passwordTwoDigits,
          },
        });

        cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'billingKey' });
        mixpanel.track('update_payment_billing_key');
        fb.track('AddPaymentInfo');
      }

      if (mode === 'register') {
        Toast.success(user.data.billingKey ? '카드 정보가 변경되었어요.' : '카드가 등록되었어요.');
        open = false;
      } else {
        const planId =
          interval === PlanInterval.YEARLY ? PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY : PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY;
        const result = await subscribePlanWithBillingKey({ input: { planId } });
        // 사후 메시지·전환 이벤트는 사전 추정(isTrial)이 아니라 실제 처리 결과로 분기한다 — 롤링 배포·만료 직후 등록에서도 정합.
        const scheduled = result.subscribePlanWithBillingKey.state === SubscriptionState.WILL_ACTIVATE;

        mixpanel.track('enroll_plan', { planId, scheduled });
        if (!scheduled) {
          const value = interval === PlanInterval.YEARLY ? '29000.00' : '2900.00';
          fb.track('Subscribe', { value, currency: 'KRW', predicted_ltv: value });
        }

        scheduledCelebration = scheduled;
        open = false;
        celebrationModalOpen = true;
      }
    },
    onError: (error) => {
      const errorMessages: Record<string, string> = {
        billing_key_issue_failed: '결제 키 발급에 실패했어요. 카드 정보를 확인해주세요.',
        billing_key_required: '결제 카드를 먼저 등록해 주세요.',
        plan_already_enrolled: '이미 구독 중이에요.',
        unpaid_invoice_exists: '미결제 내역이 있어요. 고객센터에 문의해주세요.',
        payment_failed: '결제에 실패했어요. 카드 정보를 확인해주세요.',
      };

      if (error instanceof TypieError) {
        submitError = errorMessages[error.code] || error.code;
      }
    },
  });

  $effect(() => {
    void form;
  });

  $effect(() => {
    if (!open) {
      untrack(() => {
        form.reset();
        isEditingCard = false;
        submitError = null;
      });
    }
  });

  const planFee = $derived(interval === PlanInterval.MONTHLY ? 2900 : 29_000);
  const creditDiscount = $derived(Math.min(user.data.credit, planFee));
  const finalAmount = $derived(planFee - creditDiscount);
</script>

<Modal style={css.raw({ padding: '24px', maxWidth: '480px' })} closable={!form.state.isLoading} bind:open>
  <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>
    {mode === 'register' ? (user.data.billingKey ? '결제 카드 변경' : '결제 카드 등록') : '플랜 업그레이드'}
  </h2>

  <form class={flex({ direction: 'column', gap: '24px' })} onsubmit={form.handleSubmit}>
    {#if mode === 'subscribe'}
      <div>
        <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', marginBottom: '8px' })}>플랜 선택</div>
        <div class={flex({ gap: '8px' })}>
          <button
            class={css({
              flex: '1',
              padding: '12px',
              borderRadius: '6px',
              borderWidth: '1px',
              borderColor: interval === PlanInterval.MONTHLY ? 'accent.brand.default' : 'border.subtle',
              backgroundColor: 'surface.default',
              cursor: 'pointer',
              transition: 'common',
              textAlign: 'left',
              _hover: { borderColor: interval === PlanInterval.MONTHLY ? 'accent.brand.default' : 'border.default' },
            })}
            onclick={() => (interval = PlanInterval.MONTHLY)}
            type="button"
          >
            <div class={flex({ justify: 'space-between', alignItems: 'center' })}>
              <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>월간</span>
              <span class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default' })}>2,900원</span>
            </div>
            <div class={css({ fontSize: '12px', color: 'text.subtle', marginTop: '4px' })}>매월 결제</div>
          </button>

          <button
            class={css({
              flex: '1',
              position: 'relative',
              padding: '12px',
              borderRadius: '6px',
              borderWidth: '1px',
              borderColor: interval === PlanInterval.YEARLY ? 'accent.brand.default' : 'border.subtle',
              backgroundColor: 'surface.default',
              cursor: 'pointer',
              transition: 'common',
              textAlign: 'left',
              _hover: { borderColor: interval === PlanInterval.YEARLY ? 'accent.brand.default' : 'border.default' },
            })}
            onclick={() => (interval = PlanInterval.YEARLY)}
            type="button"
          >
            <div
              class={css({
                position: 'absolute',
                top: '-8px',
                right: '8px',
                borderRadius: 'full',
                paddingX: '8px',
                paddingY: '2px',
                fontSize: '11px',
                fontWeight: 'semibold',
                color: 'text.bright',
                backgroundColor: 'accent.brand.default',
              })}
            >
              2개월 무료
            </div>
            <div class={flex({ justify: 'space-between', alignItems: 'center' })}>
              <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>연간</span>
              <span class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default' })}>29,000원</span>
            </div>
            <div class={css({ fontSize: '12px', color: 'text.subtle', marginTop: '4px' })}>
              매년 결제 · <span class={css({ color: 'accent.brand.default', fontWeight: 'medium' })}>월 2,416원</span>
            </div>
          </button>
        </div>
      </div>
    {/if}

    {#if mode === 'subscribe' && user.data.billingKey && !isEditingCard}
      <div class={flex({ direction: 'column', gap: '12px' })}>
        <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', marginBottom: '4px' })}>결제 카드</div>
        <div
          class={flex({
            justify: 'space-between',
            alignItems: 'center',
            borderRadius: '8px',
            borderWidth: '1px',
            borderColor: 'border.subtle',
            padding: '12px',
            backgroundColor: 'surface.default',
          })}
        >
          <span class={css({ fontSize: '14px', color: 'text.default' })}>{user.data.billingKey.name}</span>
          <Button onclick={() => (isEditingCard = true)} size="sm" variant="secondary">카드 변경</Button>
        </div>
      </div>
    {:else}
      <div class={flex({ direction: 'column', gap: '12px' })}>
        <div class={flex({ justify: 'space-between', alignItems: 'center', marginBottom: '4px' })}>
          <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>카드 정보</div>
          {#if isEditingCard && user.data.billingKey}
            <button
              class={css({
                fontSize: '13px',
                fontWeight: 'medium',
                color: 'text.faint',
                cursor: 'pointer',
                transition: 'common',
                _hover: { color: 'text.muted' },
              })}
              onclick={() => (isEditingCard = false)}
              type="button"
            >
              기존 카드 사용하기
            </button>
          {/if}
        </div>
      </div>
    {/if}

    {#if mode === 'subscribe'}
      <div>
        <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', marginBottom: '8px' })}>결제 정보</div>
        <div
          class={css({
            borderRadius: '6px',
            borderWidth: '1px',
            borderColor: 'border.subtle',
            padding: '12px',
            backgroundColor: 'surface.default',
          })}
        >
          <div class={flex({ justify: 'space-between', fontSize: '13px', marginBottom: '6px' })}>
            <span class={css({ color: 'text.subtle' })}>플랜 금액</span>
            <span class={css({ color: 'text.default' })}>{comma(planFee)}원</span>
          </div>
          {#if creditDiscount > 0}
            <div class={flex({ justify: 'space-between', fontSize: '13px', marginBottom: '6px' })}>
              <span class={css({ color: 'text.subtle' })}>크레딧 차감</span>
              <span class={css({ color: 'accent.brand.default', fontWeight: 'medium' })}>-{comma(creditDiscount)}원</span>
            </div>
          {/if}
          <div
            class={css({
              marginTop: '8px',
              paddingTop: '8px',
              borderTopWidth: '1px',
              borderColor: 'border.subtle',
            })}
          >
            <div class={flex({ justify: 'space-between', fontSize: '14px', fontWeight: 'semibold' })}>
              <span class={css({ color: 'text.default' })}>{isTrial ? '예상 결제 금액' : '최종 금액'}</span>
              <span class={css({ color: 'text.default' })}>{comma(finalAmount)}원</span>
            </div>
          </div>
          {#if isTrial && user.data.subscription}
            <div class={css({ marginTop: '8px', fontSize: '12px', color: 'text.subtle' })}>
              무료 체험이 끝나는 {dayjs(user.data.subscription.expiresAt).formatAsDate()}에 결제돼요. 크레딧은 결제 시점 잔액 기준으로
              차감돼요.
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <BillingCardForm
      errors={form.errors}
      fields={form.fields}
      showCardFields={!(mode === 'subscribe' && user.data.billingKey) || isEditingCard}
    />

    {#if submitError}
      <div
        class={css({
          padding: '12px',
          borderRadius: '6px',
          backgroundColor: 'accent.danger.subtle',
          borderWidth: '1px',
          borderColor: 'border.danger',
        })}
      >
        <div class={css({ fontSize: '13px', color: 'text.danger' })}>{submitError}</div>
      </div>
    {/if}

    <Button style={css.raw({ width: 'full' })} loading={form.state.isLoading} type="submit">
      {#if mode === 'register'}
        {user.data.billingKey ? '변경하기' : '등록하기'}
      {:else if isTrial}
        {finalAmount === 0 ? '구독 예약하기' : `${comma(finalAmount)}원 결제 예약하기`}
      {:else if finalAmount === 0}
        구독 시작하기
      {:else}
        {comma(finalAmount)}원 결제하고 시작하기
      {/if}
    </Button>
  </form>
</Modal>

<SubscriptionCelebrationModal
  message={scheduledCelebration ? '무료 체험이 끝나면 자동으로 결제되고 플랜이 시작돼요.' : '타이피의 모든 기능을 자유롭게 이용해보세요.'}
  title={scheduledCelebration ? '구독이 예약됐어요!' : '구독이 시작됐어요!'}
  bind:open={celebrationModalOpen}
/>
