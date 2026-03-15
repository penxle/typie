<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { PlanId } from '@typie/lib/const';
  import { PlanInterval } from '@typie/lib/enums';
  import { TypieError } from '@typie/lib/errors';
  import { cardSchema } from '@typie/lib/validation';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Checkbox, Modal, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import { z } from 'zod';
  import { fb } from '$lib/analytics';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
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

          user {
            id
            ...DashboardLayout_PreferenceModal_BillingTab_user
            ...DashboardLayout_PlanUsageWidget_user
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
        if (interval === PlanInterval.MONTHLY) {
          await subscribePlanWithBillingKey({ input: { planId: PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY } });
          mixpanel.track('enroll_plan', { planId: PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY });
          fb.track('Subscribe', { value: '4900.00', currency: 'KRW', predicted_ltv: '4900.00' });
        } else if (interval === PlanInterval.YEARLY) {
          await subscribePlanWithBillingKey({ input: { planId: PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY } });
          mixpanel.track('enroll_plan', { planId: PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY });
          fb.track('Subscribe', { value: '49000.00', currency: 'KRW', predicted_ltv: '49000.00' });
        }

        open = false;
        celebrationModalOpen = true;
      }
    },
    onError: (error) => {
      const errorMessages: Record<string, string> = {
        billing_key_issue_failed: '결제 키 발급에 실패했어요. 카드 정보를 확인해주세요.',
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
        agreementChecks = agreements.map(() => false);
      });
    }
  });

  const agreements = [
    { name: '타이피 결제 이용약관', url: 'https://typie.co/legal/terms' },
    { name: 'NICEPAY 전자금융거래 기본약관', url: 'https://www.nicepay.co.kr/cs/terms/policy1.do' },
  ];

  let agreementChecks = $state(agreements.map(() => false));
  const allChecked = $derived(agreementChecks.every(Boolean));

  $effect(() => {
    form.fields.agreementsAccepted = allChecked;
  });

  const planFee = $derived(interval === PlanInterval.MONTHLY ? 4900 : 49_000);
  const creditDiscount = $derived(Math.min(user.data.credit, planFee));
  const finalAmount = $derived(planFee - creditDiscount);

  const handleAllCheck = () => {
    agreementChecks = agreementChecks.map(() => !allChecked);
  };

  const formatBusinessNumber = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const value = input.value.replaceAll(/\D/g, '');

    if (value.length <= 6) {
      input.value = value;
    } else {
      const parts = [value.slice(0, 3), value.slice(3, 5), value.slice(5)];
      input.value = parts.filter(Boolean).join('-');
    }
  };

  const formatCardNumber = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const value = input.value.replaceAll(/\D/g, '');
    const parts = [value.slice(0, 4), value.slice(4, 8), value.slice(8, 12), value.slice(12)];
    input.value = parts.filter(Boolean).join('-');
  };

  const formatCardExpiry = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const value = input.value.replaceAll(/\D/g, '');
    input.value = value.length > 2 ? value.slice(0, 2) + '/' + value.slice(2, 4) : value;
  };
</script>

<Modal style={css.raw({ padding: '24px', maxWidth: '480px' })} bind:open>
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
              <span class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default' })}>4,900원</span>
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
              <span class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default' })}>49,000원</span>
            </div>
            <div class={css({ fontSize: '12px', color: 'text.subtle', marginTop: '4px' })}>
              매년 결제 · <span class={css({ color: 'accent.brand.default', fontWeight: 'medium' })}>월 4,083원</span>
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

        <div class={flex({ direction: 'column', gap: '8px' })}>
          <TextInput
            id="cardNumber"
            style={css.raw({ width: 'full' })}
            inputmode="numeric"
            maxlength={19}
            oninput={formatCardNumber}
            placeholder="카드 번호"
            bind:value={form.fields.cardNumber}
          />
          {#if form.errors.cardNumber}
            <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.cardNumber}</div>
          {/if}
        </div>

        <div class={flex({ gap: '8px' })}>
          <div class={flex({ direction: 'column', gap: '8px', flex: '1' })}>
            <TextInput
              id="expiryDate"
              style={css.raw({ width: 'full' })}
              inputmode="numeric"
              maxlength={5}
              oninput={formatCardExpiry}
              placeholder="유효기간 (MM/YY)"
              bind:value={form.fields.expiryDate}
            />
            {#if form.errors.expiryDate}
              <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.expiryDate}</div>
            {/if}
          </div>

          <div class={flex({ direction: 'column', gap: '8px', flex: '1' })}>
            <TextInput
              id="passwordTwoDigits"
              style={css.raw({ width: 'full' })}
              autocomplete="off"
              inputmode="numeric"
              maxlength={2}
              placeholder="비밀번호 앞 2자리"
              type="password"
              bind:value={form.fields.passwordTwoDigits}
            />
            {#if form.errors.passwordTwoDigits}
              <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.passwordTwoDigits}</div>
            {/if}
          </div>
        </div>

        <div class={flex({ direction: 'column', gap: '8px' })}>
          <TextInput
            id="birthOrBusinessRegistrationNumber"
            style={css.raw({ width: 'full' })}
            inputmode="numeric"
            maxlength={12}
            oninput={formatBusinessNumber}
            placeholder="생년월일 6자리 또는 사업자번호 10자리"
            bind:value={form.fields.birthOrBusinessRegistrationNumber}
          />
          {#if form.errors.birthOrBusinessRegistrationNumber}
            <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>
              {form.errors.birthOrBusinessRegistrationNumber}
            </div>
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
              <span class={css({ color: 'text.default' })}>최종 금액</span>
              <span class={css({ color: 'text.default' })}>{comma(finalAmount)}원</span>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <div class={flex({ direction: 'column', gap: '8px' })}>
      <div
        class={css({
          borderRadius: '8px',
          borderWidth: '1px',
          borderColor: 'border.subtle',
          padding: '16px',
          backgroundColor: 'surface.default',
        })}
      >
        <div class={flex({ direction: 'column', gap: '12px' })}>
          <Checkbox checked={allChecked} onchange={handleAllCheck} size="sm">
            <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>전체 동의</span>
          </Checkbox>

          <div class={css({ height: '1px', backgroundColor: 'border.subtle' })}></div>

          <div class={flex({ direction: 'column', gap: '8px' })}>
            {#each agreements as agreement (agreement.name)}
              <Checkbox size="sm" bind:checked={agreementChecks[agreements.indexOf(agreement)]}>
                <span class={css({ fontSize: '13px', color: 'text.subtle' })}>
                  <a
                    class={css({ color: 'text.default', textDecoration: 'underline', _hover: { color: 'accent.brand.default' } })}
                    href={agreement.url}
                    rel="noopener noreferrer"
                    target="_blank"
                  >
                    {agreement.name}
                  </a>
                  동의 (필수)
                </span>
              </Checkbox>
            {/each}
          </div>
        </div>
      </div>

      {#if form.errors.agreementsAccepted}
        <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.agreementsAccepted}</div>
      {/if}
    </div>

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
      {:else if finalAmount === 0}
        무료로 시작하기
      {:else}
        {comma(finalAmount)}원 결제하고 시작하기
      {/if}
    </Button>
  </form>
</Modal>

<SubscriptionCelebrationModal
  message="타이피의 모든 기능을 자유롭게 이용해보세요."
  title="구독이 시작됐어요!"
  bind:open={celebrationModalOpen}
/>
