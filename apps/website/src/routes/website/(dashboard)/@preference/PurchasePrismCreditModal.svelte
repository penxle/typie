<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { BillingKeyType } from '@typie/lib/enums';
  import { TypieError } from '@typie/lib/errors';
  import { PRISM_CREDIT_PACKS } from '@typie/prism';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import ClockIcon from '~icons/lucide/clock';
  import LockIcon from '~icons/lucide/lock';
  import PrismCreditIcon from '~icons/typie/prism-credit';
  import KakaoPayLogo from '$assets/icons/kakaopay.svg?component';
  import { cache } from '$lib/graphql';
  import { unwrapError } from '$lib/graphql/error';
  import { graphql } from '$mearie';
  import PaymentAgreements from '../@subscription/PaymentAgreements.svelte';
  import type { PrismCreditPackName } from '@typie/prism';

  type Props = {
    open: boolean;
    userId: string;
    balance: number;
    billingKey: { name: string; type: BillingKeyType } | null;
    onEditBillingKey: () => void;
  };

  let { open = $bindable(), userId, balance, billingKey, onEditBillingKey }: Props = $props();

  const [purchasePrismCreditPack] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_PrismCreditsTab_PurchasePrismCreditModal_Purchase_Mutation(
        $input: PurchasePrismCreditPackInput!
      ) {
        purchasePrismCreditPack(input: $input) {
          id
        }
      }
    `),
  );

  let picked = $state<PrismCreditPackName>('P100');
  let resultOpen = $state(false);
  let resultKind = $state<'success' | 'pending'>('success');
  let busy = $state(false);
  let agreementsAccepted = $state(false);
  let agreementsError = $state<string | undefined>();
  let submitError = $state('');
  let charged = $state({ credits: 0, bonus: 0, after: 0 });

  const chosen = $derived(PRISM_CREDIT_PACKS.find((grid) => grid.pack === picked) ?? PRISM_CREDIT_PACKS[0]);

  $effect(() => {
    if (!open) {
      busy = false;
      submitError = '';
      agreementsError = undefined;
    }
  });

  const submit = async () => {
    if (busy || billingKey === null) return;

    if (!agreementsAccepted) {
      agreementsError = '약관에 동의해주세요.';
      return;
    }

    agreementsError = undefined;
    submitError = '';
    busy = true;
    charged = { credits: chosen.credits, bonus: chosen.bonus, after: balance + chosen.credits + chosen.bonus };

    try {
      await purchasePrismCreditPack({ input: { pack: picked } });
      cache.invalidate({ __typename: 'User', id: userId, $field: 'prismCreditPurchases' });
      mixpanel.track('purchase_prism_credit_pack', { pack: picked });
      resultKind = 'success';
      open = false;
      resultOpen = true;
    } catch (err) {
      const error = unwrapError(err);
      const code = error instanceof TypieError ? error.code : null;

      if (code === 'payment_pending') {
        resultKind = 'pending';
        open = false;
        resultOpen = true;
        return;
      }

      if (code === 'billing_key_required') {
        submitError = '결제 수단이 필요해요. 결제 수단을 등록한 뒤 다시 시도해 주세요';
      } else if (code === 'payment_failed') {
        submitError = '결제에 실패했어요. 결제 수단을 확인해 주세요';
      } else {
        submitError = '결제하지 못했어요. 잠시 후 다시 시도해 주세요';
      }
    } finally {
      busy = false;
    }
  };

  const packRowStyle = css.raw({
    paddingX: '16px',
    paddingY: '14px',
    borderRadius: '8px',
    borderWidth: '1px',
    cursor: 'pointer',
    transition: 'common',
    textAlign: 'left',
    width: 'full',
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
  });
  const creditClass = css({
    display: 'inline-flex',
    alignItems: 'center',
    gap: '3px',
    fontSize: '14px',
    fontWeight: 'medium',
    fontVariantNumeric: 'tabular-nums',
    color: 'text.default',
  });
  const bonusBadgeClass = css({
    borderRadius: 'full',
    paddingX: '6px',
    paddingY: '1px',
    fontSize: '10px',
    fontWeight: 'semibold',
    whiteSpace: 'nowrap',
    flexShrink: '0',
    color: 'text.bright',
    backgroundColor: 'accent.brand.default',
  });
  const summaryCardClass = css({
    borderRadius: '8px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    padding: '16px',
    fontSize: '13px',
    backgroundColor: 'surface.default',
  });
  const summaryRowStyle = flex.raw({ justify: 'space-between' });
  const summaryLabelClass = css({ color: 'text.subtle' });
  const stateBadgeClass = flex({
    justifyContent: 'center',
    alignItems: 'center',
    borderRadius: 'full',
    size: '32px',
    color: 'text.bright',
    backgroundColor: 'surface.dark',
  });
  const stateCreditStyle = css.raw({
    display: 'inline-flex',
    alignItems: 'center',
    gap: '3px',
    fontWeight: 'semibold',
    fontVariantNumeric: 'tabular-nums',
    color: 'text.brand',
  });
</script>

<Modal style={css.raw({ padding: '0', maxWidth: '640px' })} closable={!busy} bind:open>
  <div class={css({ paddingX: '32px', paddingTop: '32px' })}>
    <div class={flex({ alignItems: 'center', justify: 'space-between' })}>
      <h2 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'text.default' })}>크레딧 충전하기</h2>
      <div class={flex({ alignItems: 'center', gap: '6px', fontSize: '13px', color: 'text.subtle' })}>
        보유 크레딧
        <span class={css(stateCreditStyle, { color: balance < 0 ? 'text.danger' : 'text.brand' })}>
          <Icon icon={PrismCreditIcon} size={14} />{comma(balance)}
        </span>
      </div>
    </div>
  </div>

  <div class={flex({ gap: '28px', paddingTop: '20px', paddingX: '32px', paddingBottom: '32px' })}>
    <div class={flex({ flexDirection: 'column', gap: '8px', width: '256px', flexShrink: '0' })}>
      {#each PRISM_CREDIT_PACKS as grid (grid.pack)}
        {@const on = grid.pack === picked}
        <button
          class={css(packRowStyle, {
            borderColor: on ? 'accent.brand.default' : 'border.subtle',
            backgroundColor: on ? 'accent.brand.subtle' : 'surface.default',
            _hover: { borderColor: on ? 'accent.brand.default' : 'border.default' },
          })}
          aria-pressed={on}
          onclick={() => (picked = grid.pack)}
          type="button"
        >
          <span class={creditClass}>
            <Icon style={css.raw({ color: 'text.brand' })} icon={PrismCreditIcon} size={14} />{comma(grid.credits)}
          </span>
          {#if grid.bonus > 0}
            <span class={bonusBadgeClass}>보너스 +{comma(grid.bonus)}</span>
          {/if}
          <span class={css({ flexGrow: '1' })}></span>
          <span
            class={css({
              fontSize: '14px',
              fontWeight: 'semibold',
              color: 'text.default',
              fontVariantNumeric: 'tabular-nums',
              whiteSpace: 'nowrap',
            })}
          >
            {comma(grid.price)}원
          </span>
        </button>
      {/each}

      <div class={flex({ alignItems: 'center', justify: 'center', gap: '4px', marginTop: '4px', fontSize: '12px', color: 'text.faint' })}>
        충전 후 보유 크레딧
        <span
          class={css({
            display: 'inline-flex',
            alignItems: 'center',
            gap: '2px',
            fontWeight: 'medium',
            fontVariantNumeric: 'tabular-nums',
            color: balance + chosen.credits + chosen.bonus < 0 ? 'text.danger' : 'text.subtle',
          })}
        >
          <Icon icon={PrismCreditIcon} size={12} />{comma(balance + chosen.credits + chosen.bonus)}
        </span>
      </div>
    </div>

    <div class={flex({ flex: '1', flexDirection: 'column', gap: '16px' })}>
      <div class={summaryCardClass}>
        <div class={css(summaryRowStyle)}>
          <span class={summaryLabelClass}>충전 팩</span>
          <span class={flex({ alignItems: 'center', gap: '6px' })}>
            <span
              class={css({
                display: 'inline-flex',
                alignItems: 'center',
                gap: '3px',
                color: 'text.default',
                fontWeight: 'medium',
                fontVariantNumeric: 'tabular-nums',
              })}
            >
              <Icon style={css.raw({ color: 'text.brand' })} icon={PrismCreditIcon} size={14} />{comma(chosen.credits)}
            </span>
            {#if chosen.bonus > 0}
              <span class={bonusBadgeClass}>보너스 +{comma(chosen.bonus)}</span>
            {/if}
          </span>
        </div>

        <div class={css({ marginTop: '12px', paddingTop: '12px', borderTopWidth: '1px', borderColor: 'border.subtle' })}>
          <div class={flex({ justify: 'space-between', fontSize: '14px', fontWeight: 'semibold' })}>
            <span class={css({ color: 'text.default' })}>결제 금액</span>
            <span class={css({ color: 'text.default' })}>{comma(chosen.price)}원</span>
          </div>
        </div>
      </div>

      {#if billingKey}
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
          {#if billingKey.type === BillingKeyType.KAKAOPAY}
            <KakaoPayLogo class={css({ height: '14px' })} />
          {:else}
            <span class={css({ fontSize: '14px', color: 'text.default' })}>{billingKey.name}</span>
          {/if}
          <Button onclick={onEditBillingKey} size="sm" variant="secondary">변경</Button>
        </div>

        <PaymentAgreements error={agreementsError} method={billingKey.type} onchange={(accepted) => (agreementsAccepted = accepted)} />
      {:else}
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
          <span class={css({ fontSize: '14px', color: 'text.subtle' })}>등록된 결제 수단이 없어요</span>
          <Button onclick={onEditBillingKey} size="sm" variant="secondary">결제 수단 등록</Button>
        </div>
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
        <Button style={css.raw({ width: 'full' })} disabled={billingKey === null} loading={busy} onclick={submit} size="lg" type="button">
          {comma(chosen.price)}원 결제하기
        </Button>

        <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px', fontSize: '12px', color: 'text.faint' })}>
          <div class={flex({ alignItems: 'center', gap: '5px' })}>
            <Icon icon={LockIcon} size={12} />
            <span>결제 정보는 암호화되어 안전하게 전송돼요.</span>
          </div>
          <span>사용하지 않은 크레딧은 결제 7일 내 전액 환불받을 수 있어요.</span>
        </div>
      </div>
    </div>
  </div>
</Modal>

<Modal style={css.raw({ alignItems: 'center', padding: '32px', maxWidth: '400px' })} bind:open={resultOpen}>
  {#if resultKind === 'success'}
    <div class={stateBadgeClass}>
      <Icon icon={PrismCreditIcon} size={16} />
    </div>

    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', marginTop: '16px', textAlign: 'center' })}>
      <div class={css({ fontSize: '18px', fontWeight: 'bold' })}>크레딧을 충전했어요</div>
      <div class={css({ fontSize: '13px', color: 'text.muted', wordBreak: 'keep-all' })}>충전한 크레딧은 프리즘 기능에 쓸 수 있어요.</div>
    </div>

    <div
      class={flex({
        flexDirection: 'column',
        marginTop: '24px',
        borderWidth: '1px',
        borderRadius: '8px',
        padding: '16px',
        width: 'full',
        backgroundColor: 'surface.default',
      })}
    >
      <div class={flex({ justifyContent: 'center', alignItems: 'center' })}>
        <div class={css({ fontSize: '15px', fontWeight: 'bold', color: 'text.default' })}>프리즘 크레딧</div>
      </div>

      <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />

      <div class={flex({ flexDirection: 'column', gap: '8px', fontSize: '13px' })}>
        <div class={flex({ justify: 'space-between' })}>
          <span class={css({ color: 'text.subtle' })}>충전 크레딧</span>
          <span class={css(stateCreditStyle)}><Icon icon={PrismCreditIcon} size={14} />{comma(charged.credits)}</span>
        </div>
        {#if charged.bonus > 0}
          <div class={flex({ justify: 'space-between' })}>
            <span class={css({ color: 'text.subtle' })}>보너스</span>
            <span class={css(stateCreditStyle)}><Icon icon={PrismCreditIcon} size={14} />{comma(charged.bonus)}</span>
          </div>
        {/if}
      </div>

      <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />

      <div class={flex({ justify: 'space-between', fontSize: '13px' })}>
        <span class={css({ fontWeight: 'semibold', color: 'text.default' })}>보유 크레딧</span>
        <span class={css(stateCreditStyle, { color: charged.after < 0 ? 'text.danger' : 'text.brand' })}>
          <Icon icon={PrismCreditIcon} size={14} />{comma(charged.after)}
        </span>
      </div>
    </div>

    <Button style={css.raw({ marginTop: '24px', width: 'full' })} onclick={() => (resultOpen = false)} type="button">닫기</Button>
  {:else}
    <div class={stateBadgeClass}>
      <Icon icon={ClockIcon} size={16} />
    </div>

    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', marginTop: '16px', textAlign: 'center' })}>
      <div class={css({ fontSize: '18px', fontWeight: 'bold' })}>결제를 확인하고 있어요</div>
      <div class={css({ fontSize: '13px', color: 'text.muted', wordBreak: 'keep-all' })}>
        결제가 확인되면 크레딧이 자동으로 채워져요. 다시 결제하지 말고 잠시만 기다려 주세요.
      </div>
    </div>

    <Button style={css.raw({ marginTop: '24px', width: 'full' })} onclick={() => (resultOpen = false)} type="button" variant="secondary">
      닫기
    </Button>
  {/if}
</Modal>
