<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { defaultPlanRules } from '@/const';
  import { PlanAvailability } from '@/enums';
  import { pushState } from '$app/navigation';
  import { graphql } from '$mearie';
  import PlanUpgradeModal from './PlanUpgradeModal.svelte';
  import type { DashboardLayout_PlanUsageWidget_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PlanUsageWidget_user$key;
  };

  let { user$key }: Props = $props();

  let planUpgradeModalOpen = $state(false);

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PlanUsageWidget_user on User {
        id
        canStartTrial
        ...DashboardLayout_PlanUpgradeModal_user

        usage {
          totalCharacterCount
          totalBlobSize
        }

        subscription {
          id
          startsAt
          expiresAt

          plan {
            id
            availability

            rule {
              maxTotalCharacterCount
              maxTotalBlobSize
            }
          }
        }
      }
    `),
    () => user$key,
  );

  const planRule = $derived(user.data.subscription?.plan?.rule ?? defaultPlanRules);

  const progress = $derived.by(() => {
    const charProgress =
      planRule.maxTotalCharacterCount === -1 ? -1 : Math.min(1, user.data.usage.totalCharacterCount / planRule.maxTotalCharacterCount);

    const blobProgress =
      planRule.maxTotalBlobSize === -1 ? -1 : Math.min(1, Number(user.data.usage.totalBlobSize) / planRule.maxTotalBlobSize);

    return Math.max(charProgress, blobProgress);
  });

  const canStartTrial = $derived(user.data.canStartTrial && !user.data.subscription);
  const isTrial = $derived(user.data.subscription?.plan.availability === PlanAvailability.TRIAL);

  const trialDaysRemaining = $derived.by(() => {
    if (!isTrial || !user.data.subscription?.expiresAt) {
      return 0;
    }
    return Math.max(0, dayjs(user.data.subscription.expiresAt).diff(dayjs(), 'day'));
  });

  const trialProgress = $derived.by(() => {
    if (!isTrial || !user.data.subscription) return 0;
    const totalDays = dayjs(user.data.subscription.expiresAt).diff(dayjs(user.data.subscription.startsAt), 'day');
    if (totalDays <= 0) return 1;
    return Math.min(1, (totalDays - trialDaysRemaining) / totalDays);
  });

  const visible = $derived(progress !== -1 || isTrial);
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
    <div
      class={flex({
        flexDirection: 'column',
        gap: '6px',
        width: 'full',
        paddingX: '8px',
        paddingY: '6px',
        borderRadius: '6px',
        borderWidth: '1px',
        borderColor: 'border.default',
        backgroundColor: 'surface.default',
      })}
    >
      <div class={flex({ alignItems: 'center', justifyContent: 'center', gap: '8px', width: 'full' })}>
        <div
          style:--progress={`${(isTrial ? trialProgress : progress) * 360}deg`}
          class={css({
            flexShrink: '0',
            width: '16px',
            height: '16px',
            borderRadius: 'full',
            background:
              '[conic-gradient(token(colors.accent.brand.default) var(--progress), token(colors.interactive.hover) var(--progress))]',
            mask: '[radial-gradient(circle, transparent 5px, black 5.5px)]',
          })}
        ></div>
        <span class={css({ fontSize: '12px', color: 'text.faint' })}>
          {#if isTrial}
            {#if trialDaysRemaining === 0}
              무료 체험이 오늘 종료돼요
            {:else}
              무료 체험 중 · {trialDaysRemaining}일 남음
            {/if}
          {:else}
            무료 플랜 · 현재 {Math.round(progress * 100)}% 사용
          {/if}
        </span>
      </div>

      <button
        class={css({
          width: 'full',
          paddingY: '5px',
          borderRadius: '4px',
          fontSize: '12px',
          fontWeight: 'semibold',
          color: 'white',
          backgroundColor: 'accent.brand.default',
          cursor: 'pointer',
          transition: 'common',
          _hover: { backgroundColor: 'accent.brand.hover' },
        })}
        onclick={() => {
          if (isTrial) {
            pushState('', { shallowRoute: '/preference/billing' });
            mixpanel.track('open_billing_from_trial_widget');
          } else {
            planUpgradeModalOpen = true;
            mixpanel.track('open_plan_upgrade_modal', { via: 'usage_widget' });
          }
        }}
        type="button"
      >
        {canStartTrial ? '2주 무료 체험하기' : '지금 업그레이드하기'}
      </button>
    </div>
  </div>

  {#if canStartTrial}
    <PlanUpgradeModal title="2주 무료 체험을 시작해보세요" user$key={user.data} bind:open={planUpgradeModalOpen}>
      결제 수단 등록 없이 타이피의 모든 기능을
      <br />
      무료로 이용할 수 있어요.
    </PlanUpgradeModal>
  {:else if !isTrial}
    <PlanUpgradeModal user$key={user.data} bind:open={planUpgradeModalOpen}>
      FULL ACCESS로 업그레이드하면
      <br />
      무제한으로 글을 작성하고 파일을 업로드할 수 있어요.
    </PlanUpgradeModal>
  {/if}
{/if}
