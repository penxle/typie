<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal, TextInput } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { inAppPurchaseStoreLabels, paymentInvoiceStateLabels, paymentInvoiceStateTones, paymentOutcomeLabels } from '$lib/admin-labels';
  import { AdminBadge, AdminDataTable, AdminJson } from '$lib/components/admin';
  import { cache, unwrapError } from '$lib/graphql';
  import { graphql } from '$mearie';
  import type { AdminUserBillingTab_user$key } from '$mearie';

  type Props = {
    user$key: AdminUserBillingTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment AdminUserBillingTab_user on User {
        id
        name
        credit
        uuid

        billingKey {
          id
          name
        }

        inAppPurchase {
          id
          store
          identifier
          createdAt
        }

        paymentInvoices {
          id
          state
          amount
          dueAt

          subscription {
            id

            plan {
              id
              name
            }
          }

          records {
            id
            outcome
            billingAmount
            creditAmount
            data
            createdAt
          }
        }
      }
    `),
    () => user$key,
  );

  const [adminGiveCredit] = createMutation(
    graphql(`
      mutation AdminUserBillingTab_AdminGiveCredit_Mutation($input: AdminGiveCreditInput!) {
        adminGiveCredit(input: $input)
      }
    `),
  );

  const [adminRefundPayment] = createMutation(
    graphql(`
      mutation AdminUserBillingTab_AdminRefundPayment_Mutation($input: AdminRefundPaymentInput!) {
        adminRefundPayment(input: $input)
      }
    `),
  );

  let creditModalOpen = $state(false);
  let creditAmount = $state('');
  const creditAmountValue = $derived(Number(creditAmount));
  const creditAmountValid = $derived(Number.isSafeInteger(creditAmountValue) && creditAmountValue > 0);

  let refundModalOpen = $state(false);
  let refundInvoice = $state<(typeof user.data.paymentInvoices)[number] | null>(null);
  let refundReason = $state('');

  const refundInvoiceCreditAmount = $derived(
    refundInvoice?.records.filter((record) => record.outcome === 'SUCCESS').reduce((sum, record) => sum + record.creditAmount, 0) ?? 0,
  );

  const handleGiveCredit = async () => {
    if (!creditAmountValid) {
      return;
    }

    try {
      await adminGiveCredit({ input: { userId: user.data.id, amount: creditAmountValue } });
      cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'credit' });

      creditModalOpen = false;
      creditAmount = '';
      Toast.success('크레딧을 지급했어요');
    } catch (err) {
      const unwrapped = unwrapError(err);
      Toast.error(unwrapped instanceof Error ? unwrapped.message : '크레딧 지급에 실패했어요. 잠시 후 다시 시도해주세요.');
    }
  };

  const handleRefund = async () => {
    if (!refundInvoice) {
      return;
    }

    try {
      await adminRefundPayment({ input: { invoiceId: refundInvoice.id, reason: refundReason || undefined } });
      cache.invalidate(
        { __typename: 'User', id: user.data.id, $field: 'paymentInvoices' },
        { __typename: 'User', id: user.data.id, $field: 'subscriptions' },
        { __typename: 'User', id: user.data.id, $field: 'hasActiveSubscription' },
      );

      refundModalOpen = false;
      refundInvoice = null;
      refundReason = '';
      Toast.success('환불했어요');
    } catch (err) {
      const unwrapped = unwrapError(err);
      Toast.error(unwrapped instanceof Error ? unwrapped.message : '환불에 실패했어요. 잠시 후 다시 시도해주세요.');
    }
  };
</script>

<div class={flex({ alignItems: 'center', justifyContent: 'space-between', marginBottom: '16px' })}>
  <div class={css({ fontSize: '13px', color: 'text.muted' })}>
    크레딧 {comma(user.data.credit)}원 · 빌링키 {user.data.billingKey?.name ?? '없음'}
  </div>
  <Button onclick={() => (creditModalOpen = true)} size="sm" variant="secondary">크레딧 지급</Button>
</div>

{#if user.data.inAppPurchase}
  <div
    class={css({
      marginBottom: '16px',
      padding: '12px',
      borderWidth: '1px',
      borderColor: 'border.subtle',
      borderRadius: '12px',
      backgroundColor: 'admin.card.default',
      boxShadow: 'adminCard',
      fontSize: '13px',
      color: 'text.muted',
    })}
  >
    <div class={css({ marginBottom: '4px', fontWeight: 'medium', color: 'text.default' })}>인앱결제</div>
    <div>
      스토어 {inAppPurchaseStoreLabels[user.data.inAppPurchase.store]} · 스토어 측 식별자 {user.data.inAppPurchase.identifier} · 연동일 {dayjs(
        user.data.inAppPurchase.createdAt,
      ).formatAsDateTime()}
    </div>
    <div>스토어 계정 토큰(앱스토어/플레이 콘솔 조회용) {user.data.uuid}</div>
  </div>
{/if}

<AdminDataTable
  columns={[
    { key: '$plan', label: '플랜', width: '16%' },
    { key: '$amount', label: '금액', width: '12%' },
    { key: '$state', label: '상태', width: '10%' },
    { key: '$dueAt', label: '청구일', width: '16%' },
    { key: '$records', label: '결제 기록', width: '30%' },
    { key: '$actions', label: '', width: '16%' },
  ]}
  data={[...user.data.paymentInvoices]}
  dataKey="id"
  emptyText={user.data.inAppPurchase ? '인앱결제 구독은 인보이스가 생성되지 않아요' : '인보이스가 없습니다'}
>
  {#snippet $plan(invoice)}
    {invoice.subscription?.plan.name ?? '—'}
  {/snippet}

  {#snippet $amount(invoice)}
    {comma(invoice.amount)}원
  {/snippet}

  {#snippet $state(invoice)}
    <AdminBadge label={paymentInvoiceStateLabels[invoice.state]} tone={paymentInvoiceStateTones[invoice.state]} />
  {/snippet}

  {#snippet $dueAt(invoice)}
    {dayjs(invoice.dueAt).formatAsDateTime()}
  {/snippet}

  {#snippet $records(invoice)}
    {#each invoice.records as record (record.id)}
      <details>
        <summary class={css({ fontSize: '12px', cursor: 'pointer' })}>
          {dayjs(record.createdAt).formatAsDateTime()} · {paymentOutcomeLabels[record.outcome]} · 청구 {comma(record.billingAmount)}원 ·
          크레딧 {comma(record.creditAmount)}원
        </summary>
        <div class={css({ marginTop: '6px' })}>
          <AdminJson value={record.data} />
        </div>
      </details>
    {:else}
      <span class={css({ color: 'text.disabled' })}>—</span>
    {/each}
  {/snippet}

  {#snippet $actions(invoice)}
    {#if invoice.state === 'PAID'}
      <Button
        onclick={() => {
          refundInvoice = invoice;
          refundReason = '';
          refundModalOpen = true;
        }}
        size="sm"
        variant="danger"
      >
        환불
      </Button>
    {/if}
  {/snippet}
</AdminDataTable>

<Modal style={css.raw({ padding: '24px', maxWidth: '400px' })} bind:open={creditModalOpen}>
  <div class={flex({ flexDirection: 'column', gap: '24px' })}>
    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', color: 'text.default' })}>크레딧을 지급할까요?</div>
      <div class={css({ fontSize: '13px', color: 'text.faint', wordBreak: 'keep-all' })}>
        {user.data.name}님에게 입력한 금액만큼 크레딧을 지급해요.
      </div>
    </div>

    <TextInput placeholder="금액(원)" size="sm" type="number" bind:value={creditAmount} />

    <div class={flex({ justifyContent: 'flex-end', gap: '10px' })}>
      <Button onclick={() => (creditModalOpen = false)} size="sm" type="button" variant="secondary">취소</Button>
      <Button disabled={!creditAmountValid} onclick={handleGiveCredit} size="sm" type="button">지급</Button>
    </div>
  </div>
</Modal>

<Modal style={css.raw({ padding: '24px', maxWidth: '420px' })} bind:open={refundModalOpen}>
  <div class={flex({ flexDirection: 'column', gap: '24px' })}>
    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', color: 'text.default' })}>이 결제를 환불할까요?</div>

      {#if refundInvoice}
        <div class={css({ fontSize: '13px', color: 'text.faint', wordBreak: 'keep-all' })}>
          {refundInvoice.subscription?.plan.name ?? '—'} · {comma(refundInvoice.amount)}원 ·
          {dayjs(refundInvoice.dueAt).formatAsDateTime()}
        </div>
      {/if}

      <div class={css({ fontSize: '13px', color: 'text.danger', wordBreak: 'keep-all' })}>
        되돌릴 수 없는 작업이에요. 구독이 즉시 만료되고, 예약된 구독도 함께 취소돼요.
      </div>

      <div class={css({ fontSize: '13px', color: 'text.danger', wordBreak: 'keep-all' })}>
        {#if refundInvoiceCreditAmount > 0}
          이 결제에서 선차감된 크레딧 {comma(refundInvoiceCreditAmount)}원은 환불로 복원되지 않아요. 필요하면 별도로 크레딧을 지급하세요.
        {:else}
          결제에 사용된 크레딧은 환불로 복원되지 않아요.
        {/if}
      </div>
    </div>

    <TextInput placeholder="사유(선택)" size="sm" bind:value={refundReason} />

    <div class={flex({ justifyContent: 'flex-end', gap: '10px' })}>
      <Button onclick={() => (refundModalOpen = false)} size="sm" type="button" variant="secondary">취소</Button>
      <Button onclick={handleRefund} size="sm" type="button" variant="danger">환불</Button>
    </div>
  </div>
</Modal>
