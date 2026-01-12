<script lang="ts">
  import { cache } from '@typie/sark/internal';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { PLAN_FEATURES } from '@typie/ui/constants';
  import { Dialog } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CrownIcon from '~icons/lucide/crown';
  import GiftIcon from '~icons/lucide/gift';
  import KeyIcon from '~icons/lucide/key';
  import StarIcon from '~icons/lucide/star';
  import TagIcon from '~icons/lucide/tag';
  import { pushState } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import SubscriptionCelebrationModal from './SubscriptionCelebrationModal.svelte';
  import type { Snippet } from 'svelte';
  import type { DashboardLayout_PlanUpgradeModal_user } from '$graphql';

  type Props = {
    open: boolean;
    $user: DashboardLayout_PlanUpgradeModal_user;
    title?: string;
    children?: Snippet;
  };

  let { open = $bindable(false), $user: _user, title = '플랜 업그레이드가 필요해요', children }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PlanUpgradeModal_user on User {
        id

        canStartTrial

        subscription {
          id
        }
      }
    `),
  );

  const subscribePlanWithTrial = graphql(`
    mutation DashboardLayout_PlanUpgradeModal_SubscribePlanWithTrial_Mutation {
      subscribePlanWithTrial {
        id
        state
        expiresAt

        plan {
          id
          name
          availability
        }
      }
    }
  `);

  const canStartTrial = $derived($user.canStartTrial);

  let trialStartedModalOpen = $state(false);
</script>

<Modal
  style={css.raw({
    alignItems: 'center',
    padding: '32px',
    maxWidth: '400px',
  })}
  bind:open
>
  <div
    class={flex({
      alignItems: 'center',
      '& > div': {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        borderWidth: '2px',
        borderColor: 'surface.default',
        borderRadius: 'full',
        marginRight: '-8px',
        size: '32px',
        color: 'text.bright',
        backgroundColor: 'surface.dark',
      },
    })}
  >
    <div>
      <Icon icon={CrownIcon} size={16} />
    </div>

    <div>
      <Icon icon={TagIcon} size={16} />
    </div>

    <div>
      <Icon icon={StarIcon} size={16} />
    </div>

    <div>
      <Icon icon={KeyIcon} size={16} />
    </div>

    <div>
      <Icon icon={GiftIcon} size={16} />
    </div>
  </div>

  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px', marginTop: '16px', textAlign: 'center' })}>
    <div class={css({ fontSize: '18px', fontWeight: 'bold' })}>{title}</div>

    <div class={css({ fontSize: '13px', color: 'text.faint' })}>
      {@render children?.()}
    </div>
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      marginTop: '24px',
      borderWidth: '1px',
      borderRadius: '8px',
      paddingX: '16px',
      paddingTop: '16px',
      paddingBottom: '32px',
      width: 'full',
      backgroundColor: 'surface.default',
    })}
  >
    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', color: 'text.default' })}>타이피 FULL ACCESS</div>

      <div class={css({ color: 'text.brand' })}>
        <span class={css({ fontSize: '15px', fontWeight: 'bold' })}>4,900</span>
        <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>원</span>
        <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>/ 월</span>
      </div>
    </div>

    <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />

    <ul class={flex({ flexDirection: 'column', gap: '8px', fontSize: '13px', fontWeight: 'medium', color: 'text.subtle' })}>
      {#each PLAN_FEATURES.full as feature, index (index)}
        <li class={flex({ alignItems: 'center', gap: '6px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={feature.icon} size={14} />
          <span>{feature.label}</span>
        </li>
      {/each}
    </ul>
  </div>

  <div class={flex({ flexDirection: 'column', gap: '8px', marginTop: '32px', width: 'full' })}>
    {#if canStartTrial}
      <Button
        style={css.raw({ width: 'full', height: '40px' })}
        gradient
        onclick={() => {
          Dialog.confirm({
            title: '무료 체험을 시작하시겠어요?',
            message: '결제 수단 등록 없이 2주간 타이피의 모든 기능을 무료로 이용할 수 있어요. 체험 종료 후 자동 결제되지 않아요.',
            actionLabel: '시작하기',
            actionHandler: async () => {
              await subscribePlanWithTrial();
              cache.invalidate({ __typename: 'User', id: $user.id, field: 'subscription' });
              cache.invalidate({ __typename: 'User', id: $user.id, field: 'canStartTrial' });
              mixpanel.track('start_trial');
              open = false;
              trialStartedModalOpen = true;
            },
          });
        }}
      >
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <span>2주 무료 체험하기</span>

          <Icon
            style={css.raw({
              transition: 'transform',
              _groupHover: { transform: 'translateX(2px)' },
            })}
            icon={ArrowRightIcon}
            size={16}
          />
        </div>
      </Button>
    {/if}

    <Button
      style={css.raw({ width: 'full', height: '40px' })}
      gradient={!canStartTrial}
      onclick={() => {
        open = false;
        pushState('', { shallowRoute: '/preference/billing' });
      }}
      variant={canStartTrial ? 'secondary' : undefined}
    >
      업그레이드
    </Button>
  </div>
</Modal>

<SubscriptionCelebrationModal
  message="2주간 타이피의 모든 기능을 자유롭게 이용해보세요."
  title="무료 체험이 시작됐어요!"
  bind:open={trialStartedModalOpen}
/>
