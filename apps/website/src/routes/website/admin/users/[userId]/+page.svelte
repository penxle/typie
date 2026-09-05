<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { TypieError } from '@typie/lib/errors';
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import ArrowLeftIcon from '~icons/lucide/arrow-left';
  import { AdminIcon, AdminModal } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';
  import { unwrapError } from '$lib/graphql/error';
  import { isIndefinitePeriod } from '$lib/subscription-logic';
  import { graphql } from '$mearie';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  let impersonateModalOpen = $state(false);

  const [adminImpersonate] = createMutation(
    graphql(`
      mutation AdminUserDetail_AdminImpersonate_Mutation($input: AdminImpersonateInput!) {
        adminImpersonate(input: $input)
      }
    `),
  );

  const handleImpersonate = async () => {
    await adminImpersonate({ input: { userId: query.data.adminUser.id } });
    location.assign('/initial');
  };

  const [adminGiveCredit] = createMutation(
    graphql(`
      mutation AdminUserDetail_AdminGiveCredit_Mutation($input: AdminGiveCreditInput!) {
        adminGiveCredit(input: $input)
      }
    `),
  );

  const [adminRefundPayment] = createMutation(
    graphql(`
      mutation AdminUserDetail_AdminRefundPayment_Mutation($input: AdminRefundPaymentInput!) {
        adminRefundPayment(input: $input)
      }
    `),
  );

  const [adminGrantPrismCredit] = createMutation(
    graphql(`
      mutation AdminUserDetail_AdminGrantPrismCredit_Mutation($input: AdminGrantPrismCreditInput!) {
        adminGrantPrismCredit(input: $input)
      }
    `),
  );

  const [adminAdjustPrismCredit] = createMutation(
    graphql(`
      mutation AdminUserDetail_AdminAdjustPrismCredit_Mutation($input: AdminAdjustPrismCreditInput!) {
        adminAdjustPrismCredit(input: $input)
      }
    `),
  );

  const handleGrantPrismCredit = async () => {
    const amount = Number.parseInt(prompt('Enter prism credits to grant (integer): ') || '');
    if (Number.isNaN(amount) || amount <= 0) return;
    const note = prompt('Enter note (required): ')?.trim();
    if (!note) return;
    try {
      await adminGrantPrismCredit({ input: { userId: query.data.adminUser.id, amount, note } });
      query.refetch();
    } catch (err) {
      const unwrapped = unwrapError(err);
      alert(unwrapped instanceof Error ? unwrapped.message : 'Grant failed');
    }
  };

  const handleAdjustPrismCredit = async () => {
    const paidDelta = Number.parseInt(prompt('Paid delta in credits (integer, may be negative): ') || '');
    if (Number.isNaN(paidDelta)) return;
    const freeDelta = Number.parseInt(prompt('Free delta in credits (integer, may be negative): ') || '');
    if (Number.isNaN(freeDelta)) return;
    const note = prompt('Enter note (required): ')?.trim();
    if (!note) return;
    try {
      await adminAdjustPrismCredit({ input: { userId: query.data.adminUser.id, paidDelta, freeDelta, note } });
      query.refetch();
    } catch (err) {
      const unwrapped = unwrapError(err);
      alert(unwrapped instanceof Error ? unwrapped.message : 'Adjust failed');
    }
  };

  let refundTarget = $state<{ kind: 'WITHDRAWAL' | 'REMAINDER'; purchaseId: string | null } | null>(null);

  const refundQuote = createQuery(
    graphql(`
      query AdminUserDetail_PrismCreditRefundQuote_Query($userId: String!, $kind: PrismCreditRefundKind!, $purchaseId: String) {
        adminPrismCreditRefundQuote(userId: $userId, kind: $kind, purchaseId: $purchaseId) {
          eligible
          reason
          amount
          paidCredits
          freeCredits
          shortfall
          cancels {
            purchaseId
            paymentKey
            amount
          }
        }
      }
    `),
    () => ({ userId: query.data.adminUser.id, kind: refundTarget?.kind ?? 'REMAINDER', purchaseId: refundTarget?.purchaseId ?? null }),
    () => ({ skip: refundTarget === null }),
  );

  const [adminRefundPrismCredit] = createMutation(
    graphql(`
      mutation AdminUserDetail_AdminRefundPrismCredit_Mutation($input: AdminRefundPrismCreditInput!) {
        adminRefundPrismCredit(input: $input) {
          id
          state
        }
      }
    `),
  );

  const [adminRetryPrismCreditRefund] = createMutation(
    graphql(`
      mutation AdminUserDetail_AdminRetryPrismCreditRefund_Mutation($input: AdminRetryPrismCreditRefundInput!) {
        adminRetryPrismCreditRefund(input: $input) {
          id
          state
        }
      }
    `),
  );

  let prismRefundOpen = $state(false);
  let prismRefundNote = $state('');
  let prismRefundMethod = $state<'PG_CANCEL' | 'MANUAL'>('PG_CANCEL');
  let prismRefundBusy = $state(false);

  const prismRefundQuote = $derived(refundQuote.data?.adminPrismCreditRefundQuote);
  const prismRefundManualOnly = $derived(!!prismRefundQuote?.eligible && prismRefundQuote.shortfall > 0);

  const openPrismRefund = (kind: 'WITHDRAWAL' | 'REMAINDER', purchaseId: string | null) => {
    prismRefundNote = '';
    prismRefundMethod = 'PG_CANCEL';
    refundTarget = { kind, purchaseId };
    prismRefundOpen = true;
  };

  $effect(() => {
    if (!prismRefundOpen) refundTarget = null;
  });

  $effect(() => {
    if (prismRefundManualOnly) prismRefundMethod = 'MANUAL';
  });

  const handlePrismRefund = async () => {
    const quote = refundQuote.data?.adminPrismCreditRefundQuote;
    if (prismRefundBusy || !refundTarget || !quote?.eligible || !prismRefundNote.trim()) return;
    prismRefundBusy = true;
    try {
      await adminRefundPrismCredit({
        input: {
          userId: query.data.adminUser.id,
          kind: refundTarget.kind,
          purchaseId: refundTarget.purchaseId,
          expectedAmount: quote.amount,
          method: prismRefundMethod,
          note: prismRefundNote.trim(),
        },
      });
      prismRefundOpen = false;
      query.refetch();
    } catch (err) {
      const unwrapped = unwrapError(err);
      alert(unwrapped instanceof Error ? unwrapped.message : 'Refund failed');
      if (unwrapped instanceof TypieError && unwrapped.code === 'refund_quote_changed') refundQuote.refetch();
      query.refetch();
    } finally {
      prismRefundBusy = false;
    }
  };

  const handleRetryPrismRefund = async (refundId: string, method: 'PG_CANCEL' | 'MANUAL') => {
    if (prismRefundBusy) return;
    prismRefundBusy = true;
    try {
      await adminRetryPrismCreditRefund({ input: { refundId, method } });
    } catch (err) {
      const unwrapped = unwrapError(err);
      alert(unwrapped instanceof Error ? unwrapped.message : 'Retry failed');
    } finally {
      prismRefundBusy = false;
    }
    query.refetch();
  };

  const prismRefundCancels = (data: unknown): { purchaseId: string; paymentKey: string; amount: number; status: string }[] =>
    (data as { cancels?: { purchaseId: string; paymentKey: string; amount: number; status: string }[] } | null)?.cancels ?? [];

  let refundModalOpen = $state(false);
  let selectedInvoice: (typeof query.data.adminUser.paymentInvoices)[number] | null = $state(null);
  let refundReason = $state('');

  const handleRefund = async () => {
    if (!selectedInvoice) return;
    try {
      await adminRefundPayment({ input: { invoiceId: selectedInvoice.id, reason: refundReason || undefined } });
      refundModalOpen = false;
      refundReason = '';
      selectedInvoice = null;
      query.refetch();
    } catch (err) {
      const unwrapped = unwrapError(err);
      alert(unwrapped instanceof Error ? unwrapped.message : 'Refund failed');
    }
  };

  const invoiceStateColor = (state: string) => {
    switch (state) {
      case 'PAID': {
        return 'success.default';
      }
      case 'CANCELED': {
        return 'text.hint';
      }
      case 'OVERDUE': {
        return 'danger.default';
      }
      case 'UPCOMING': {
        return 'warning.default';
      }
      default: {
        return 'text.hint';
      }
    }
  };
