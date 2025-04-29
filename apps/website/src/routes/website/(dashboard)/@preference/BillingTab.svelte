<script lang="ts">
  import dayjs from 'dayjs';
  import { fragment, graphql } from '$graphql';
  import { Button, HorizontalDivider } from '$lib/components';
  import { Dialog } from '$lib/notification';
  import { comma } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import UpdatePaymentMethodModal from './UpdatePaymentMethodModal.svelte';
  import type { DashboardLayout_PreferenceModal_BillingTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_BillingTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_BillingTab_user on User {
        id
        ...DashboardLayout_PreferenceModal_BillingTab_UpdatePaymentMethodModal_user

        paymentBillingKey {
          id
          name
        }

        plan {
          id
          fee
          billingCycle
          state
          createdAt
          expiresAt

          plan {
            id
            name
            fee
          }
        }
      }
    `),
  );

  const cancelPlan = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_CancelPlan_Mutation {
      cancelPlan {
        id
      }
    }
  `);

  let updatePaymentMethodOpen = $state(false);
</script>

<div class={flex({ direction: 'column', gap: '24px' })}>
  <p class={css({ fontSize: '20px', fontWeight: 'bold' })}>결제 설정</p>

  {#if !$user.plan || !$user.paymentBillingKey}
    <div class={flex({ direction: 'column', gap: '8px' })}>
      <p class={css({ fontWeight: 'medium' })}>현재 플랜</p>

      <div
        class={flex({
          align: 'center',
          justify: 'space-between',
          borderRadius: '4px',
          padding: '12px',
          fontSize: '15px',
          fontWeight: 'medium',
          color: 'gray.700',
          backgroundColor: 'gray.100',
        })}
      >
        무료 플랜

        <Button onclick={() => (updatePaymentMethodOpen = true)}>카드 등록 및 결제</Button>
      </div>
    </div>
  {:else}
    <div class={flex({ direction: 'column', gap: '8px' })}>
      <p class={css({ fontWeight: 'medium' })}>현재 플랜</p>

      <div class={css({ borderRadius: '4px', padding: '12px', backgroundColor: 'gray.100' })}>
        <p class={css({ fontSize: '15px', fontWeight: 'medium', color: 'gray.700' })}>
          {$user.plan.plan.name} 플랜
        </p>

        <p class={css({ marginTop: '4px', fontSize: '14px', color: 'gray.600' })}>
          {dayjs($user.plan.createdAt).formatAsDate()} - {dayjs($user.plan.expiresAt).formatAsDate()}

          <span class={css({ fontSize: '12px', color: 'gray.400' })}>
            {#if $user.plan.state === 'ACTIVE'}
              ({dayjs($user.plan.expiresAt).formatAsDate()}에 {comma($user.plan.fee)}원 결제 예정)
            {:else}
              ({dayjs($user.plan.expiresAt).formatAsDate()}에 해지 예정)
            {/if}
          </span>
        </p>
      </div>
    </div>

    <div>
      <p class={css({ fontWeight: 'medium' })}>결제 카드 정보</p>

      <div class={flex({ align: 'center', justify: 'space-between', fontSize: '15px', color: 'gray.700' })}>
        {$user.paymentBillingKey.name}

        <Button onclick={() => (updatePaymentMethodOpen = true)} size="sm" variant="secondary">결제 카드 변경</Button>
      </div>
    </div>

    {#if $user.plan.state === 'ACTIVE'}
      <HorizontalDivider color="secondary" />
      <button
        class={css({ padding: '4px', fontSize: '13px', color: 'gray.400', width: 'fit' })}
        onclick={() => {
          Dialog.confirm({
            title: '정말로 해지하시겠습니까?',
            message: '해지 후에도 남은 기간 동안 서비스를 이용하실 수 있습니다.',
            action: 'danger',
            actionLabel: '해지',
            actionHandler: async () => {
              await cancelPlan();
            },
          });
        }}
        type="button"
      >
        해지하기
      </button>
    {/if}
  {/if}
</div>

<UpdatePaymentMethodModal {$user} bind:open={updatePaymentMethodOpen} />
