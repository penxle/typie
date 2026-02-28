<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { PLAN_FEATURES } from '@typie/ui/constants';
  import mixpanel from 'mixpanel-browser';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CrownIcon from '~icons/lucide/crown';
  import GiftIcon from '~icons/lucide/gift';
  import KeyIcon from '~icons/lucide/key';
  import StarIcon from '~icons/lucide/star';
  import TagIcon from '~icons/lucide/tag';
  import { pushState } from '$app/navigation';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import type { DashboardLayout_TrialExpiredModal_user$key } from '$mearie';

  type Props = {
    open: boolean;
    user$key: DashboardLayout_TrialExpiredModal_user$key;
  };

  let { open = $bindable(false), user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_TrialExpiredModal_user on User {
        id
      }
    `),
    () => user$key,
  );

  const [recordSurvey] = createMutation(
    graphql(`
      mutation DashboardLayout_TrialExpiredModal_RecordSurvey_Mutation($input: RecordSurveyInput!) {
        recordSurvey(input: $input) {
          id
        }
      }
    `),
  );

  async function markAsShown() {
    await recordSurvey({
      input: {
        name: 'trial_expired_modal_shown',
        value: {},
      },
    });
    cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'surveys' });
  }

  async function handleClose() {
    await markAsShown();
    mixpanel.track('dismiss_trial_expired_modal');
    open = false;
  }

  async function handleUpgrade() {
    await markAsShown();
    mixpanel.track('click_upgrade_from_trial_expired');
    open = false;
    pushState('', { shallowRoute: '/preference/billing' });
  }

  $effect(() => {
    if (open) {
      mixpanel.track('view_trial_expired_modal');
    }
  });
</script>

<Modal
  style={css.raw({
    alignItems: 'center',
    padding: '32px',
    maxWidth: '400px',
  })}
  closable={false}
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
    <div class={css({ fontSize: '18px', fontWeight: 'bold' })}>무료 체험이 종료됐어요</div>

    <div class={css({ fontSize: '13px', color: 'text.faint', wordBreak: 'keep-all' })}>무료 체험은 어떠셨나요?</div>

    <div class={css({ fontSize: '13px', color: 'text.faint', wordBreak: 'keep-all' })}>
      타이피의 모든 기능을 계속 이용하시려면
      <br />
      플랜을 업그레이드해 주세요.
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

  <Button style={css.raw({ marginTop: '32px', width: 'full', height: '40px' })} onclick={handleClose} variant="secondary">
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <span>좀 더 둘러볼게요</span>
    </div>
  </Button>

  <Button style={css.raw({ marginTop: '8px', width: 'full', height: '40px' })} gradient onclick={handleUpgrade}>
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <span>지금 업그레이드</span>

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
</Modal>
