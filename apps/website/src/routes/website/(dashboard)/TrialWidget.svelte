<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { PlanAvailability } from '@/enums';
  import HourglassIcon from '~icons/lucide/hourglass';
  import ZapIcon from '~icons/lucide/zap';
  import { pushState } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import PlanUpgradeModal from './PlanUpgradeModal.svelte';
  import type { DashboardLayout_TrialWidget_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_TrialWidget_user;
  };

  let { $user: _user }: Props = $props();

  let planUpgradeModalOpen = $state(false);

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_TrialWidget_user on User {
        id
        canStartTrial
        ...DashboardLayout_PlanUpgradeModal_user

        subscription {
          id
          expiresAt

          plan {
            id
            availability
          }
        }
      }
    `),
  );

  const isTrial = $derived($user.subscription?.plan.availability === PlanAvailability.TRIAL);
  const canStartTrial = $derived($user.canStartTrial);

  const trialDaysRemaining = $derived.by(() => {
    if (!isTrial || !$user.subscription?.expiresAt) {
      return 0;
    }
    return Math.max(0, dayjs($user.subscription.expiresAt).diff(dayjs(), 'day'));
  });
</script>

{#if canStartTrial && !$user.subscription}
  <div class={css({ borderTopWidth: '1px', borderColor: 'border.default', paddingX: '12px', paddingY: '8px' })}>
    <Button
      style={css.raw({ width: 'full' })}
      gradient
      onclick={() => {
        planUpgradeModalOpen = true;
        mixpanel.track('open_plan_upgrade_modal', { via: 'trial_widget' });
      }}
      size="sm"
    >
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <Icon icon={ZapIcon} size={14} />
        <span>2주 무료 체험하기</span>
      </div>
    </Button>
  </div>
{:else if isTrial}
  <button
    class={flex({
      alignItems: 'center',
      gap: '6px',
      borderTopWidth: '1px',
      borderColor: 'border.default',
      paddingX: '12px',
      paddingY: '10px',
      width: 'full',
      cursor: 'pointer',
      backgroundColor: 'transparent',
      _hover: { backgroundColor: 'surface.muted' },
      transitionProperty: '[background-color]',
      transitionDuration: '150ms',
      transitionTimingFunction: 'ease',
    })}
    onclick={() => {
      pushState('', { shallowRoute: '/preference/billing' });
      mixpanel.track('open_billing_from_trial_widget');
    }}
    type="button"
  >
    <Icon style={css.raw({ color: 'text.faint' })} icon={HourglassIcon} size={14} />
    <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>
      무료 체험이 {trialDaysRemaining}일 후 종료돼요
    </span>
  </button>
{/if}

<PlanUpgradeModal {$user} title="2주 무료 체험을 시작해보세요" bind:open={planUpgradeModalOpen}>
  결제 수단 등록 없이 타이피의 모든 기능을
  <br />
  무료로 이용할 수 있어요.
</PlanUpgradeModal>
