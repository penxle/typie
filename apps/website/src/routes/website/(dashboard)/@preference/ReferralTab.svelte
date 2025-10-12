<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { comma } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import CopyIcon from '~icons/lucide/copy';
  import GiftIcon from '~icons/lucide/gift';
  import UsersIcon from '~icons/lucide/users';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import type { DashboardLayout_PreferenceModal_ReferralTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_ReferralTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_ReferralTab_user on User {
        id

        referrals {
          id
          compensated
        }
      }
    `),
  );

  const issueReferralUrl = graphql(`
    mutation DashboardLayout_PreferenceModal_ReferralTab_IssueReferralUrl_Mutation {
      issueReferralUrl
    }
  `);

  const copyReferralUrl = async () => {
    const referralUrl = await issueReferralUrl();
    await navigator.clipboard.writeText(`📝 타이피 가입하고 한달 무료 혜택 받아가세요! ${referralUrl}`);

    Toast.success('초대 링크가 클립보드에 복사되었어요. 친구들에게 공유해보세요!');

    mixpanel.track('copy_referral_url');
  };
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '12px' })}>초대</h1>

    <p class={css({ fontSize: '14px', color: 'text.muted' })}>
      친구를 초대하면 친구는 바로 1달 무료, 친구가 첫 결제를 하면 나도 1달 무료 혜택을 받아요.
    </p>
  </div>

  <!-- Friend Invitation Section -->
  <div>
    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          초대 링크
        {/snippet}
        {#snippet description()}
          링크를 복사해 친구들에게 공유해 보세요.
        {/snippet}
        {#snippet value()}
          <Button onclick={copyReferralUrl} size="sm" variant="secondary">
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon icon={CopyIcon} size={16} />
              링크 복사
            </div>
          </Button>
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>

  <!-- Referral Status Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>초대 현황</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          <div class={flex({ align: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={UsersIcon} size={16} />
            <span>초대한 친구</span>
          </div>
        {/snippet}
        {#snippet value()}
          <span>{comma($user.referrals.length)}명</span>
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          <div class={flex({ align: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.disabled' })} icon={GiftIcon} size={16} />
            <span>지금까지 내가 받은 혜택</span>
          </div>
        {/snippet}
        {#snippet value()}
          <span>
            {#if $user.referrals.some((referral) => referral.compensated)}
              {comma($user.referrals.filter((referral) => referral.compensated).length * 4900)}원
            {:else}
              없음
            {/if}
          </span>
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>

  <!-- Referral Benefits Guide Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>초대 혜택 안내</h2>

    <div
      class={css({
        borderRadius: '8px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
        padding: '20px',
        backgroundColor: 'surface.default',
      })}
    >
      <ul class={flex({ direction: 'column', gap: '12px' })}>
        <li class={flex({ gap: '8px', alignItems: 'flex-start' })}>
          <span class={css({ color: 'text.disabled', fontSize: '13px', flexShrink: 0, marginTop: '2px' })}>•</span>
          <span class={css({ fontSize: '13px', color: 'text.faint', lineHeight: '[1.6]' })}>
            초대 링크를 통해 웹에서 가입하고, 웹에서 플랜을 가입해야 초대 혜택을 받을 수 있어요. 앱에서 가입하면 혜택을 받을 수 없어요.
          </span>
        </li>
        <li class={flex({ gap: '8px', alignItems: 'flex-start' })}>
          <span class={css({ color: 'text.disabled', fontSize: '13px', flexShrink: 0, marginTop: '2px' })}>•</span>
          <span class={css({ fontSize: '13px', color: 'text.faint', lineHeight: '[1.6]' })}>
            친구가 초대 링크로 가입하면 친구는 즉시 FULL ACCESS 플랜 1개월에 해당하는 크레딧을 지급받아요. 지급받은 크레딧으로 바로 FULL
            ACCESS 플랜을 체험해볼 수 있어요.
          </span>
        </li>
        <li class={flex({ gap: '8px', alignItems: 'flex-start' })}>
          <span class={css({ color: 'text.disabled', fontSize: '13px', flexShrink: 0, marginTop: '2px' })}>•</span>
          <span class={css({ fontSize: '13px', color: 'text.faint', lineHeight: '[1.6]' })}>
            친구가 크레딧을 통한 체험을 끝내고 첫 결제를 완료하면 나도 FULL ACCESS 플랜 1개월에 상응하는 크레딧을 지급받아요. 이 크레딧은
            다음 FULL ACCESS 플랜 갱신시 자동으로 이용돼요.
          </span>
        </li>
        <li class={flex({ gap: '8px', alignItems: 'flex-start' })}>
          <span class={css({ color: 'text.disabled', fontSize: '13px', flexShrink: 0, marginTop: '2px' })}>•</span>
          <span class={css({ fontSize: '13px', color: 'text.faint', lineHeight: '[1.6]' })}>초대 횟수에는 제한이 없어요.</span>
        </li>
      </ul>
    </div>
  </div>
</div>
