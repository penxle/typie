<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Switch } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import PrismCreditIcon from '~icons/typie/prism-credit';
  import { SettingsCard, SettingsRow } from '$lib/components';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import PurchasePrismCreditModal from './PurchasePrismCreditModal.svelte';
  import UpdatePaymentMethodModal from './UpdatePaymentMethodModal.svelte';
  import type { DashboardLayout_PreferenceModal_PrismTab_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_PrismTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_PrismTab_user on User {
        id
        preferences
        prismAccess

        ...DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user

        billingKey {
          id
          name
          type
        }

        prismCredit {
          balance
        }

        prismCreditPurchases {
          id
          pack
          price
          credits
          bonusCredits
          paidAt
          receiptUrl
          refundedAmount
        }
      }
    `),
    () => user$key,
  );

  const [updatePreferences] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_PrismTab_UpdatePreferences_Mutation($input: UpdatePreferencesInput!) {
        updatePreferences(input: $input) {
          id
          preferences
        }
      }
    `),
  );

  const persistedAiOptIn = $derived(user.data.preferences.aiOptIn ?? false);
  let aiOptInOverride = $state<boolean>();
  let updatingAiOptIn = $state(false);
  const aiOptIn = $derived(aiOptInOverride ?? persistedAiOptIn);

  $effect(() => {
    if (!updatingAiOptIn && aiOptInOverride !== undefined && aiOptInOverride === persistedAiOptIn) {
      aiOptInOverride = undefined;
    }
  });

  const updateAiOptIn = async (enabled: boolean) => {
    aiOptInOverride = enabled;
    updatingAiOptIn = true;
    try {
      await updatePreferences(
        { input: { value: { aiOptIn: enabled } } },
        enabled
          ? undefined
          : {
              metadata: {
                cache: {
                  optimisticResponse: {
                    updatePreferences: {
                      id: user.data.id,
                      preferences: { ...user.data.preferences, aiOptIn: enabled },
                    },
                  },
                },
              },
            },
      );
      mixpanel.track('ai_opt_in', { enabled });
    } catch {
      aiOptInOverride = undefined;
      Toast.error('AI 설정을 바꾸지 못했어요. 잠시 후 다시 시도해 주세요');
    } finally {
      updatingAiOptIn = false;
    }
  };

  const handleToggle = () => {
    if (aiOptIn) {
      if (!SubscribeModal.gate('preferences_ai')) {
        return;
      }

      void updateAiOptIn(false);
    } else {
      Dialog.confirm({
        title: 'AI 기능을 활성화하시겠어요?',
        message:
          '사용자의 글은 AI 모델 학습에 절대 사용되지 않으며, 사용자가 요청할 때만 AI가 사용돼요. 언제든지 설정에서 비활성화할 수 있어요.',
        action: 'primary',
        actionLabel: '활성화',
        actionHandler: async () => {
          if (!SubscribeModal.gate('preferences_ai')) {
            return;
          }

          await updateAiOptIn(true);
        },
      });
    }
  };

  let purchaseOpen = $state(false);
  let paymentMethodOpen = $state(false);
  let resumePurchaseAfterBillingKey = false;

  const openPurchase = () => {
    purchaseOpen = true;
  };

  const editBillingKey = () => {
    purchaseOpen = false;
    resumePurchaseAfterBillingKey = true;
    paymentMethodOpen = true;
  };

  $effect(() => {
    if (!paymentMethodOpen && resumePurchaseAfterBillingKey) {
      resumePurchaseAfterBillingKey = false;
      purchaseOpen = true;
    }
  });

  const creditStyle = css.raw({
    display: 'inline-flex',
    alignItems: 'center',
    gap: '3px',
    fontSize: '14px',
    fontWeight: 'semibold',
    fontVariantNumeric: 'tabular-nums',
    color: 'text.brand',
  });
  const historyBonusChipClass = css({
    borderRadius: 'full',
    paddingX: '6px',
    paddingY: '1px',
    fontSize: '11px',
    fontWeight: 'medium',
    whiteSpace: 'nowrap',
    flexShrink: '0',
    color: 'text.subtle',
    backgroundColor: 'surface.muted',
  });
  const statusChipStyle = css.raw({
    borderRadius: 'full',
    paddingX: '8px',
    paddingY: '2px',
    fontSize: '11px',
    fontWeight: 'medium',
    whiteSpace: 'nowrap',
    flexShrink: '0',
  });
