<script lang="ts">
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import TypeIcon from '~icons/lucide/book-open-text';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FlaskConicalIcon from '~icons/lucide/flask-conical';
  import HeadsetIcon from '~icons/lucide/headset';
  import ImagesIcon from '~icons/lucide/images';
  import LinkIcon from '~icons/lucide/link';
  import SearchIcon from '~icons/lucide/search';
  import SproutIcon from '~icons/lucide/sprout';
  import { fragment, graphql } from '$graphql';
  import { Button, HorizontalDivider, Icon } from '$lib/components';
  import { Dialog } from '$lib/notification';
  import { comma } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { flex, grid } from '$styled-system/patterns';
  import RedeemCreditCodeModal from './RedeemCreditCodeModal.svelte';
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
        credit
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

          nextInvoice {
            id
            amount
            billingAt
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
  let redeemCreditCodeOpen = $state(false);
</script>

<div class={flex({ direction: 'column', gap: '24px' })}>
  <p class={css({ fontSize: '20px', fontWeight: 'bold' })}>결제 설정</p>

  <div class={flex({ direction: 'column', gap: '8px' })}>
    <p class={css({ fontWeight: 'medium' })}>플랜 정보</p>

    <div class={grid({ columns: 2, gap: '12px' })}>
      <div
        class={flex({
          flexDirection: 'column',
          borderWidth: '1px',
          borderRadius: '8px',
          paddingX: '16px',
          paddingTop: '16px',
          paddingBottom: '32px',
          backgroundColor: 'white',
        })}
      >
        <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '8px' })}>
          <div class={css({ fontSize: '15px', fontWeight: 'bold', color: 'gray.950' })}>타이피 BASIC ACCESS</div>

          <div class={css({ color: 'brand.500' })}>
            <span class={css({ fontSize: '15px', fontWeight: 'semibold' })}>무료</span>
          </div>
        </div>

        <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />

        <ul class={flex({ flexDirection: 'column', gap: '8px', fontSize: '13px', fontWeight: 'medium', color: 'gray.700' })}>
          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={TypeIcon} size={14} />
            <span>총 16,000자까지 작성 가능</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={ImagesIcon} size={14} />
            <span>총 20MB까지 파일 업로드 가능</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={SearchIcon} size={14} />
            <span>일반 검색</span>
          </li>
        </ul>
      </div>

      <div
        class={flex({
          flexDirection: 'column',
          borderWidth: '1px',
          borderRadius: '8px',
          paddingX: '16px',
          paddingTop: '16px',
          paddingBottom: '32px',
          backgroundColor: 'white',
        })}
      >
        <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '8px' })}>
          <div class={css({ fontSize: '15px', fontWeight: 'bold', color: 'gray.950' })}>타이피 FULL ACCESS</div>

          <div class={css({ color: 'brand.500' })}>
            <span class={css({ fontSize: '15px', fontWeight: 'bold' })}>4,900</span>
            <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>원</span>
            <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>/ 월</span>
          </div>
        </div>

        <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />

        <ul class={flex({ flexDirection: 'column', gap: '8px', fontSize: '13px', fontWeight: 'medium', color: 'gray.700' })}>
          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={TypeIcon} size={14} />
            <span>무제한 글자 수</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={ImagesIcon} size={14} />
            <span>무제한 파일 업로드</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={SearchIcon} size={14} />
            <span>고급 검색</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={LinkIcon} size={14} />
            <span>커스텀 공유 주소</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={FlaskConicalIcon} size={14} />
            <span>베타 기능 우선 접근</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={HeadsetIcon} size={14} />
            <span>문제 발생시 우선 지원</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={SproutIcon} size={14} />
            <span>디스코드 커뮤니티 참여</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'gray.500' })} icon={EllipsisIcon} size={14} />
            <span>그리고 더 많은 혜택</span>
          </li>
        </ul>
      </div>
    </div>
  </div>

  {#if !$user.plan || !$user.paymentBillingKey}
    <div class={flex({ direction: 'column', gap: '8px' })}>
      <p class={css({ fontWeight: 'medium' })}>이용중인 플랜</p>

      <div
        class={flex({
          align: 'center',
          justify: 'space-between',
          borderRadius: '8px',
          paddingX: '16px',
          paddingY: '12px',
          fontSize: '15px',
          fontWeight: 'medium',
          color: 'gray.700',
          backgroundColor: 'gray.100',
        })}
      >
        <span class={css({ fontWeight: 'bold' })}>타이피 BASIC ACCESS</span>

        <Button onclick={() => (updatePaymentMethodOpen = true)}>업그레이드</Button>
      </div>
    </div>
  {:else}
    <div class={flex({ direction: 'column', gap: '8px' })}>
      <p class={css({ fontWeight: 'medium' })}>이용중인 플랜</p>

      <div class={css({ borderRadius: '8px', paddingX: '16px', paddingY: '12px', backgroundColor: 'gray.100' })}>
        <p class={css({ fontSize: '15px', fontWeight: 'medium', color: 'gray.700' })}>
          {$user.plan.plan.name} 플랜
        </p>

        <p class={css({ marginTop: '4px', fontSize: '14px', color: 'gray.600' })}>
          {dayjs($user.plan.createdAt).formatAsDate()} - {dayjs($user.plan.expiresAt).formatAsDate()}

          <span class={css({ fontSize: '12px', color: 'gray.400' })}>
            {#if $user.plan.nextInvoice}
              ({dayjs($user.plan.nextInvoice.billingAt).formatAsDate()}에 {comma($user.plan.nextInvoice.amount)}원 결제 예정)
            {:else}
              ({dayjs($user.plan.expiresAt).formatAsDate()} 해지 예정)
            {/if}
          </span>
        </p>
      </div>
    </div>
  {/if}

  <HorizontalDivider color="secondary" />

  <div>
    <div class={flex({ align: 'center', justify: 'space-between' })}>
      <div>
        <p class={css({ fontWeight: 'medium' })}>현재 크레딧</p>

        <p class={css({ marginTop: '4px', fontSize: '12px', color: 'gray.600' })}>플랜 결제 시 잔여 크레딧이 먼저 사용됩니다.</p>
      </div>

      <p class={css({ fontSize: '14px', color: 'gray.800' })}>{comma($user.credit)}원</p>
    </div>

    <Button
      style={css.raw({ marginTop: '8px', marginLeft: 'auto' })}
      onclick={() => (redeemCreditCodeOpen = true)}
      size="sm"
      variant="secondary"
    >
      할인 코드 등록
    </Button>
  </div>

  {#if $user.paymentBillingKey}
    <HorizontalDivider color="secondary" />

    <div>
      <p class={css({ fontWeight: 'medium' })}>결제 카드 정보</p>

      <div class={flex({ align: 'center', justify: 'space-between', fontSize: '15px', color: 'gray.700' })}>
        {$user.paymentBillingKey.name}

        <Button onclick={() => (updatePaymentMethodOpen = true)} size="sm" variant="secondary">결제 카드 변경</Button>
      </div>
    </div>
  {/if}

  {#if $user.plan?.state === 'ACTIVE'}
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
            mixpanel.track('cancel_plan');
          },
        });
      }}
      type="button"
    >
      해지하기
    </button>
  {/if}
</div>

<UpdatePaymentMethodModal {$user} bind:open={updatePaymentMethodOpen} />
<RedeemCreditCodeModal bind:open={redeemCreditCodeOpen} />
