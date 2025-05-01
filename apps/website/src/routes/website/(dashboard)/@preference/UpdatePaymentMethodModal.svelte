<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { cardSchema, redeemCodeSchema } from '@/validation';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import { fragment, graphql } from '$graphql';
  import { Button, Checkbox, HorizontalDivider, Icon, Modal, SegmentButtons, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { Dialog, Toast } from '$lib/notification';
  import { comma } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { UserPlanBillingCycle } from '@/enums';
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

        plan {
          id
        }
      }
    `),
  );

  const query = graphql(`
    query DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_Query($code: String!) @client {
      creditCode(code: $code) {
        id
        amount
        code
      }
    }
  `);

  const updatePaymentBillingKey = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_UpdatePaymentBillingKey_Mutation(
      $input: UpdatePaymentBillingKeyInput!
    ) {
      updatePaymentBillingKey(input: $input) {
        id
        name
        createdAt
      }
    }
  `);

  const redeemCreditCode = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_RedeemCreditCode_Mutation($input: RedeemCreditCodeInput!) {
      redeemCreditCode(input: $input) {
        id
        credit
      }
    }
  `);

  const enrollPlan = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_EnrollPlan_Mutation($input: EnrollPlanInput!) {
      enrollPlan(input: $input) {
        id
        fee
        createdAt
        billingCycle

        plan {
          id
          name
        }
      }
    }
  `);

  let billingCycle = $state<UserPlanBillingCycle>('MONTHLY');

  const redeemCodeForm = createForm({
    schema: z.object({
      code: redeemCodeSchema,
    }),
    onSubmit: async (data) => {
      const resp = await query.load({ code: data.code });
      redeemCode = resp.creditCode;
    },
    onError: () => {
      throw new FormError('code', '유효하지 않은 할인 코드입니다.');
    },
  });

  const form = createForm({
    schema: z.object({
      cardNumber: cardSchema.cardNumber,
      expiryDate: cardSchema.expiryDate,
      birthOrBusinessRegistrationNumber: cardSchema.birthOrBusinessRegistrationNumber,
      passwordTwoDigits: cardSchema.passwordTwoDigits,
    }),
    onSubmit: async (data) => {
      await updatePaymentBillingKey({
        birthOrBusinessRegistrationNumber: data.birthOrBusinessRegistrationNumber,
        cardNumber: data.cardNumber,
        expiryDate: data.expiryDate,
        passwordTwoDigits: data.passwordTwoDigits,
      });
      mixpanel.track('update_payment_billing_key');

      if ($user.plan) {
        open = false;
      } else {
        if (redeemCode) {
          await redeemCreditCode({ code: redeemCode.code });
          mixpanel.track('redeem_credit_code', { via: 'update-payment-method-modal' });
        }

        if (!redeemCode && (redeemCodeForm.errors.code || (redeemCodeForm.fields.code?.length ?? 0) > 0)) {
          Dialog.confirm({
            title: '할인 코드 사용',
            message: '할인 코드가 적용되지 않았어요. 그래도 결제를 할까요?',
            actionLabel: '결제',
            actionHandler: async () => {
              await enrollPlan({ billingCycle, planId: 'PL0PLUS' });
              mixpanel.track('enroll_plan', { billingCycle, planId: 'PL0PLUS' });
              open = false;
            },
          });
        } else {
          await enrollPlan({ billingCycle, planId: 'PL0PLUS' });
          mixpanel.track('enroll_plan', { billingCycle, planId: 'PL0PLUS' });
          open = false;
        }
      }
    },
    onError: (error) => {
      const errorMessages: Record<string, string> = {
        billing_key_issue_failed: '결제 키 발급에 실패했습니다. 카드 정보를 다시 확인해주세요.',
        plan_already_enrolled: '이미 결제 정보가 등록되어 있습니다.',
        payment_failed: '결제에 실패했습니다. 카드 정보를 다시 확인해주세요.',
      };

      if (error instanceof TypieError) {
        const message = errorMessages[error.code] || error.code;
        Toast.error(message);
      }
    },
  });

  const agreements = [
    { name: '타이피 결제 이용약관', url: 'https://help.typie.co/legal/terms' },
    { name: 'NICEPAY 전자금융거래 기본약관', url: 'https://www.nicepay.co.kr/cs/terms/policy1.do' },
  ];

  let agreementChecks = $state(agreements.map(() => false));
  const allChecked = $derived(agreementChecks.every(Boolean));
  let redeemInputOpen = $state(false);
  let redeemCode = $state<{ id: string; amount: number; code: string } | null>(null);
  let planFee = $derived(billingCycle === 'MONTHLY' ? 4900 : 49_000);
  let paymentAmount = $derived(
    planFee -
      ($user.credit >= planFee ? $user.credit - ($user.credit - planFee) : $user.credit) -
      (redeemCode ? (redeemCode.amount >= planFee ? redeemCode.amount - (redeemCode.amount - planFee) : redeemCode.amount) : 0),
  );

  const handleAllCheck = () => {
    agreementChecks = agreementChecks.map(() => !allChecked);
  };

  const formatBusinessNumber = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const value = input.value.replaceAll(/\D/g, '');
    const parts = [value.slice(0, 3), value.slice(3, 5), value.slice(5)];
    input.value = parts.filter(Boolean).join('-');
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

