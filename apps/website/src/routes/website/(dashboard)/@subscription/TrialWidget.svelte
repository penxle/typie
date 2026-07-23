<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { PlanAvailability } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import dayjs from 'dayjs';
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { isLegacyTrial, shouldShowTrialReminder, trialDaysLeft, trialReminderLabel, trialStatusLabel } from '$lib/subscription-logic';
  import { graphql } from '$mearie';
  import PlanChangeNoticeModal from './PlanChangeNoticeModal.svelte';
  import { SubscribeModal } from './subscribe-modal.svelte';
  import type { DashboardLayout_TrialWidget_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_TrialWidget_user$key;
  };

  let { user$key }: Props = $props();

  const app = getAppContext();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_TrialWidget_user on User {
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

  let reminderOpen = $state(false);
  let planChangeNoticeOpen = $state(false);

  onMount(() => {
    if (!isTrial || hasScheduled) {
      return;
    }

    const today = dayjs().format('YYYY-MM-DD');
    if (!shouldShowTrialReminder({ daysLeft, today, lastShownDate: app.preference.current.trialReminderLastShownDate })) {
      return;
    }

    app.preference.current.trialReminderLastShownDate = today;

    const showTimer = setTimeout(() => (reminderOpen = true), 500);
    const hideTimer = setTimeout(() => (reminderOpen = false), 5500);

    return () => {
      clearTimeout(showTimer);
      clearTimeout(hideTimer);
    };
  });

  const handleClick = () => {
    if (legacy) {
      planChangeNoticeOpen = true;
      return;
    }

    SubscribeModal.show(isTrial ? 'trial_widget' : 'expired_widget');
  };
</script>

{#if visible}
  <div
    class={css({
      position: 'sticky',
      bottom: '0',
      paddingX: '12px',
      paddingTop: '12px',
      paddingBottom: '2px',
      backgroundColor: 'surface.subtle',
    })}
  >
    <div class={css({ position: 'relative' })}>
      {#if reminderOpen}
        <button
          class={css({
            position: 'absolute',
            bottom: '[calc(100% + 8px)]',
            left: '0',
            right: '0',
            borderRadius: '8px',
            paddingX: '12px',
            paddingY: '10px',
            fontSize: '12px',
            fontWeight: 'semibold',
            textAlign: 'center',
            color: 'text.bright',
            backgroundColor: 'surface.dark',
            boxShadow: 'medium',
            cursor: 'pointer',
            _after: {
              content: '""',
              position: 'absolute',
              bottom: '-4px',
              left: '[50%]',
              size: '10px',
              backgroundColor: 'surface.dark',
              transform: 'translateX(-50%) rotate(45deg)',
            },
          })}
          onclick={() => {
            reminderOpen = false;
            handleClick();
          }}
          type="button"
          transition:fade={{ duration: 150 }}
        >
          {trialReminderLabel(daysLeft, legacy)}
        </button>
      {/if}

      {#if expired}
        <div
          class={flex({
            flexDirection: 'column',
            alignItems: 'center',
            gap: '8px',
            width: 'full',
            padding: '14px',
            borderRadius: '12px',
            borderWidth: '1px',
            borderColor: 'border.default',
            backgroundColor: 'surface.default',
            textAlign: 'center',
          })}
        >
          <p class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.default' })}>
            {user.data.hadSubscription ? '이용 기간 만료' : '무료 체험 종료'}
          </p>

          <p class={css({ fontSize: '12px', color: 'text.muted', lineHeight: '[1.6]' })}>
            쓰던 글을 이어가려면
            <br />
            구독을 시작해 주세요.
          </p>

          <Button style={css.raw({ width: 'full' })} onclick={handleClick} size="sm">타이피 계속 쓰기</Button>
        </div>
      {:else}
        <div
          class={flex({
            flexDirection: 'column',
            alignItems: 'center',
            gap: '8px',
            width: 'full',
            padding: '14px',
            borderRadius: '12px',
            borderWidth: '1px',
            borderColor: 'border.default',
            backgroundColor: 'surface.default',
            textAlign: 'center',
          })}
        >
          <p class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.default' })}>{trialStatusLabel(daysLeft, legacy)}</p>

          <p class={css({ fontSize: '12px', color: 'text.muted', lineHeight: '[1.6]' })}>
            기간이 끝나도 이어 쓸 수 있도록,
            <br />
            {user.data.billingKey ? '구독을 미리 예약해 보세요.' : '결제 수단을 미리 등록해 보세요.'}
          </p>

          <Button style={css.raw({ width: 'full' })} onclick={handleClick} size="sm">
            {user.data.billingKey ? '구독 예약하기' : '결제 수단 등록하기'}
          </Button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<PlanChangeNoticeModal
  onsubscribe={() => SubscribeModal.show('plan_change_notice')}
  showSubscribe={true}
  bind:open={planChangeNoticeOpen}
/>