</script>

<div class={flex({ flexDirection: 'column', gap: '24px', color: 'text.default' })}>
  <div class={flex({ alignItems: 'center', gap: '12px' })}>
    <button
      class={css({
        borderWidth: '2px',
        borderColor: 'border.default',
        paddingX: '12px',
        paddingY: '6px',
        fontSize: '12px',
        color: 'text.default',
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        backgroundColor: 'transparent',
        _hover: {
          backgroundColor: 'surface.hover',
        },
      })}
      onclick={() => history.back()}
      type="button"
    >
      <AdminIcon icon={ArrowLeftIcon} size={16} />
      BACK TO LIST
    </button>
    <h2 class={css({ fontSize: '18px', color: 'text.default' })}>USER DETAILS</h2>
  </div>

  {#if query.data.adminUser}
    <div
      class={grid({
        gap: '24px',
        gridTemplateColumns: '2fr 1fr',
        alignItems: 'start',
      })}
    >
      <!-- 왼쪽 컬럼: 핵심 콘텐츠 -->
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <!-- PROFILE -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>PROFILE</h3>

          <div class={flex({ gap: '20px', marginBottom: '24px' })}>
            <div
              class={css({
                size: '80px',
                backgroundColor: 'accent.subtle',
                overflow: 'hidden',
                flexShrink: '0',
              })}
            >
              {#if query.data.adminUser.avatar?.url}
                <img alt={query.data.adminUser.name} src={query.data.adminUser.avatar.url} />
              {/if}
            </div>

            <div class={flex({ flexDirection: 'column', gap: '8px' })}>
              <h4 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'text.default' })}>
                {query.data.adminUser.name}
              </h4>
              <div class={css({ fontSize: '12px', color: 'text.muted' })}>
                {query.data.adminUser.email}
              </div>
            </div>
          </div>
        </div>

        <!-- ACTIVITY -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>ACTIVITY</h3>

          <div class={grid({ gridTemplateColumns: 'repeat(2, 1fr)', gap: '24px', marginBottom: '32px' })}>
            <div>
              <div class={css({ fontSize: '24px', color: 'text.default', marginBottom: '4px' })}>
                {query.data.adminUser.documentCount}
              </div>
              <div class={css({ fontSize: '11px', color: 'text.muted' })}>DOCUMENTS</div>
            </div>

            <div>
              <div class={css({ fontSize: '24px', color: 'text.default', marginBottom: '4px' })}>
                {comma(query.data.adminUser.usage.totalCharacterCount)}
              </div>
              <div class={css({ fontSize: '11px', color: 'text.muted' })}>CHARACTERS</div>
            </div>
          </div>
        </div>

        <!-- PAYMENT HISTORY -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>
            PAYMENT HISTORY ({query.data.adminUser.paymentInvoices.length})
          </h3>

          {#if query.data.adminUser.paymentInvoices.length > 0}
            <div class={flex({ flexDirection: 'column', gap: '12px' })}>
              {#each query.data.adminUser.paymentInvoices as invoice (invoice.id)}
                {@const successRecord = invoice.records.find((r: { outcome: string }) => r.outcome === 'SUCCESS')}
                <div
                  class={css({
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    padding: '16px',
                  })}
                >
                  <div class={flex({ alignItems: 'center', justifyContent: 'space-between', marginBottom: '8px' })}>
                    <div class={flex({ alignItems: 'center', gap: '8px' })}>
                      <span class={css({ fontSize: '12px', color: 'text.default' })}>
                        {dayjs(invoice.dueAt).format('YYYY-MM-DD')}
                      </span>
                      <span class={css({ fontSize: '12px', color: invoiceStateColor(invoice.state) })}>
                        [{invoice.state}]
                      </span>
                    </div>
                    <span class={css({ fontSize: '14px', fontWeight: 'bold', color: 'text.default' })}>
                      ₩{comma(invoice.amount)}
                    </span>
                  </div>

                  <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '4px' })}>
                    {invoice.subscription.plan.name}
                  </div>

                  {#if successRecord}
                    <div class={css({ fontSize: '11px', color: 'text.muted' })}>
                      BILLING: ₩{comma(successRecord.billingAmount)} / CREDIT: ₩{comma(successRecord.creditAmount)}
                    </div>
                  {/if}

                  {#if invoice.state === 'PAID' && successRecord}
                    <button
                      class={css({
                        marginTop: '8px',
                        borderWidth: '1px',
                        borderColor: 'danger.default',
                        paddingX: '10px',
                        paddingY: '4px',
                        fontSize: '11px',
                        color: 'danger.default',
                        backgroundColor: 'transparent',
                        cursor: 'pointer',
                        _hover: {
                          backgroundColor: 'danger.default',
                          color: 'text.on.danger',
                        },
                      })}
                      onclick={() => {
                        selectedInvoice = invoice;
                        refundReason = '';
                        refundModalOpen = true;
                      }}
                      type="button"
                    >
                      REFUND
                    </button>
                  {/if}
                </div>
              {/each}
            </div>
          {:else}
            <div class={css({ fontSize: '12px', color: 'text.hint' })}>NO PAYMENT HISTORY</div>
          {/if}
        </div>

        <!-- SITES -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>
            SITES ({query.data.adminUser.sites.length})
          </h3>

          {#if query.data.adminUser.sites.length > 0}
            <div class={flex({ flexDirection: 'column', gap: '12px' })}>
              {#each query.data.adminUser.sites as site (site.id)}
                <div
                  class={css({
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    padding: '16px',
                  })}
                >
                  <a
                    class={css({
                      fontSize: '14px',
                      fontWeight: 'bold',
                      color: 'text.default',
                      _hover: { textDecoration: 'underline' },
                      display: 'block',
                      marginBottom: '4px',
                    })}
                    href={site.url}
                    rel="noopener noreferrer"
                    target="_blank"
                  >
                    {site.name}
                  </a>
                  <div class={css({ fontSize: '12px', color: 'text.muted' })}>
                    {site.url}
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <div class={css({ fontSize: '12px', color: 'text.hint' })}>NO SITES OWNED</div>
          {/if}
        </div>
      </div>

      <!-- 오른쪽 컬럼: 메타데이터 -->
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <!-- METADATA -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>METADATA</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'text.muted' })}>USER ID</span>
              <span class={css({ fontSize: '12px', color: 'text.default' })}>
                {query.data.adminUser.id}
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'text.muted' })}>ROLE</span>
              <span class={css({ fontSize: '12px', color: query.data.adminUser.role === 'ADMIN' ? 'text.default' : 'text.hint' })}>
                [{query.data.adminUser.role}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'text.muted' })}>STATE</span>
              <span
                class={css({
                  fontSize: '12px',
                  color: query.data.adminUser.state === 'ACTIVE' ? 'success.default' : 'danger.default',
                })}
              >
                [{query.data.adminUser.state}]
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'text.muted' })}>JOINED</span>
              <span class={css({ fontSize: '12px', color: 'text.default' })}>
                {dayjs(query.data.adminUser.createdAt).formatAsDateTime()}
              </span>
            </div>
          </div>
        </div>

        <!-- AUTHENTICATION -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>AUTHENTICATION</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div>
              <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '8px' })}>LOGIN METHODS</div>
              {#if query.data.adminUser.singleSignOns.length > 0}
                <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                  {#each query.data.adminUser.singleSignOns as sso (sso.id)}
                    <div class={css({ fontSize: '12px', color: 'text.default' })}>
                      [{sso.provider}] {sso.email}
                    </div>
                  {/each}
                </div>
              {:else}
                <div class={css({ fontSize: '12px', color: 'text.default' })}>
                  [EMAIL] {query.data.adminUser.email}
                </div>
              {/if}
            </div>
          </div>
        </div>

        <!-- IDENTITY -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>IDENTITY</h3>

          {#if query.data.adminUser.personalIdentity}
            <div class={flex({ flexDirection: 'column', gap: '16px' })}>
              <div>
                <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '4px' })}>NAME</div>
                <div class={css({ fontSize: '14px', color: 'text.default', fontWeight: 'bold' })}>
                  {query.data.adminUser.personalIdentity.name}
                </div>
              </div>

              <div class={grid({ gridTemplateColumns: '1fr 1fr', gap: '16px' })}>
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '4px' })}>BIRTH DATE</div>
                  <div class={css({ fontSize: '12px', color: 'text.default' })}>
                    {dayjs(query.data.adminUser.personalIdentity.birthDate).format('YYYY-MM-DD')}
                  </div>
                </div>
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '4px' })}>GENDER</div>
                  <div class={css({ fontSize: '12px', color: 'text.default' })}>
                    [{query.data.adminUser.personalIdentity.gender}]
                  </div>
                </div>
              </div>

              {#if query.data.adminUser.personalIdentity.phoneNumber}
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '4px' })}>PHONE NUMBER</div>
                  <div class={css({ fontSize: '12px', color: 'text.default' })}>
                    {query.data.adminUser.personalIdentity.phoneNumber}
                  </div>
                </div>
              {/if}
            </div>
          {:else}
            <div class={css({ fontSize: '12px', color: 'text.hint', textAlign: 'center', paddingY: '24px' })}>NO IDENTITY VERIFICATION</div>
          {/if}
        </div>

        <!-- SUBSCRIPTION -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>SUBSCRIPTION</h3>

          {#if query.data.adminUser.subscription}
            <div class={flex({ flexDirection: 'column', gap: '16px' })}>
              <div>
                <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '4px' })}>PLAN</div>
                <div class={css({ fontSize: '14px', color: 'text.default', fontWeight: 'bold' })}>
                  {query.data.adminUser.subscription.plan.name}
                </div>
              </div>

              <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                <span class={css({ fontSize: '11px', color: 'text.muted' })}>STATUS</span>
                <span
                  class={css({
                    fontSize: '12px',
                    color:
                      query.data.adminUser.subscription.state === 'ACTIVE'
                        ? 'success.default'
                        : query.data.adminUser.subscription.state === 'WILL_EXPIRE'
                          ? 'warning.default'
                          : query.data.adminUser.subscription.state === 'IN_GRACE_PERIOD'
                            ? 'danger.default'
                            : 'text.hint',
                  })}
                >
                  [{query.data.adminUser.subscription.state}]
                </span>
              </div>

              <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                <span class={css({ fontSize: '11px', color: 'text.muted' })}>STARTED</span>
                <span class={css({ fontSize: '12px', color: 'text.default' })}>
                  {dayjs(query.data.adminUser.subscription.startsAt).formatAsDateTime()}
                </span>
              </div>

              <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                <span class={css({ fontSize: '11px', color: 'text.muted' })}>PERIOD ENDS</span>
                <span class={css({ fontSize: '12px', color: 'text.default' })}>
                  {isIndefinitePeriod(query.data.adminUser.subscription.currentPeriodEndsAt)
                    ? 'INDEFINITE'
                    : dayjs(query.data.adminUser.subscription.currentPeriodEndsAt).formatAsDateTime()}
                </span>
              </div>

              <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                <span class={css({ fontSize: '11px', color: 'text.muted' })}>PAYMENT METHOD</span>
                <span class={css({ fontSize: '12px', color: 'text.default' })}>
                  [{query.data.adminUser.subscription.plan.availability}]
                </span>
              </div>
            </div>
          {:else}
            <div class={css({ fontSize: '12px', color: 'text.hint', textAlign: 'center', paddingY: '24px' })}>NO ACTIVE SUBSCRIPTION</div>
          {/if}
        </div>

        <!-- PAYMENT -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>PAYMENT</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'text.muted' })}>BILLING KEY</span>
              {#if query.data.adminUser.billingKey}
                <span class={css({ fontSize: '12px', color: 'text.default' })}>
                  {query.data.adminUser.billingKey.name}
                </span>
              {:else}
                <span class={css({ fontSize: '12px', color: 'text.hint' })}>NONE</span>
              {/if}
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'text.muted' })}>CREDIT BALANCE</span>
              <span class={css({ fontSize: '12px', color: query.data.adminUser.credit === 0 ? 'text.hint' : 'text.default' })}>
                ₩{comma(query.data.adminUser.credit)}
              </span>
            </div>
          </div>
        </div>

        <!-- PRISM CREDIT -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>PRISM CREDIT</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'text.muted' })}>BALANCE</span>
              <span class={css({ fontSize: '12px', color: query.data.adminPrismCredit.display === 0 ? 'text.hint' : 'text.default' })}>
                {comma(query.data.adminPrismCredit.display)} ({query.data.adminPrismCredit.total}m)
              </span>
            </div>

            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'text.muted' })}>PAID / FREE</span>
              <span class={css({ fontSize: '12px', color: 'text.default' })}>
                {query.data.adminPrismCredit.paid}m / {query.data.adminPrismCredit.free}m
              </span>
            </div>

            <button
              class={css({
                borderWidth: '1px',
                borderColor: 'border.default',
                paddingX: '12px',
                paddingY: '8px',
                fontSize: '12px',
                color: 'text.default',
                width: 'full',
              })}
              onclick={() => openPrismRefund('REMAINDER', null)}
              type="button"
            >
              REFUND REMAINDER (90%)
            </button>

            {#if query.data.adminPrismCreditEntries.length > 0}
              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                {#each query.data.adminPrismCreditEntries as entry (entry.id)}
                  <div class={css({ borderWidth: '1px', borderColor: 'border.default', padding: '8px' })}>
                    <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                      <span class={css({ fontSize: '11px', color: 'text.muted' })}>
                        {dayjs(entry.createdAt).formatAsDateTime()} [{entry.kind}]
                      </span>
                      <span class={css({ fontSize: '12px', color: 'text.default' })}>
                        {entry.paidDelta}m / {entry.freeDelta}m
                      </span>
                    </div>
                    {#if entry.key || entry.note || entry.actor || entry.expiresAt}
                      <div class={css({ fontSize: '11px', color: 'text.muted', marginTop: '4px' })}>
                        {#if entry.key}KEY: {entry.key}{/if}
                        {#if entry.actor}
                          BY: {entry.actor.name}{/if}
                        {#if entry.note}
                          NOTE: {entry.note}{/if}
                        {#if entry.expiresAt}
                          EXPIRES: {dayjs(entry.expiresAt).formatAsDateTime()}{/if}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {:else}
              <div class={css({ fontSize: '12px', color: 'text.hint', textAlign: 'center', paddingY: '8px' })}>NO ENTRIES</div>
            {/if}

            <button
              class={css({
                borderWidth: '1px',
                borderColor: 'border.default',
                paddingX: '12px',
                paddingY: '8px',
                fontSize: '12px',
                color: 'text.default',
                backgroundColor: 'transparent',
                width: 'full',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '8px',
                _hover: {
                  backgroundColor: 'surface.hover',
                },
              })}
              onclick={handleGrantPrismCredit}
              type="button"
            >
              GRANT PRISM CREDIT
            </button>

            <button
              class={css({
                borderWidth: '1px',
                borderColor: 'border.default',
                paddingX: '12px',
                paddingY: '8px',
                fontSize: '12px',
                color: 'text.default',
                backgroundColor: 'transparent',
                width: 'full',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '8px',
                _hover: {
                  backgroundColor: 'surface.hover',
                },
              })}
              onclick={handleAdjustPrismCredit}
              type="button"
            >
              ADJUST PRISM CREDIT
            </button>

            <div class={css({ fontSize: '11px', color: 'text.muted', marginTop: '8px' })}>PURCHASES</div>
            {#if query.data.adminPrismCreditPurchases.length === 0}
              <div class={css({ fontSize: '12px', color: 'text.hint' })}>NONE</div>
            {:else}
              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                {#each query.data.adminPrismCreditPurchases as purchase (purchase.id)}
                  <div class={css({ borderWidth: '1px', borderColor: 'border.default', padding: '8px' })}>
                    <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                      <span class={css({ fontSize: '11px', color: 'text.muted' })}>
                        {dayjs(purchase.paidAt ?? purchase.createdAt).formatAsDateTime()} [{purchase.state}] {purchase.pack}
                      </span>
                      <span class={css({ fontSize: '12px', color: 'text.default' })}>
                        {comma(purchase.price)}원 · {purchase.credits}+{purchase.bonusCredits} · REFUNDED {comma(purchase.refundedAmount)}원
                      </span>
                    </div>
                    <div class={css({ fontSize: '11px', color: 'text.muted', marginTop: '4px' })}>KEY: {purchase.paymentKey}</div>
                    {#if purchase.state === 'FAILED' && purchase.data?.failure}
                      <div class={css({ fontSize: '11px', color: 'danger.default', marginTop: '4px' })}>
                        FAIL: {purchase.data.failure.code}{purchase.data.failure.message ? ` ${purchase.data.failure.message}` : ''}
                      </div>
                    {/if}
                    {#if purchase.state === 'PAID'}
                      <button
                        class={css({
                          marginTop: '6px',
                          borderWidth: '1px',
                          borderColor: 'border.default',
                          paddingX: '8px',
                          paddingY: '4px',
                          fontSize: '11px',
                          color: 'text.default',
                        })}
                        onclick={() => openPrismRefund('WITHDRAWAL', purchase.id)}
                        type="button"
                      >
                        WITHDRAW (7D)
                      </button>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}

            <div class={css({ fontSize: '11px', color: 'text.muted', marginTop: '8px' })}>REFUNDS</div>
            {#if query.data.adminPrismCreditRefunds.length === 0}
              <div class={css({ fontSize: '12px', color: 'text.hint' })}>NONE</div>
            {:else}
              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                {#each query.data.adminPrismCreditRefunds as refund (refund.id)}
                  <div
                    class={css({
                      borderWidth: '1px',
                      borderColor: refund.state === 'PENDING' ? 'danger.default' : 'border.default',
                      padding: '8px',
                    })}
                  >
                    <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
                      <span class={css({ fontSize: '11px', color: 'text.muted' })}>
                        {dayjs(refund.createdAt).formatAsDateTime()} [{refund.state}] {refund.kind} / {refund.method}
                      </span>
                      <span class={css({ fontSize: '12px', color: 'text.default' })}>{comma(refund.amount)}원</span>
                    </div>
                    <div class={css({ fontSize: '11px', color: 'text.muted', marginTop: '4px' })}>
                      BY: {refund.actor.name} · NOTE: {refund.note}
                      {#if refund.purchaseId}
                        · PURCHASE: {refund.purchaseId}{/if}
                    </div>
                    {#each prismRefundCancels(refund.data) as cancel (cancel.purchaseId)}
                      <div class={css({ fontSize: '11px', color: 'text.muted' })}>
                        CANCEL {cancel.paymentKey}: {comma(cancel.amount)}원 [{cancel.status}]
                      </div>
                    {/each}
                    {#if refund.state === 'PENDING'}
                      <div class={flex({ gap: '8px', marginTop: '6px' })}>
                        <button
                          class={css({
                            borderWidth: '1px',
                            borderColor: 'border.default',
                            paddingX: '8px',
                            paddingY: '4px',
                            fontSize: '11px',
                            color: 'text.default',
                          })}
                          onclick={() => handleRetryPrismRefund(refund.id, 'PG_CANCEL')}
                          type="button"
                        >
                          RETRY PG
                        </button>
                        <button
                          class={css({
                            borderWidth: '1px',
                            borderColor: 'border.default',
                            paddingX: '8px',
                            paddingY: '4px',
                            fontSize: '11px',
                            color: 'text.default',
                          })}
                          onclick={() => handleRetryPrismRefund(refund.id, 'MANUAL')}
                          type="button"
                        >
                          MARK MANUAL DONE
                        </button>
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <!-- PREFERENCES -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>PREFERENCES</h3>

          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <span class={css({ fontSize: '11px', color: 'text.muted' })}>MARKETING</span>
              <span
                class={css({
                  fontSize: '12px',
                  color: query.data.adminUser.marketingConsent ? 'success.default' : 'text.hint',
                })}
              >
                {query.data.adminUser.marketingConsent ? 'CONSENTED' : 'NOT CONSENTED'}
              </span>
            </div>
          </div>
        </div>

        <!-- ACTIONS -->
        <div
          class={css({
            borderWidth: '2px',
            borderColor: 'border.default',
            padding: '24px',
            backgroundColor: 'surface.default',
          })}
        >
          <h3 class={css({ fontSize: '16px', color: 'text.default', marginBottom: '20px' })}>ACTIONS</h3>
          <button
            class={css({
              borderWidth: '1px',
              borderColor: 'border.default',
              paddingX: '12px',
              paddingY: '8px',
              marginY: '8px',
              fontSize: '12px',
              color: 'text.default',
              backgroundColor: 'transparent',
              width: 'full',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '8px',
              _hover: {
                backgroundColor: 'surface.hover',
              },
            })}
            onclick={() => (impersonateModalOpen = true)}
            type="button"
          >
            IMPERSONATE USER
          </button>

          <button
            class={css({
              borderWidth: '1px',
              borderColor: 'border.default',
              paddingX: '12px',
              paddingY: '8px',
              marginY: '8px',
              fontSize: '12px',
              color: 'text.default',
              backgroundColor: 'transparent',
              width: 'full',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '8px',
              _hover: {
                backgroundColor: 'surface.hover',
              },
            })}
            onclick={async () => {
              const amount = Number.parseInt(prompt('Enter the amount of credit to give: ') || '');

              if (!Number.isNaN(amount)) {
                await adminGiveCredit({ input: { userId: query.data.adminUser.id, amount } });
                query.refetch();
                alert(`${amount} points given to user ${query.data.adminUser.name}`);
              }
            }}
            type="button"
          >
            GIVE CREDIT
          </button>
        </div>
      </div>
    </div>

    <AdminModal
      actions={{
        cancel: {},
        confirm: {
          label: 'CONFIRM IMPERSONATE',
          onclick: handleImpersonate,
          variant: 'primary',
        },
      }}
      title="CONFIRM IMPERSONATION"
      bind:open={impersonateModalOpen}
    >
      <div class={css({ marginBottom: '16px' })}>
        <p class={css({ marginBottom: '8px' })}>ARE YOU SURE YOU WANT TO IMPERSONATE THIS USER?</p>
        <p class={css({ color: 'text.muted' })}>
          USER: {query.data.adminUser.name.toUpperCase()} ({query.data.adminUser.email})
        </p>
      </div>
    </AdminModal>

    <AdminModal
      actions={{
        cancel: {},
        confirm: {
          label: 'CONFIRM REFUND',
          onclick: handleRefund,
          variant: 'danger',
        },
      }}
      title="CONFIRM REFUND"
      bind:open={refundModalOpen}
    >
      {#if selectedInvoice}
        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          <div>
            <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '4px' })}>INVOICE</div>
            <div class={css({ fontSize: '12px', color: 'text.default' })}>{selectedInvoice.id}</div>
          </div>

          <div>
            <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '4px' })}>PLAN</div>
            <div class={css({ fontSize: '12px', color: 'text.default' })}>{selectedInvoice.subscription.plan.name}</div>
          </div>

          <div>
            <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '4px' })}>AMOUNT</div>
            <div class={css({ fontSize: '14px', fontWeight: 'bold', color: 'text.default' })}>₩{comma(selectedInvoice.amount)}</div>
          </div>

          <div
            class={css({
              color: 'text.on.danger.subtle',
              fontSize: '11px',
              padding: '8px',
              borderWidth: '1px',
              borderColor: 'danger.default',
              backgroundColor: 'danger.subtle',
            })}
          >
            THIS WILL CANCEL THE PAYMENT AND EXPIRE THE SUBSCRIPTION IMMEDIATELY.
          </div>

          <div>
            <div class={css({ fontSize: '11px', color: 'text.muted', marginBottom: '4px' })}>REASON</div>
            <input
              class={css({
                width: 'full',
                padding: '8px',
                fontSize: '12px',
                color: 'text.default',
                backgroundColor: 'surface.inset',
                borderWidth: '1px',
                borderColor: 'border.default',
                outline: 'none',
                _focus: { borderColor: 'accent.default' },
              })}
              placeholder="Enter refund reason..."
              bind:value={refundReason}
            />
          </div>
        </div>
      {/if}
    </AdminModal>

    <AdminModal
      actions={{
        cancel: {},
        confirm: {
          label: 'CONFIRM PRISM REFUND',
          onclick: handlePrismRefund,
          variant: 'danger',
          disabled:
            prismRefundBusy ||
            !refundQuote.data?.adminPrismCreditRefundQuote.eligible ||
            prismRefundNote.trim().length === 0 ||
            (refundQuote.data.adminPrismCreditRefundQuote.shortfall > 0 && prismRefundMethod === 'PG_CANCEL'),
        },
      }}
      title="PRISM CREDIT REFUND"
      bind:open={prismRefundOpen}
    >
      {#if refundTarget}
        {@const quote = prismRefundQuote}
        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          <div class={css({ fontSize: '12px', color: 'text.default' })}>
            {refundTarget.kind}{refundTarget.purchaseId ? ` · ${refundTarget.purchaseId}` : ''}
          </div>
          {#if !quote}
            <div class={css({ fontSize: '12px', color: 'text.muted' })}>LOADING…</div>
          {:else if !quote.eligible}
            <div class={css({ fontSize: '12px', color: 'danger.default' })}>NOT ELIGIBLE: {quote.reason}</div>
          {:else}
            <div class={css({ fontSize: '12px', color: 'text.default' })}>AMOUNT: {comma(quote.amount)}원</div>
            <div class={css({ fontSize: '12px', color: 'text.default' })}>RECLAIM: paid {quote.paidCredits} / free {quote.freeCredits}</div>
            {#if quote.shortfall > 0}
              <div class={css({ fontSize: '12px', color: 'danger.default' })}>
                PG SHORTFALL: {comma(quote.shortfall)}원 — PG_CANCEL UNAVAILABLE, METHOD FORCED TO MANUAL
              </div>
            {/if}
            <div class={css({ fontSize: '11px', color: 'text.muted' })}>
              {#each quote.cancels as cancel (cancel.purchaseId)}
                <div>CANCEL {cancel.paymentKey}: {comma(cancel.amount)}원</div>
              {/each}
            </div>
            <label class={css({ fontSize: '11px', color: 'text.muted' })}>
              METHOD
              <select
                class={css({
                  marginLeft: '8px',
                  backgroundColor: 'surface.inset',
                  color: 'text.default',
                  borderWidth: '1px',
                  borderColor: 'border.default',
                })}
                bind:value={prismRefundMethod}
              >
                <option disabled={prismRefundManualOnly} value="PG_CANCEL">PG_CANCEL</option>
                <option value="MANUAL">MANUAL</option>
              </select>
            </label>
            <label class={css({ fontSize: '11px', color: 'text.muted' })}>
              NOTE (required)
              <input
                class={css({
                  marginLeft: '8px',
                  width: 'full',
                  backgroundColor: 'surface.inset',
                  color: 'text.default',
                  borderWidth: '1px',
                  borderColor: 'border.default',
                  padding: '4px',
                })}
                bind:value={prismRefundNote}
              />
            </label>
          {/if}
        </div>
      {/if}
    </AdminModal>
  {/if}
</div>
