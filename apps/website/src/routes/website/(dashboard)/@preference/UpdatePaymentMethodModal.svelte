<script lang="ts">
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { cardSchema } from '@/validation';
  import { fragment, graphql } from '$graphql';
  import { Button, Checkbox, HorizontalDivider, Modal, SegmentButtons, TextInput } from '$lib/components';
  import { createForm } from '$lib/form';
  import { Toast } from '$lib/notification';
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

        plan {
          id
        }
      }
    `),
  );

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

  const form = createForm({
    schema: z.object({
      cardNumber: cardSchema.cardNumber,
      expiryDate: cardSchema.expiryDate,
      birthOrBusinessRegistrationNumber: cardSchema.birthOrBusinessRegistrationNumber,
      passwordTwoDigits: cardSchema.passwordTwoDigits,
    }),
    onSubmit: async (data) => {
      try {
        await updatePaymentBillingKey({
          birthOrBusinessRegistrationNumber: data.birthOrBusinessRegistrationNumber,
          cardNumber: data.cardNumber,
          expiryDate: data.expiryDate,
          passwordTwoDigits: data.passwordTwoDigits,
        });

        if (!$user.plan) {
          await enrollPlan({ billingCycle, planId: 'PL0PLUS' });
        }

        open = false;
      } catch (err) {
        const errorMessages: Record<string, string> = {
          billing_key_issue_failed: '결제 키 발급에 실패했습니다. 카드 정보를 다시 확인해주세요.',
          plan_already_enrolled: '이미 결제 정보가 등록되어 있습니다.',
          payment_failed: '결제에 실패했습니다. 카드 정보를 다시 확인해주세요.',
        };

        if (err instanceof TypieError) {
          const message = errorMessages[err.code] || err.code;
          Toast.error(message);
        }
      }
    },
  });

  const agreements = [
    { name: '타이피 결제 이용약관', url: 'https://help.typie.co/legal/terms' },
    { name: 'NICEPAY 전자금융거래 기본약관', url: 'https://www.nicepay.co.kr/cs/terms/policy1.do' },
  ];

  let agreementChecks = $state(agreements.map(() => false));
  const allChecked = $derived(agreementChecks.every(Boolean));

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

<Modal style={css.raw({ gap: '24px', padding: '20px' })} bind:open>
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

    <div
      class={css({
        borderRadius: '4px',
        padding: '12px',
        fontSize: '15px',
        fontWeight: 'medium',
        backgroundColor: 'gray.100',
      })}
    >
      결제 금액: {billingCycle === 'MONTHLY' ? '4,900' : '49,000'}원
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

    <div class={flex({ direction: 'column', gap: '8px', marginY: '12px' })}>
      <Checkbox checked={allChecked} onchange={handleAllCheck} size="sm">
        <span class={css({ color: 'gray.700' })}>모두 확인하고 동의합니다</span>
      </Checkbox>

      <HorizontalDivider color="secondary" />

      {#each agreements as agreement (agreement.name)}
        <Checkbox size="sm" bind:checked={agreementChecks[agreements.indexOf(agreement)]}>
          <span class={flex({ fontSize: '14px', color: 'gray.700' })}>
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

    <Button disabled={!allChecked} size="lg" type="submit">결제</Button>
  </form>
</Modal>