<Modal style={css.raw({ gap: '24px', padding: '20px', maxWidth: '500px' })} bind:open>
  <p class={css({ fontWeight: 'semibold' })}>카드 등록 및 결제</p>

  {#if !$user.plan}
    <div class={css({ position: 'relative' })}>
      <SegmentButtons
        items={[
          { label: '월 결제', value: 'MONTHLY' },
          { label: '연 결제', value: 'YEARLY' },
        ]}
        onselect={(value) => {
          billingCycle = value as UserPlanBillingCycle;
        }}
        size="sm"
        value={billingCycle}
      />

      <div
        class={css({
          position: 'absolute',
          top: '-10px',
          right: '-4px',
          borderRadius: 'full',
          paddingX: '8px',
          paddingY: '2px',
          fontSize: '12px',
          color: 'white',
          backgroundColor: 'brand.500',
          pointerEvents: 'none',
        })}
      >
        2개월 무료
      </div>
    </div>

    <div class={flex({ direction: 'column', gap: '8px' })}>
      <div
        class={css({
          borderRadius: '4px',
          padding: '12px',
          fontSize: '15px',
          fontWeight: 'medium',
          backgroundColor: 'gray.100',
        })}
      >
        <div class={flex({ align: 'center', justify: 'space-between' })}>
          <p>결제 금액</p>

          <p>{comma(paymentAmount)}원</p>
        </div>

        {#if paymentAmount !== planFee || $user.credit > 0 || redeemCode}
          <HorizontalDivider style={css.raw({ marginY: '4px' })} />
          <div class={flex({ direction: 'column', gap: '1px', fontSize: '12px', color: 'gray.700' })}>
            {#if paymentAmount !== planFee}
              <div>
                플랜 금액: {comma(planFee)}원
              </div>
            {/if}
            {#if $user.credit > 0}
              <div>
                잔여 크레딧: -{comma($user.credit)}원
              </div>
            {/if}
            {#if redeemCode}
              <div>
                할인 코드 사용: -{comma(redeemCode.amount)}원
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <div class={flex({ direction: 'column', gap: '4px' })}>
        <button
          class={flex({ align: 'center', justify: 'space-between', color: 'gray.400' })}
          onclick={() => (redeemInputOpen = !redeemInputOpen)}
          type="button"
        >
          <p class={css({ fontSize: '12px' })}>할인 코드를 갖고 계신가요?</p>

          {#if redeemInputOpen}
            <Icon icon={ChevronUpIcon} size={12} />
          {:else}
            <Icon icon={ChevronDownIcon} size={12} />
          {/if}
        </button>

        {#if redeemInputOpen}
          <form class={flex({ align: 'flex-start', gap: '4px' })} onsubmit={redeemCodeForm.handleSubmit}>
            <div class={css({ width: 'full' })}>
              <TextInput
                id="code"
                style={css.raw({ width: 'full' })}
                placeholder="할인 코드 입력하기"
                size="sm"
                bind:value={redeemCodeForm.fields.code}
              />

              {#if redeemCodeForm.errors.code}
                <div class={css({ marginTop: '4px', paddingLeft: '4px', fontSize: '12px', color: 'red.500' })}>
                  {redeemCodeForm.errors.code}
                </div>
              {/if}
            </div>

            <Button style={css.raw({ flex: 'none' })} size="sm" type="submit" variant="secondary">확인</Button>
          </form>
        {/if}

        {#if redeemCode}
          <div
            class={flex({
              align: 'center',
              justify: 'space-between',
              borderWidth: '1px',
              borderColor: 'gray.100',
              borderRadius: '4px',
              paddingX: '8px',
              paddingY: '6px',
            })}
          >
            <p class={css({ fontSize: '13px' })}>사전등록 할인</p>
            <p class={css({ fontSize: '13px', color: 'gray.600' })}>{comma(redeemCode.amount)}원</p>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <form class={flex({ direction: 'column', gap: '16px' })} onsubmit={form.handleSubmit}>
    <div>
      <label class={flex({ direction: 'column', gap: '4px', fontSize: '14px', color: 'gray.700', fontWeight: 'medium' })}>
        카드 번호
        <TextInput
          id="cardNumber"
          style={css.raw({ width: 'full' })}
          inputmode="numeric"
          maxlength={19}
          oninput={formatCardNumber}
          placeholder="0000-0000-0000-0000"
          bind:value={form.fields.cardNumber}
        />
      </label>

      {#if form.errors.cardNumber}
        <div class={css({ marginTop: '4px', paddingLeft: '4px', fontSize: '12px', color: 'red.500' })}>{form.errors.cardNumber}</div>
      {/if}
    </div>

    <div class={css({ display: 'flex', gap: '8px', width: 'full' })}>
      <div class={css({ flexGrow: '1' })}>
        <label class={flex({ direction: 'column', gap: '4px', fontSize: '14px', color: 'gray.700', fontWeight: 'medium' })}>
          유효 기간(MM/YY)
          <TextInput
            id="expiryDate"
            style={css.raw({ width: 'full' })}
            inputmode="numeric"
            maxlength={5}
            oninput={formatCardExpiry}
            placeholder="MM/YY"
            bind:value={form.fields.expiryDate}
          />
        </label>

        {#if form.errors.expiryDate}
          <div class={css({ marginTop: '4px', paddingLeft: '4px', fontSize: '12px', color: 'red.500' })}>{form.errors.expiryDate}</div>
        {/if}
      </div>

      <div class={css({ flexGrow: '1' })}>
        <label class={flex({ direction: 'column', gap: '4px', fontSize: '14px', color: 'gray.700', fontWeight: 'medium' })}>
          비밀번호 앞 두자리

          <TextInput
            id="passwordTwoDigits"
            style={css.raw({ width: 'full' })}
            autocomplete="off"
            inputmode="numeric"
            maxlength={2}
            placeholder="**"
            type="password"
            bind:value={form.fields.passwordTwoDigits}
          />
        </label>

        {#if form.errors.passwordTwoDigits}
          <div class={css({ marginTop: '4px', paddingLeft: '4px', fontSize: '12px', color: 'red.500' })}>
            {form.errors.passwordTwoDigits}
          </div>
        {/if}
      </div>
    </div>

    <div>
      <label class={flex({ direction: 'column', gap: '4px', fontSize: '14px', color: 'gray.700', fontWeight: 'medium' })}>
        생년월일 6자리(개인) / 사업자등록번호 10자리(법인)

        <TextInput
          id="birthOrBusinessRegistrationNumber"
          style={css.raw({ width: 'full' })}
          inputmode="numeric"
          maxlength={12}
          oninput={(form.fields.birthOrBusinessRegistrationNumber?.length ?? 0) > 6 ? formatBusinessNumber : undefined}
          bind:value={form.fields.birthOrBusinessRegistrationNumber}
        />
      </label>

      {#if form.errors.birthOrBusinessRegistrationNumber}
        <div class={css({ marginTop: '4px', paddingLeft: '4px', fontSize: '12px', color: 'red.500' })}>
          {form.errors.birthOrBusinessRegistrationNumber}
        </div>
      {/if}
    </div>

    <div class={flex({ direction: 'column', gap: '6px', marginY: '12px' })}>
      <Checkbox checked={allChecked} onchange={handleAllCheck} size="sm">
        <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'gray.700' })}>모두 확인하고 동의해요</span>
      </Checkbox>

      <HorizontalDivider color="secondary" />

      {#each agreements as agreement (agreement.name)}
        <Checkbox size="sm" bind:checked={agreementChecks[agreements.indexOf(agreement)]}>
          <span class={flex({ fontSize: '13px', color: 'gray.700' })}>
            <a
              class={css({ textDecoration: 'underline', color: 'gray.900' })}
              href={agreement.url}
              rel="noopener noreferrer"
              target="_blank"
            >
              {agreement.name}
            </a>
            에 동의해요 (필수)
          </span>
        </Checkbox>
      {/each}
    </div>

    <Button disabled={!allChecked} size="lg" type="submit">{redeemCode ? (paymentAmount === 0 ? '등록' : '등록 및 결제') : '결제'}</Button>
  </form>
</Modal>
