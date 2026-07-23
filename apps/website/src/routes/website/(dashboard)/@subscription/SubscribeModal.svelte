<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { PlanId } from '@typie/lib/const';
  import { PlanAvailability, PlanInterval, SubscriptionState } from '@typie/lib/enums';
  import { TypieError } from '@typie/lib/errors';
  import { cardSchema } from '@typie/lib/validation';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import { PLAN_FEATURES } from '@typie/ui/constants';
  import { createForm, FormError } from '@typie/ui/form';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { z } from 'zod';
  import InfoIcon from '~icons/lucide/info';
  import LockIcon from '~icons/lucide/lock';
  import MoonIcon from '~icons/lucide/moon';
  import { fb } from '$lib/analytics';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import SubscriptionCelebrationModal from '../SubscriptionCelebrationModal.svelte';
  import BillingCardForm from './BillingCardForm.svelte';
  import { SubscribeModal } from './subscribe-modal.svelte';
  import type { DashboardLayout_SubscribeModal_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_SubscribeModal_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_SubscribeModal_user on User {
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

        nextSubscription {
          id
        }
      }
    `),
    () => user$key,
  );

  const [updateBillingKey] = createMutation(
    graphql(`
      mutation DashboardLayout_SubscribeModal_UpdateBillingKey_Mutation($input: UpdateBillingKeyInput!) {
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
      mutation DashboardLayout_SubscribeModal_SubscribePlanWithBillingKey_Mutation($input: SubscribePlanWithBillingKeyInput!) {
        subscribePlanWithBillingKey(input: $input) {
          id
          state

          user {
            id
            ...DashboardLayout_PreferenceModal_BillingTab_user
            ...DashboardLayout_Profile_user
            ...DashboardLayout_SubscribeModal_user
          }
        }
      }
    `),
  );

  let step = $state<'plan' | 'payment'>('plan');
  let stepDirection = $state(1);
  let stepContainerEl = $state<HTMLDivElement>();
  let stepHeightFrom: number | undefined;

  const goToStep = (next: 'plan' | 'payment') => {
    stepHeightFrom = stepContainerEl?.offsetHeight;
    stepDirection = next === 'payment' ? 1 : -1;
    step = next;
  };

  const stepIntro = (node: HTMLElement) => {
    const el = stepContainerEl;
    const from = stepHeightFrom;
    stepHeightFrom = undefined;

    let to = 0;
    let dy = 0;

    if (el && from !== undefined) {
      el.style.height = '';
      to = el.offsetHeight;
      dy = (from - to) / 2;
    }

    return {
      duration: 250,
      easing: cubicOut,
      tick: (t: number, u: number) => {
        node.style.opacity = String(t);
        node.style.transform = `translate(${24 * stepDirection * u}px, ${dy * u}px)`;

        if (el && from !== undefined && from !== to) {
          el.style.height = t === 1 ? '' : `${from + (to - from) * t}px`;
        }

        if (t === 1) {
          node.style.opacity = '';
          node.style.transform = '';
        }
      },
    };
  };
  let interval = $state<PlanInterval>(PlanInterval.MONTHLY);
  let submitError = $state<string | null>(null);
  let isEditingCard = $state(false);
  let celebrationModalOpen = $state(false);
  let scheduledCelebration = $state(false);

  const isTrial = $derived(user.data.subscription?.plan.availability === PlanAvailability.TRIAL);
  const hasScheduled = $derived(Boolean(user.data.nextSubscription));
  const firstBillingDate = $derived(user.data.subscription ? dayjs(user.data.subscription.expiresAt).format('M월 D일') : null);

  const planFee = $derived(interval === PlanInterval.MONTHLY ? 2900 : 29_000);
  const creditDiscount = $derived(Math.min(user.data.credit, planFee));
  const finalAmount = $derived(planFee - creditDiscount);

  const useExistingCard = $derived(Boolean(user.data.billingKey) && !isEditingCard);

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

      if (!useExistingCard) {
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

      const planId =
        interval === PlanInterval.YEARLY ? PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY : PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY;
      const result = await subscribePlanWithBillingKey({ input: { planId } });
      const scheduled = result.subscribePlanWithBillingKey.state === SubscriptionState.WILL_ACTIVATE;

      mixpanel.track('enroll_plan', { planId, scheduled });
      if (!scheduled) {
        const value = interval === PlanInterval.YEARLY ? '29000.00' : '2900.00';
        fb.track('Subscribe', { value, currency: 'KRW', predicted_ltv: value });
      }

      scheduledCelebration = scheduled;
      SubscribeModal.close();
      celebrationModalOpen = true;
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
    if (!SubscribeModal.open) {
      untrack(() => {
        step = 'plan';
        interval = PlanInterval.MONTHLY;
        submitError = null;
        isEditingCard = false;
        form.reset();
      });
    }
  });
</script>

{#if SubscribeModal.open}
  <Modal
    style={css.raw({ padding: '0', maxWidth: '640px' })}
    closable={!form.state.isLoading}
    onclose={() => SubscribeModal.close()}
    open={true}
  >
    <div bind:this={stepContainerEl} class={css({ overflow: 'hidden' })}>
      {#key step}
        <div in:stepIntro>
          {#if step === 'plan'}
            <div class={css({ paddingX: '32px', paddingTop: '32px' })}>
              <h2 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'text.default' })}>타이피 구독하기</h2>
              <p class={css({ marginTop: '6px', fontSize: '14px', color: 'text.muted' })}>글쓰기에 필요한 모든 기능을 제한 없이</p>
            </div>

            <div class={css({ paddingX: '32px', paddingTop: '16px' })}>
              <div
                class={flex({
                  alignItems: 'center',
                  gap: '12px',
                  borderRadius: '10px',
                  borderWidth: '1px',
                  borderColor: 'border.subtle',
                  paddingX: '16px',
                  paddingY: '12px',
                  backgroundColor: 'surface.subtle',
                })}
              >
                <Icon style={css.raw({ flexShrink: '0', color: 'text.subtle' })} icon={MoonIcon} size={18} />
                <div>
                  <p class={flex({ alignItems: 'center', gap: '4px', fontSize: '13px', fontWeight: 'semibold', color: 'text.default' })}>
                    쉬는 달엔 결제도 쉬어요
                    <span
                      class={flex({ alignItems: 'center' })}
                      use:tooltip={{
                        message:
                          '한 번도 사용하지 않은 달이나 해에는 구독료가 발생하지 않아요.\n결제를 건너뛴 동안에도 작성한 글은 그대로 남아 있어요.',
                        placement: 'top-start',
                      }}
                    >
                      <Icon style={css.raw({ color: 'text.faint' })} icon={InfoIcon} size={12} />
                    </span>
                  </p>
                  <p class={css({ marginTop: '2px', fontSize: '12px', color: 'text.faint' })}>쓰지 않은 달의 결제는 자동으로 건너뛰어요</p>
                </div>
              </div>
            </div>

            <div class={flex({ gap: '28px', paddingTop: '20px', paddingX: '32px', paddingBottom: '32px' })}>
              <ul
                class={flex({
                  flex: '1',
                  flexDirection: 'column',
                  gap: '14px',
                  borderRightWidth: '1px',
                  borderColor: 'border.subtle',
                  paddingRight: '24px',
                  fontSize: '14px',
                  fontWeight: 'medium',
                  color: 'text.subtle',
                })}
              >
                {#each PLAN_FEATURES.full as feature, index (index)}
                  <li class={flex({ alignItems: 'center', gap: '10px' })}>
                    <Icon style={css.raw({ color: 'text.faint' })} icon={feature.icon} size={16} />
                    <span>{feature.label}</span>
                  </li>
                {/each}
              </ul>

              <div class={flex({ flex: '1', flexDirection: 'column', gap: '10px' })}>
                {#if hasScheduled}
                  <div
                    class={css({
                      borderRadius: '8px',
                      borderWidth: '1px',
                      borderColor: 'border.subtle',
                      padding: '16px',
                      fontSize: '13px',
                      color: 'text.muted',
                      backgroundColor: 'surface.default',
                    })}
                  >
                    구독이 예약되어 있어요. 무료 체험이 끝나면 자동으로 시작돼요.
                  </div>
                {:else}
                  <button
                    class={css({
                      padding: '16px',
                      borderRadius: '8px',
                      borderWidth: '1px',
                      borderColor: interval === PlanInterval.MONTHLY ? 'accent.brand.default' : 'border.subtle',
                      backgroundColor: interval === PlanInterval.MONTHLY ? 'accent.brand.subtle' : 'surface.default',
                      cursor: 'pointer',
                      transition: 'common',
                      textAlign: 'left',
                      _hover: { borderColor: interval === PlanInterval.MONTHLY ? 'accent.brand.default' : 'border.default' },
                    })}
                    onclick={() => (interval = PlanInterval.MONTHLY)}
                    type="button"
                  >
                    <div class={flex({ justify: 'space-between', alignItems: 'center' })}>
                      <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>월간</span>
                      <span class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>2,900원</span>
                    </div>
                    <div class={css({ fontSize: '13px', color: 'text.subtle', marginTop: '6px' })}>매월 결제</div>
                  </button>

                  <button
                    class={css({
                      padding: '16px',
                      borderRadius: '8px',
                      borderWidth: '1px',
                      borderColor: interval === PlanInterval.YEARLY ? 'accent.brand.default' : 'border.subtle',
                      backgroundColor: interval === PlanInterval.YEARLY ? 'accent.brand.subtle' : 'surface.default',
                      cursor: 'pointer',
                      transition: 'common',
                      textAlign: 'left',
                      _hover: { borderColor: interval === PlanInterval.YEARLY ? 'accent.brand.default' : 'border.default' },
                    })}
                    onclick={() => (interval = PlanInterval.YEARLY)}
                    type="button"
                  >
                    <div class={flex({ justify: 'space-between', alignItems: 'center' })}>
                      <div class={flex({ alignItems: 'center', gap: '6px' })}>
                        <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>연간</span>
                        <span
                          class={css({
                            borderRadius: 'full',
                            paddingX: '6px',
                            paddingY: '1px',
                            fontSize: '10px',
                            fontWeight: 'semibold',
                            color: 'text.bright',
                            backgroundColor: 'accent.brand.default',
                          })}
                        >
                          2개월 무료
                        </span>
                      </div>
                      <span class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>29,000원</span>
                    </div>
                    <div class={css({ fontSize: '13px', color: 'text.subtle', marginTop: '6px' })}>
                      매년 결제 · <span class={css({ color: 'accent.brand.default', fontWeight: 'medium' })}>월 2,416원</span>
                    </div>
                  </button>

                  <div class={flex({ flexDirection: 'column', gap: '10px', marginTop: 'auto', paddingTop: '14px' })}>
                    <p class={css({ fontSize: '12px', color: 'text.faint', textAlign: 'center' })}>언제든 해지할 수 있어요.</p>

                    <Button style={css.raw({ width: 'full' })} onclick={() => goToStep('payment')} size="lg">구독하기</Button>
                  </div>
                {/if}
              </div>
            </div>
          {:else}
            <div class={css({ paddingX: '32px', paddingTop: '24px' })}>
              <button
                class={css({ fontSize: '13px', color: 'text.faint', cursor: 'pointer', _hover: { color: 'text.muted' } })}
                onclick={() => goToStep('plan')}
                type="button"
              >
                ← 뒤로
              </button>
            </div>

            <form class={flex({ gap: '28px', paddingTop: '20px', paddingX: '32px', paddingBottom: '32px' })} onsubmit={form.handleSubmit}>
              <div class={flex({ flexDirection: 'column', gap: '12px', width: '232px', flexShrink: '0' })}>
                <div
                  class={css({
                    borderRadius: '8px',
                    borderWidth: '1px',
                    borderColor: 'border.subtle',
                    padding: '16px',
                    fontSize: '13px',
                    backgroundColor: 'surface.default',
                  })}
                >
                  <div class={flex({ justify: 'space-between' })}>
                    <span class={css({ color: 'text.subtle' })}>플랜</span>
                    <span class={css({ color: 'text.default', fontWeight: 'medium' })}>
                      {interval === PlanInterval.YEARLY ? '연간' : '월간'} · FULL ACCESS
                    </span>
                  </div>

                  <div class={flex({ justify: 'space-between', marginTop: '8px' })}>
                    <span class={css({ color: 'text.subtle' })}>플랜 금액</span>
                    <span class={css({ color: 'text.default' })}>{comma(planFee)}원</span>
                  </div>

                  {#if user.data.credit > 0}
                    <div class={flex({ justify: 'space-between', marginTop: '8px' })}>
                      <span class={css({ color: 'text.subtle' })}>보유 크레딧</span>
                      <span class={css({ color: 'text.default' })}>{comma(user.data.credit)}원</span>
                    </div>
                  {/if}

                  {#if !isTrial && creditDiscount > 0}
                    <div class={flex({ justify: 'space-between', marginTop: '8px' })}>
                      <span class={css({ color: 'text.subtle' })}>크레딧 차감</span>
                      <span class={css({ color: 'accent.brand.default', fontWeight: 'medium' })}>-{comma(creditDiscount)}원</span>
                    </div>
                  {/if}

                  <div class={css({ marginTop: '12px', paddingTop: '12px', borderTopWidth: '1px', borderColor: 'border.subtle' })}>
                    {#if isTrial}
                      <div class={flex({ justify: 'space-between', fontSize: '14px', fontWeight: 'semibold' })}>
                        <span class={css({ color: 'text.default' })}>오늘 결제 금액</span>
                        <span class={css({ color: 'text.default' })}>0원</span>
                      </div>
                      <div class={flex({ justify: 'space-between', marginTop: '6px', fontSize: '12px' })}>
                        <span class={css({ color: 'text.subtle' })}>첫 결제일</span>
                        {#if user.data.credit > 0}
                          <span class={flex({ alignItems: 'center', gap: '3px', color: 'text.default' })}>
                            {firstBillingDate} · 예상 {comma(finalAmount)}원
                            <span
                              class={flex({ alignItems: 'center' })}
                              use:tooltip={{
                                message:
                                  '현재 보유 크레딧 기준 예상 금액이에요.\n실제 결제 금액은 결제 시점에 남아 있는 크레딧에 따라 달라질 수 있어요.',
                                placement: 'top-start',
                              }}
                            >
                              <Icon style={css.raw({ color: 'text.faint' })} icon={InfoIcon} size={12} />
                            </span>
                          </span>
                        {:else}
                          <span class={css({ color: 'text.default' })}>{firstBillingDate} · {comma(planFee)}원</span>
                        {/if}
                      </div>
                    {:else}
                      <div class={flex({ justify: 'space-between', fontSize: '14px', fontWeight: 'semibold' })}>
                        <span class={css({ color: 'text.default' })}>오늘 결제 금액</span>
                        <span class={css({ color: 'text.default' })}>{comma(finalAmount)}원</span>
                      </div>
                    {/if}
                  </div>
                </div>

                {#if isTrial}
                  <div
                    class={css({
                      borderRadius: '8px',
                      padding: '14px',
                      fontSize: '12px',
                      color: 'text.brand',
                      backgroundColor: 'accent.brand.subtle',
                      lineHeight: '[1.6]',
                    })}
                  >
                    오늘은 결제되지 않아요. 무료 체험이 끝나는 {firstBillingDate}에 첫 결제돼요. 그 전에 언제든 취소할 수 있어요.
                  </div>
                {/if}
              </div>

              <div class={flex({ flex: '1', flexDirection: 'column', gap: '16px' })}>
                {#if useExistingCard}
                  <div
                    class={flex({
                      justify: 'space-between',
                      alignItems: 'center',
                      borderRadius: '8px',
                      borderWidth: '1px',
                      borderColor: 'border.subtle',
                      padding: '14px',
                      backgroundColor: 'surface.default',
                    })}
                  >
                    <span class={css({ fontSize: '14px', color: 'text.default' })}>{user.data.billingKey?.name}</span>
                    <Button onclick={() => (isEditingCard = true)} size="sm" variant="secondary">변경</Button>
                  </div>
                  <BillingCardForm errors={form.errors} fields={form.fields} showCardFields={false} />
                {:else}
                  {#if isEditingCard && user.data.billingKey}
                    <button
                      class={css({
                        alignSelf: 'flex-end',
                        fontSize: '13px',
                        fontWeight: 'medium',
                        color: 'text.faint',
                        cursor: 'pointer',
                        _hover: { color: 'text.muted' },
                      })}
                      onclick={() => (isEditingCard = false)}
                      type="button"
                    >
                      기존 카드 사용하기
                    </button>
                  {/if}
                  <BillingCardForm errors={form.errors} fields={form.fields} />
                {/if}

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

                <div class={flex({ flexDirection: 'column', gap: '10px', marginTop: 'auto' })}>
                  <Button style={css.raw({ width: 'full' })} loading={form.state.isLoading} size="lg" type="submit">
                    {#if isTrial}
                      {firstBillingDate} 결제 예약하기
                    {:else if finalAmount === 0}
                      구독 시작하기
                    {:else}
                      {comma(finalAmount)}원 결제하기
                    {/if}
                  </Button>

                  <div class={flex({ alignItems: 'center', justify: 'center', gap: '5px', fontSize: '12px', color: 'text.faint' })}>
                    <Icon icon={LockIcon} size={12} />
                    <span>카드 정보는 암호화되어 안전하게 전송돼요.</span>
                  </div>
                </div>
              </div>
            </form>
          {/if}
        </div>
      {/key}
    </div>
  </Modal>
{/if}

<SubscriptionCelebrationModal
  message={scheduledCelebration ? '무료 체험이 끝나면 자동으로 결제되고 플랜이 시작돼요.' : '타이피의 모든 기능을 자유롭게 이용해보세요.'}
  title={scheduledCelebration ? '구독이 예약됐어요!' : '구독이 시작됐어요!'}
  bind:open={celebrationModalOpen}
/>
