<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Checkbox, Modal, SegmentButtons, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { PlanId } from '@/const';
  import { PlanInterval } from '@/enums';
  import { TypieError } from '@/errors';
  import { cardSchema } from '@/validation';
  import { fragment, graphql } from '$graphql';
  import { fb } from '$lib/analytics';
  import type { DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user } from '$graphql';

  type Props = {
    open: boolean;
    $user: DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user;
  };

  let { open = $bindable(), $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user on User {
        id
        credit

        subscription {
          id
        }
      }
    `),
  );

  const updateBillingKey = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_UpdateBillingKey_Mutation($input: UpdateBillingKeyInput!) {
      updateBillingKey(input: $input) {
        id
        name
        createdAt
      }
    }
  `);

  const subscribePlanWithBillingKey = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_SubscribePlanWithBillingKey_Mutation(
      $input: SubscribePlanWithBillingKeyInput!
    ) {
      subscribePlanWithBillingKey(input: $input) {
        id

        user {
          id
          ...DashboardLayout_PreferenceModal_BillingTab_user
          ...DashboardLayout_PlanUsageWidget_user
          ...DashboardLayout_UserMenu_user
          ...Editor_BottomToolbar_FontFamily_user
          ...Editor_BottomToolbar_FontWeight_user

          sites {
            id

            ...Editor_TopToolbar_site
            ...Editor_Limit_site
          }
        }
      }
    }
  `);

  let interval = $state<PlanInterval>(PlanInterval.MONTHLY);
  let submitError = $state<string | null>(null);

  const form = createForm({
    schema: z.object({
      cardNumber: cardSchema.cardNumber,
      expiryDate: cardSchema.expiryDate,
      birthOrBusinessRegistrationNumber: cardSchema.birthOrBusinessRegistrationNumber,
      passwordTwoDigits: cardSchema.passwordTwoDigits,
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

      await updateBillingKey({
        birthOrBusinessRegistrationNumber: data.birthOrBusinessRegistrationNumber,
        cardNumber: data.cardNumber,
        expiryDate: data.expiryDate,
        passwordTwoDigits: data.passwordTwoDigits,
      });

      mixpanel.track('update_payment_billing_key');
      fb.track('AddPaymentInfo');

      if ($user.subscription) {
        Toast.success('카드 정보가 변경되었어요.');
        open = false;
      } else {
        if (interval === PlanInterval.MONTHLY) {
          await subscribePlanWithBillingKey({ planId: PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY });
          mixpanel.track('enroll_plan', { planId: PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY });
          fb.track('Subscribe', { value: '4900.00', currency: 'KRW', predicted_ltv: '4900.00' });
        } else if (interval === PlanInterval.YEARLY) {
          await subscribePlanWithBillingKey({ planId: PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY });
          mixpanel.track('enroll_plan', { planId: PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY });
          fb.track('Subscribe', { value: '49000.00', currency: 'KRW', predicted_ltv: '49000.00' });
        }

        open = false;
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
  const creditDiscount = $derived(Math.min($user.credit, planFee));
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
    {$user.subscription ? '결제 수단 변경' : '플랜 업그레이드'}
  </h2>

  <form class={flex({ direction: 'column', gap: '24px' })} onsubmit={form.handleSubmit}>
    {#if !$user.subscription}
      <div class={flex({ direction: 'column', gap: '16px' })}>
        <div>
          <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', marginBottom: '8px' })}>플랜 선택</div>
          <div class={css({ position: 'relative' })}>
            <SegmentButtons
              items={[
                { label: '월 4,900원', value: PlanInterval.MONTHLY },
                { label: '연 49,000원', value: PlanInterval.YEARLY },
              ]}
              onselect={(value) => {
                interval = value;
              }}
              size="sm"
              value={interval}
            />

            <div
              class={css({
                position: 'absolute',
                top: '-8px',
                right: '4px',
                borderRadius: 'full',
                paddingX: '8px',
                paddingY: '2px',
                fontSize: '11px',
                fontWeight: 'semibold',
                color: 'text.bright',
                backgroundColor: 'accent.brand.default',
                pointerEvents: 'none',
              })}
            >
              2개월 무료
            </div>
          </div>
        </div>

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

    <div class={flex({ direction: 'column', gap: '12px' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', marginBottom: '4px' })}>카드 정보</div>

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

    <Button style={css.raw({ width: 'full' })} type="submit">
      {#if $user.subscription}
        변경하기
      {:else if finalAmount === 0}
        무료로 시작하기
      {:else}
        {comma(finalAmount)}원 결제하고 시작하기
      {/if}
    </Button>
  </form>
</Modal>
