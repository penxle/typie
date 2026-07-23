<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { PlanAvailability } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import { isLegacyTrial, trialDaysLeft, trialStatusLabel } from '$lib/subscription-logic';
  import { graphql } from '$mearie';
  import PlanChangeNoticeModal from './PlanChangeNoticeModal.svelte';
  import { SubscribeModal } from './subscribe-modal.svelte';
  import type { HomePane_TrialBanner_user$key } from '$mearie';

  type Props = {
    user$key: HomePane_TrialBanner_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment HomePane_TrialBanner_user on User {
        id
        hadSubscription

        billingKey {
          id
        }

        subscription {
          id
          startsAt
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

  const subscription = $derived(user.data.subscription);
  const isTrial = $derived(subscription?.plan.availability === PlanAvailability.TRIAL);
  const expired = $derived(!subscription);
  const hasScheduled = $derived(Boolean(user.data.nextSubscription));

  const legacy = $derived(
    subscription ? isLegacyTrial({ availability: subscription.plan.availability, startsAt: subscription.startsAt }) : false,
  );
  const daysLeft = $derived(subscription ? trialDaysLeft(subscription.expiresAt, dayjs()) : 0);

  const visible = $derived((isTrial || expired) && !hasScheduled);

  let planChangeNoticeOpen = $state(false);

  const handleClick = () => {
    if (legacy) {
      planChangeNoticeOpen = true;
      return;
    }

    SubscribeModal.show(isTrial ? 'home_banner' : 'home_banner_expired');
  };
</script>

{#if visible}
  <div
    class={flex({
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: '16px',
      width: '800px',
      maxWidth: 'full',
      paddingX: '20px',
      paddingY: '16px',
      borderRadius: '12px',
      borderWidth: '1px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
    })}
  >
    <div class={flex({ flexDirection: 'column', gap: '2px' })}>
      {#if expired}
        <p class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default' })}>
          {user.data.hadSubscription ? '이용 기간 만료' : '무료 체험 종료'}
        </p>
        <p class={css({ fontSize: '12px', color: 'text.muted' })}>쓰던 글을 이어가려면 구독을 시작해 주세요.</p>
      {:else}
        <p class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default' })}>{trialStatusLabel(daysLeft, legacy)}</p>
        <p class={css({ fontSize: '12px', color: 'text.muted' })}>
          기간이 끝나도 이어 쓸 수 있도록, {user.data.billingKey ? '구독을 미리 예약해 보세요.' : '결제 수단을 미리 등록해 보세요.'}
        </p>
      {/if}
    </div>

    <Button style={css.raw({ flexShrink: '0' })} onclick={handleClick} size="sm">
      {expired ? '타이피 계속 쓰기' : user.data.billingKey ? '구독 예약하기' : '결제 수단 등록하기'}
    </Button>
  </div>
{/if}

<PlanChangeNoticeModal
  onsubscribe={() => SubscribeModal.show('plan_change_notice')}
  showSubscribe={true}
  bind:open={planChangeNoticeOpen}
/>