</script>

<div class={flex({ direction: 'column', maxWidth: '640px' })}>
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '20px' })}>프리즘</h1>
  </div>

  <div
    class={css({
      padding: '20px',
      borderRadius: '12px',
      backgroundColor: 'surface.subtle',
      borderWidth: '1px',
      borderColor: 'border.default',
    })}
  >
    <h2 class={css({ fontSize: '15px', fontWeight: 'semibold', color: 'text.default', marginBottom: '16px' })}>
      타이피는 사용자의 글을 절대 학습하지 않아요
    </h2>

    <div class={flex({ direction: 'column', gap: '12px', fontSize: '14px', color: 'text.default' })}>
      <p>
        타이피는 사용자의 프라이버시를 최우선으로 생각해요. 사용자가 작성한 글은
        <span class={css({ fontWeight: 'semibold' })}>어떠한 경우에도 AI 모델 학습에 사용되지 않아요.</span>
      </p>

      <ul class={css({ paddingLeft: '20px', listStyleType: 'disc' })}>
        <li class={css({ marginBottom: '8px' })}>
          <span class={css({ fontWeight: 'semibold' })}>학습 금지:</span>
          사용자의 글은 AI 모델 학습이나 개선에 절대 사용되지 않아요.
        </li>
        <li class={css({ marginBottom: '8px' })}>
          <span class={css({ fontWeight: 'semibold' })}>요청 시에만:</span>
          사용자가 요청하지 않는 한 타이피가 임의로 AI를 사용하지 않아요.
        </li>
        <li class={css({ marginBottom: '8px' })}>
          <span class={css({ fontWeight: 'semibold' })}>투명한 처리:</span>
          AI가 언제, 어떻게 사용되는지 사용자가 항상 알 수 있어요.
        </li>
        <li class={css({ marginBottom: '8px' })}>
          <span class={css({ fontWeight: 'semibold' })}>완전한 통제:</span>
          AI 기능은 언제든 끌 수 있고, 비활성화하면 어떤 AI 처리도 일어나지 않아요.
        </li>
        <li>
          <span class={css({ fontWeight: 'semibold' })}>권리 보장:</span>
          타이피는 사용자 창작물에 대한 어떤 권리도 주장하지 않아요.
        </li>
      </ul>
    </div>
  </div>

  <div class={css({ height: '20px' })}></div>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}
        AI 기능 활성화
      {/snippet}
      {#snippet description()}
        활성화하면 AI 피드백 등 타이피가 제공하는 AI 기능을 사용할 수 있어요.
      {/snippet}
      {#snippet value()}
        <Switch
          checked={aiOptIn}
          disabled={updatingAiOptIn}
          onclick={(e) => {
            // The switch is controlled by the confirmed/optimistic preference.
            // Put the input back before its input event so a cancelled enable
            // confirmation cannot leave the native checkbox visually toggled.
            e.currentTarget.checked = aiOptIn;
            handleToggle();
          }}
        />
      {/snippet}
    </SettingsRow>
  </SettingsCard>

  {#if user.data.prismAccess}
    <div class={css({ height: '20px' })}></div>

    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>크레딧</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          보유 크레딧
        {/snippet}
        {#snippet description()}
          프리즘 기능에 쓰는 크레딧이에요.
        {/snippet}
        {#snippet value()}
          <div class={flex({ alignItems: 'center', gap: '12px' })}>
            <span class={css(creditStyle, { color: user.data.prismCredit.balance < 0 ? 'text.danger' : 'text.brand' })}>
              <Icon icon={PrismCreditIcon} size={16} />{comma(user.data.prismCredit.balance)}
            </span>
            <Button onclick={openPurchase} size="sm">충전</Button>
          </div>
        {/snippet}
      </SettingsRow>
    </SettingsCard>

    <div class={css({ height: '12px' })}></div>

    <SettingsCard>
      <SettingsRow vertical>
        {#snippet label()}
          충전 내역
        {/snippet}
        {#snippet value()}
          <div class={css({ height: '1px', backgroundColor: 'border.subtle', marginTop: '8px', marginBottom: '12px' })}></div>
          {#if user.data.prismCreditPurchases.length === 0}
            <span class={css({ fontSize: '13px', color: 'text.faint' })}>아직 충전한 적이 없어요</span>
          {:else}
            <div
              class={css({
                display: 'grid',
                gridTemplateColumns: 'auto 1fr auto auto auto',
                columnGap: '12px',
                rowGap: '8px',
                alignItems: 'center',
                width: 'full',
                fontSize: '13px',
              })}
            >
              {#each user.data.prismCreditPurchases as purchase (purchase.id)}
                {@const refundedFully = purchase.refundedAmount >= purchase.price}
                <div class={css({ display: 'contents' })}>
                  <span class={css({ color: 'text.subtle', whiteSpace: 'nowrap' })}>
                    {purchase.paidAt ? dayjs(purchase.paidAt).formatAsDate() : ''}
                  </span>
                  <span class={css({ display: 'inline-flex', alignItems: 'center', gap: '6px', minWidth: '0' })}>
                    <span class={css({ whiteSpace: 'nowrap', color: 'text.default' })}>{comma(purchase.credits)} 크레딧</span>
                    {#if purchase.bonusCredits > 0}
                      <span class={historyBonusChipClass}>보너스 +{comma(purchase.bonusCredits)}</span>
                    {/if}
                  </span>
                  <span class={css({ justifySelf: 'end' })}>
                    {#if purchase.refundedAmount > 0}
                      <span class={css(statusChipStyle, { color: 'text.subtle', backgroundColor: 'surface.muted' })}>
                        {refundedFully ? '환불됨' : '부분 환불'}
                      </span>
                    {:else}
                      <span class={css(statusChipStyle, { color: 'text.success', backgroundColor: 'accent.success.subtle' })}>완료</span>
                    {/if}
                  </span>
                  <span
                    class={css({
                      justifySelf: 'end',
                      fontSize: '14px',
                      fontWeight: 'semibold',
                      fontVariantNumeric: 'tabular-nums',
                      whiteSpace: 'nowrap',
                      color: refundedFully ? 'text.faint' : 'text.default',
                      textDecoration: refundedFully ? 'line-through' : 'none',
                    })}
                  >
                    {comma(purchase.price)}원
                  </span>
                  <span class={css({ justifySelf: 'end' })}>
                    {#if purchase.receiptUrl}
                      <a
                        class={css({ fontSize: '12px', color: 'text.subtle', textDecoration: 'underline' })}
                        href={purchase.receiptUrl}
                        rel="noopener noreferrer"
                        target="_blank"
                      >
                        영수증
                      </a>
                    {/if}
                  </span>
                </div>
              {/each}
            </div>
          {/if}
        {/snippet}
      </SettingsRow>
    </SettingsCard>

    <div
      class={css({
        marginTop: '12px',
        borderRadius: '8px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
        padding: '20px',
        backgroundColor: 'surface.default',
      })}
    >
      <ul class={flex({ direction: 'column', gap: '12px' })}>
        <li class={flex({ gap: '8px', alignItems: 'flex-start' })}>
          <span class={css({ color: 'text.disabled', fontSize: '13px', flexShrink: 0, marginTop: '2px' })}>•</span>
          <span class={css({ fontSize: '13px', color: 'text.faint', lineHeight: '[1.6]' })}>
            사용하지 않은 크레딧은 충전 후 7일 안에 전액 환불받을 수 있어요.
          </span>
        </li>
        <li class={flex({ gap: '8px', alignItems: 'flex-start' })}>
          <span class={css({ color: 'text.disabled', fontSize: '13px', flexShrink: 0, marginTop: '2px' })}>•</span>
          <span class={css({ fontSize: '13px', color: 'text.faint', lineHeight: '[1.6]' })}>
            크레딧을 사용했거나 7일이 지났다면 남은 유상 크레딧의 90%를 돌려드려요.
          </span>
        </li>
        <li class={flex({ gap: '8px', alignItems: 'flex-start' })}>
          <span class={css({ color: 'text.disabled', fontSize: '13px', flexShrink: 0, marginTop: '2px' })}>•</span>
          <span class={css({ fontSize: '13px', color: 'text.faint', lineHeight: '[1.6]' })}>환불은 고객센터에 문의해주세요.</span>
        </li>
      </ul>
    </div>
  {/if}
</div>

<UpdatePaymentMethodModal user$key={user.data} bind:open={paymentMethodOpen} />
<PurchasePrismCreditModal
  balance={user.data.prismCredit.balance}
  billingKey={user.data.billingKey ?? null}
  onEditBillingKey={editBillingKey}
  userId={user.data.id}
  bind:open={purchaseOpen}
/>
