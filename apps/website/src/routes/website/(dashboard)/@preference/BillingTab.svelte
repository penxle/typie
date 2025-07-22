<script lang="ts">
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { SubscriptionState } from '@/enums';
  import TypeIcon from '~icons/lucide/book-open-text';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FlaskConicalIcon from '~icons/lucide/flask-conical';
  import HeadsetIcon from '~icons/lucide/headset';
  import ImagesIcon from '~icons/lucide/images';
  import LinkIcon from '~icons/lucide/link';
  import SearchIcon from '~icons/lucide/search';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import SproutIcon from '~icons/lucide/sprout';
  import TypeOutlineIcon from '~icons/lucide/type-outline';
  import { fragment, graphql } from '$graphql';
  import { Button, Icon } from '$lib/components';
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

        billingKey {
          id
          name
        }

        subscription {
          id
          state
          startsAt
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

  const scheduleSubscriptionCancellation = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_ScheduleSubscriptionCancellation_Mutation {
      scheduleSubscriptionCancellation {
        id
      }
    }
  `);

  let updatePaymentMethodOpen = $state(false);
  let redeemCreditCodeOpen = $state(false);
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>결제</h1>

  <div class={flex({ direction: 'column', gap: '16px' })}>
    <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>플랜 정보</h3>

    <div class={grid({ columns: 2, gap: '12px' })}>
      <div
        class={flex({
          flexDirection: 'column',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '12px',
          padding: '20px',
          backgroundColor: 'surface.default',
        })}
      >
        <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '8px', marginBottom: '16px' })}>
          <div class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default' })}>타이피 BASIC ACCESS</div>

          <div class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.muted' })}>무료</div>
        </div>

        <ul class={flex({ flexDirection: 'column', gap: '10px', fontSize: '13px', color: 'text.muted' })}>
          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={TypeIcon} size={14} />
            <span>총 16,000자까지 작성 가능</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={ImagesIcon} size={14} />
            <span>총 20MB까지 파일 업로드 가능</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={SearchIcon} size={14} />
            <span>일반 검색</span>
          </li>
        </ul>
      </div>

      <div
        class={flex({
          flexDirection: 'column',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '12px',
          padding: '20px',
          backgroundColor: 'surface.default',
        })}
      >
        <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '8px', marginBottom: '16px' })}>
          <div class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default' })}>타이피 FULL ACCESS</div>

          <div>
            <span class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>4,900</span>
            <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted' })}>원 / 월</span>
          </div>
        </div>

        <ul class={flex({ flexDirection: 'column', gap: '10px', fontSize: '13px', color: 'text.muted' })}>
          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={TypeIcon} size={14} />
            <span>무제한 글자 수</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={ImagesIcon} size={14} />
            <span>무제한 파일 업로드</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={SearchIcon} size={14} />
            <span>고급 검색</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={SpellCheckIcon} size={14} />
            <span>맞춤법 검사</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={LinkIcon} size={14} />
            <span>커스텀 게시 주소</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={TypeOutlineIcon} size={14} />
            <span>커스텀 폰트 업로드</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={FlaskConicalIcon} size={14} />
            <span>베타 기능 우선 접근</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={HeadsetIcon} size={14} />
            <span>문제 발생시 우선 지원</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={SproutIcon} size={14} />
            <span>디스코드 커뮤니티 참여</span>
          </li>

          <li class={flex({ alignItems: 'center', gap: '6px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={EllipsisIcon} size={14} />
            <span>그리고 더 많은 혜택</span>
          </li>
        </ul>
      </div>
    </div>
  </div>

  {#if !$user.subscription}
    <div class={flex({ direction: 'column', gap: '12px' })}>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>이용중인 플랜</h3>

      <div
        class={flex({
          align: 'center',
          justify: 'space-between',
          borderRadius: '8px',
          padding: '16px',
          borderWidth: '1px',
          borderColor: 'border.default',
          backgroundColor: 'surface.subtle',
        })}
      >
        <div>
          <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>타이피 BASIC ACCESS</p>
          <p class={css({ marginTop: '2px', fontSize: '13px', color: 'text.faint' })}>무료 플랜을 사용 중입니다</p>
        </div>

        <Button onclick={() => (updatePaymentMethodOpen = true)} size="sm" variant="secondary">업그레이드</Button>
      </div>
    </div>
  {:else}
    <div class={flex({ direction: 'column', gap: '12px' })}>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>이용중인 플랜</h3>

      <div
        class={css({
          borderRadius: '8px',
          padding: '16px',
          borderWidth: '1px',
          borderColor: 'border.default',
          backgroundColor: 'surface.subtle',
        })}
      >
        <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>
          {$user.subscription.plan.name} 플랜
        </p>

        <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.muted' })}>
          {dayjs($user.subscription.startsAt).formatAsDate()} - {dayjs($user.subscription.expiresAt).formatAsDate()}
        </p>

        {#if $user.subscription.state === SubscriptionState.ACTIVE}
          <p class={css({ marginTop: '8px', fontSize: '12px', color: 'text.faint' })}>
            {dayjs($user.subscription.expiresAt).formatAsDate()}에 {comma($user.subscription.plan.fee)}원 결제 예정
          </p>
        {:else if $user.subscription.state === SubscriptionState.WILL_EXPIRE}
          <p class={css({ marginTop: '8px', fontSize: '12px', color: 'text.danger' })}>
            {dayjs($user.subscription.expiresAt).formatAsDate()} 해지 예정
          </p>
        {/if}
      </div>
    </div>
  {/if}

  <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>

  <div>
    <div class={flex({ direction: 'column', gap: '12px' })}>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>크레딧</h3>

      <div class={flex({ align: 'center', justify: 'space-between' })}>
        <div>
          <p class={css({ fontSize: '14px', color: 'text.muted' })}>현재 크레딧</p>
          <p class={css({ marginTop: '2px', fontSize: '12px', color: 'text.faint' })}>플랜 결제 시 잔여 크레딧이 먼저 사용됩니다</p>
        </div>

        <p class={css({ fontSize: '16px', fontWeight: 'medium', color: 'text.default' })}>{comma($user.credit)}원</p>
      </div>

      <Button style={css.raw({ alignSelf: 'flex-start' })} onclick={() => (redeemCreditCodeOpen = true)} size="sm" variant="secondary">
        할인 코드 등록
      </Button>
    </div>
  </div>

  {#if $user.billingKey}
    <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>

    <div class={flex({ direction: 'column', gap: '12px' })}>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>결제 카드 정보</h3>

      <div class={flex({ align: 'center', justify: 'space-between' })}>
        <p class={css({ fontSize: '14px', color: 'text.subtle' })}>{$user.billingKey.name}</p>

        <Button onclick={() => (updatePaymentMethodOpen = true)} size="sm" variant="secondary">결제 카드 변경</Button>
      </div>
    </div>
  {/if}

  {#if $user.subscription?.state === SubscriptionState.ACTIVE}
    <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>
    <button
      class={css({
        alignSelf: 'flex-start',
        paddingX: '8px',
        paddingY: '4px',
        fontSize: '13px',
        color: 'text.faint',
        width: 'fit',
        borderRadius: '4px',
        transition: 'common',
        _hover: { color: 'text.danger', backgroundColor: 'accent.danger.subtle' },
      })}
      onclick={() => {
        Dialog.confirm({
          title: '정말로 해지하시겠습니까?',
          message: '해지 후에도 남은 기간 동안 서비스를 이용하실 수 있습니다.',
          action: 'danger',
          actionLabel: '해지',
          actionHandler: async () => {
            await scheduleSubscriptionCancellation();
            mixpanel.track('cancel_plan');
          },
        });
      }}
      type="button"
    >
      구독 해지
    </button>
  {/if}
</div>

<UpdatePaymentMethodModal {$user} bind:open={updatePaymentMethodOpen} />
<RedeemCreditCodeModal bind:open={redeemCreditCodeOpen} />
