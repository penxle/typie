<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import PrismCreditIcon from '~icons/typie/prism-credit';
  import { SettingsCard, SettingsRow } from '$lib/components';
  import { graphql } from '$mearie';
  import PurchasePrismCreditModal from './PurchasePrismCreditModal.svelte';
  import UpdatePaymentMethodModal from './UpdatePaymentMethodModal.svelte';
  import type { DashboardLayout_PreferenceModal_PrismCreditsTab_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_PrismCreditsTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_PrismCreditsTab_user on User {
        id

        ...DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user

        billingKey {
          id
          name
          type
        }

        prismCredit {
          balance
          expiringAmount
          expiresAt
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
    color: 'text.default',
  });
  const historyBonusChipClass = css({
    borderRadius: 'full',
    paddingX: '6px',
    paddingY: '1px',
    fontSize: '11px',
    fontWeight: 'medium',
    whiteSpace: 'nowrap',
    flexShrink: '0',
    color: 'text.muted',
    backgroundColor: 'surface.inset',
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
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>크레딧</h1>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}보유 크레딧{/snippet}
      {#snippet description()}프리즘 기능에 쓰는 크레딧이에요.{/snippet}
      {#snippet value()}
        <div class={flex({ alignItems: 'center', gap: '12px' })}>
          <span class={css(creditStyle, { color: user.data.prismCredit.balance < 0 ? 'danger.default' : 'text.default' })}>
            <Icon icon={PrismCreditIcon} size={16} />{comma(user.data.prismCredit.balance)}
          </span>
          <Button onclick={openPurchase} size="sm">충전</Button>
        </div>
      {/snippet}
    </SettingsRow>
  </SettingsCard>

  {#if user.data.prismCredit.expiringAmount > 0 && user.data.prismCredit.expiresAt}
    <div
      class={css({
        marginTop: '12px',
        borderRadius: '8px',
        borderWidth: '1px',
        borderColor: 'border.hairline',
        paddingX: '20px',
        paddingY: '16px',
        backgroundColor: 'surface.canvas',
      })}
    >
      <span class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]' })}>
        보유 크레딧 중 {comma(user.data.prismCredit.expiringAmount)} 크레딧은 {dayjs(user.data.prismCredit.expiresAt)
          .kst()
          .subtract(1, 'day')
          .formatAsDate()}까지 사용할 수 있어요.
      </span>
    </div>
  {/if}

  <div class={css({ height: '12px' })}></div>

  <SettingsCard>
    <SettingsRow vertical>
      {#snippet label()}충전 내역{/snippet}
      {#snippet value()}
        <div class={css({ height: '1px', backgroundColor: 'border.hairline', marginTop: '8px', marginBottom: '12px' })}></div>
        {#if user.data.prismCreditPurchases.length === 0}
          <span class={css({ fontSize: '13px', color: 'text.hint' })}>아직 충전한 적이 없어요</span>
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
                <span class={css({ color: 'text.muted', whiteSpace: 'nowrap' })}>
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
                    <span class={css(statusChipStyle, { color: 'text.muted', backgroundColor: 'surface.inset' })}>
                      {refundedFully ? '환불됨' : '부분 환불'}
                    </span>
                  {:else}
                    <span class={css(statusChipStyle, { color: 'text.on.success.subtle', backgroundColor: 'success.subtle' })}>완료</span>
                  {/if}
                </span>
                <span
                  class={css({
                    justifySelf: 'end',
                    fontSize: '14px',
                    fontWeight: 'semibold',
                    fontVariantNumeric: 'tabular-nums',
                    whiteSpace: 'nowrap',
                    color: refundedFully ? 'text.hint' : 'text.default',
                    textDecoration: refundedFully ? 'line-through' : 'none',
                  })}
                >
                  {comma(purchase.price)}원
                </span>
                <span class={css({ justifySelf: 'end' })}>
                  {#if purchase.receiptUrl}
                    <a
                      class={css({ fontSize: '12px', color: 'text.default', textDecoration: 'underline' })}
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
      borderColor: 'border.hairline',
      padding: '20px',
      backgroundColor: 'surface.canvas',
    })}
  >
    <ul class={flex({ direction: 'column', gap: '12px' })}>
      <li class={flex({ gap: '8px', alignItems: 'flex-start' })}>
        <span class={css({ color: 'text.muted', fontSize: '13px', flexShrink: 0, marginTop: '2px' })}>•</span>
        <span class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]' })}>
          사용하지 않은 크레딧은 충전 후 7일 안에 전액 환불받을 수 있어요.
        </span>
      </li>
      <li class={flex({ gap: '8px', alignItems: 'flex-start' })}>
        <span class={css({ color: 'text.muted', fontSize: '13px', flexShrink: 0, marginTop: '2px' })}>•</span>
        <span class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]' })}>
          크레딧을 사용했거나 7일이 지났다면 남은 유상 크레딧의 90%를 돌려드려요.
        </span>
      </li>
      <li class={flex({ gap: '8px', alignItems: 'flex-start' })}>
        <span class={css({ color: 'text.muted', fontSize: '13px', flexShrink: 0, marginTop: '2px' })}>•</span>
        <span class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]' })}>환불은 고객센터에 문의해주세요.</span>
      </li>
    </ul>
  </div>
</div>

<UpdatePaymentMethodModal user$key={user.data} bind:open={paymentMethodOpen} />
<PurchasePrismCreditModal
  balance={user.data.prismCredit.balance}
  billingKey={user.data.billingKey ?? null}
  onEditBillingKey={editBillingKey}
  userId={user.data.id}
  bind:open={purchaseOpen}
/>
