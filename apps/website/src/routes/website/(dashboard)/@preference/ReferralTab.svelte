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

<div class={flex({ direction: 'column', gap: '32px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>초대</h1>

  <div class={flex({ direction: 'column', gap: '16px' })}>
    <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>친구 초대</h3>

    <p class={css({ fontSize: '13px', color: 'text.muted' })}>
      친구를 초대하면 친구는 바로 1달 무료, 친구가 첫 결제를 하면 나도 1달 무료 혜택을 받아요.
    </p>

    <Button style={css.raw({ height: '36px', alignSelf: 'flex-start' })} onclick={copyReferralUrl} size="sm" variant="secondary">
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon icon={CopyIcon} size={16} />
        초대 링크 복사
      </div>
    </Button>
  </div>

  <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>

  <div class={flex({ direction: 'column', gap: '20px' })}>
    <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>초대 현황</h3>

    <div class={flex({ gap: '16px' })}>
      <div
        class={flex({
          flex: '1',
          direction: 'column',
          gap: '8px',
          borderRadius: '8px',
          padding: '16px',
          borderWidth: '1px',
          borderColor: 'border.default',
          backgroundColor: 'surface.subtle',
        })}
      >
        <div class={flex({ align: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.disabled' })} icon={UsersIcon} size={16} />
          <p class={css({ fontSize: '13px', color: 'text.muted' })}>초대한 친구</p>
        </div>
        <p class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>{comma($user.referrals.length)}명</p>
      </div>

      <div
        class={flex({
          flex: '1',
          direction: 'column',
          gap: '8px',
          borderRadius: '8px',
          padding: '16px',
          borderWidth: '1px',
          borderColor: 'border.default',
          backgroundColor: 'surface.subtle',
        })}
      >
        <div class={flex({ align: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.disabled' })} icon={GiftIcon} size={16} />
          <p class={css({ fontSize: '13px', color: 'text.muted' })}>받은 혜택</p>
        </div>
        <p class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>
          {#if $user.referrals.some((referral) => referral.compensated)}
            {comma($user.referrals.filter((referral) => referral.compensated).length)}개월 무료 이용
          {:else}
            없음
          {/if}
        </p>
      </div>
    </div>
  </div>

  <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>

  <div
    class={css({
      borderRadius: '8px',
      padding: '16px',
      backgroundColor: 'surface.subtle',
    })}
  >
    <h4 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default', marginBottom: '12px' })}>초대 혜택 안내</h4>

    <ul class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]', listStyleType: 'disc', paddingLeft: '20px' })}>
      <li>
        친구가 초대 링크로 가입하면 친구는 즉시 FULL ACCESS 플랜 1개월에 해당하는 크레딧을 지급받아요. 지급받은 크레딧으로 바로 FULL ACCESS
        플랜을 체험해볼 수 있어요.
      </li>
      <li>
        친구가 크레딧을 통한 체험을 끝내고 첫 결제를 완료하면 나도 FULL ACCESS 플랜 1개월에 상응하는 크레딧을 지급받아요. 이 크레딧은 다음
        FULL ACCESS 플랜 갱신시 자동으로 이용돼요.
      </li>
      <li>초대 횟수에는 제한이 없어요.</li>
    </ul>
  </div>
</div>
