<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import * as Sentry from '@sentry/sveltekit';
  import { BillingKeyType } from '@typie/lib/enums';
  import { TypieError } from '@typie/lib/errors';
  import { cardSchema } from '@typie/lib/validation';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import { z } from 'zod';
  import { fb } from '$lib/analytics';
  import { cache } from '$lib/graphql';
  import { requestKakaoPayBillingKey } from '$lib/portone';
  import { graphql } from '$mearie';
  import BillingCardForm from '../@subscription/BillingCardForm.svelte';
  import PaymentAgreements from '../@subscription/PaymentAgreements.svelte';
  import PaymentMethodSelector from '../@subscription/PaymentMethodSelector.svelte';
  import type { DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user$key } from '$mearie';

  type Props = {
    open: boolean;
    user$key: DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user$key;
  };

  let { open = $bindable(), user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user on User {
        id
        usableBillingKeyTypes

        billingKey {
          id
          name
          type
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
          type
          createdAt
        }
      }
    `),
  );

  const [updateBillingKeyWithEasyPay] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_UpdateBillingKeyWithEasyPay_Mutation(
        $input: UpdateBillingKeyWithEasyPayInput!
      ) {
        updateBillingKeyWithEasyPay(input: $input) {
          id
          name
          type
          createdAt
        }
      }
    `),
  );

  let submitError = $state<string | null>(null);
  let method = $state<BillingKeyType>(BillingKeyType.CARD);

  const subscriptionUnsupportedMessages: Record<BillingKeyType, string> = {
    [BillingKeyType.CARD]: '구독 중인 플랜은 신용·체크카드로 결제할 수 없어요.',
    [BillingKeyType.KAKAOPAY]: '연간 플랜은 카카오페이로 결제할 수 없어요. 월간 플랜으로 전환하면 카카오페이를 사용할 수 있어요.',
  };

  const disabledMethods = $derived(
    Object.fromEntries(
      Object.values(BillingKeyType)
        .filter((type) => !user.data.usableBillingKeyTypes.includes(type))
        .map((type) => [type, subscriptionUnsupportedMessages[type]]),
    ),
  );

  $effect(() => {
    if (Object.hasOwn(disabledMethods, method)) {
      method = BillingKeyType.CARD;
    }
  });

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

      if (method === BillingKeyType.KAKAOPAY) {
        const result = await requestKakaoPayBillingKey({ userId: user.data.id });

        if (result.status === 'canceled') {
          return;
        }

        if (result.status === 'failed') {
          Sentry.captureMessage('kakaopay billing key issue failed', {
            level: 'warning',
            extra: { code: result.code, message: result.message, pgCode: result.pgCode, pgMessage: result.pgMessage },
          });

          submitError = '카카오페이 결제 수단 등록에 실패했어요.';
          return;
        }

        await updateBillingKeyWithEasyPay({ input: { billingKey: result.billingKey } });
      } else {
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
      }

      cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'billingKey' });
      mixpanel.track('update_payment_billing_key');
      fb.track('AddPaymentInfo');

      Toast.success(user.data.billingKey ? '결제 수단이 변경되었어요.' : '결제 수단이 등록되었어요.');
      open = false;
    },
    onError: (error) => {
      const isCard = method === BillingKeyType.CARD;
      const errorMessages: Record<string, string> = {
        billing_key_issue_failed: isCard
          ? '결제 키 발급에 실패했어요. 카드 정보를 확인해주세요.'
          : '결제 수단 등록에 실패했어요. 다시 시도해 주세요.',
        plan_interval_not_supported: '선택한 결제 수단으로는 이 결제 주기를 이용할 수 없어요.',
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
        submitError = null;
        method = BillingKeyType.CARD;
      });
    }
  });
</script>

<Modal style={css.raw({ padding: '24px', maxWidth: '480px' })} closable={!form.state.isLoading} bind:open>
  <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>
    {user.data.billingKey ? '결제 수단 변경' : '결제 수단 등록'}
  </h2>

  <form class={flex({ direction: 'column', gap: '24px' })} onsubmit={form.handleSubmit}>
    <PaymentMethodSelector disabled={disabledMethods} bind:method />

    {#if method === BillingKeyType.CARD}
      <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>카드 정보</div>
      <BillingCardForm errors={form.errors} fields={form.fields} />
    {/if}

    <PaymentAgreements
      error={form.errors.agreementsAccepted}
      {method}
      onchange={(accepted) => (form.fields.agreementsAccepted = accepted)}
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
      {user.data.billingKey ? '변경하기' : '등록하기'}
    </Button>
  </form>
</Modal>
