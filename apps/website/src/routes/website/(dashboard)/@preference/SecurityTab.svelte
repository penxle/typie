<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import NaverIcon from '~icons/simple-icons/naver';
  import GoogleIcon from '~icons/typie/google';
  import KakaoIcon from '~icons/typie/kakao';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import UpdatePasswordModal from './UpdatePasswordModal.svelte';
  import type { DashboardLayout_PreferenceModal_SecurityTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_SecurityTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_SecurityTab_user on User {
        id
        hasPassword

        singleSignOns {
          id
          email
          provider
        }
      }
    `),
  );

  const deleteUser = graphql(`
    mutation DashboardLayout_PreferenceModal_SecurityTab_DeleteUser_Mutation {
      deleteUser
    }
  `);

  let updatePasswordOpen = $state(false);
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>보안</h1>
  </div>

  <!-- Password Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>로그인 수단</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          비밀번호
        {/snippet}
        {#snippet value()}
          <Button onclick={() => (updatePasswordOpen = true)} size="sm" variant="secondary">
            {$user.hasPassword ? '변경' : '설정'}
          </Button>
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>

  {#if $user.singleSignOns.length > 0}
    <!-- Connected Accounts Section -->
    <div>
      <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>연결된 SNS 계정</h2>

      <SettingsCard>
        {#each $user.singleSignOns as singleSignOn, index (singleSignOn.id)}
          {#if index > 0}
            <SettingsDivider />
          {/if}
          <SettingsRow>
            {#snippet label()}
              <div class={flex({ align: 'center', gap: '12px' })}>
                {#if singleSignOn.provider === 'GOOGLE'}
                  <Icon icon={GoogleIcon} size={24} />
                {:else if singleSignOn.provider === 'NAVER'}
                  <div
                    class={center({
                      borderWidth: '1px',
                      borderColor: '[#03C75A]',
                      borderRadius: '6px',
                      color: 'text.bright',
                      backgroundColor: '[#03C75A]',
                      size: '24px',
                    })}
                  >
                    <Icon icon={NaverIcon} size={14} />
                  </div>
                {:else if singleSignOn.provider === 'KAKAO'}
                  <div
                    class={center({
                      borderWidth: '1px',
                      borderColor: '[#FEE500]',
                      borderRadius: '6px',
                      color: '[#000000]',
                      backgroundColor: '[#FEE500]',
                      size: '24px',
                    })}
                  >
                    <Icon icon={KakaoIcon} size={18} />
                  </div>
                {/if}
                <span class={css({ textTransform: 'capitalize' })}>{singleSignOn.provider.toLowerCase()}</span>
              </div>
            {/snippet}
            {#snippet value()}
              <span>{singleSignOn.email}</span>
            {/snippet}
          </SettingsRow>
        {/each}
      </SettingsCard>
    </div>
  {/if}

  <!-- Account Deletion Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>계정 삭제</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          회원 탈퇴
        {/snippet}
        {#snippet description()}
          계정과 모든 데이터가 영구적으로 삭제되며 되돌릴 수 없어요.
        {/snippet}
        {#snippet value()}
          <Button
            onclick={async () => {
              Dialog.confirm({
                title: '정말로 탈퇴하시겠어요?',
                message: '탈퇴 시 모든 정보가 삭제되며, 복구할 수 없어요.',
                action: 'danger',
                actionLabel: '탈퇴',
                actionHandler: async () => {
                  await deleteUser();
                  mixpanel.track('delete_user');
                  globalThis.location.href = '/';
                },
              });
            }}
            size="sm"
            variant="ghost"
          >
            탈퇴하기
          </Button>
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>
</div>

<UpdatePasswordModal hasPassword={$user.hasPassword} bind:open={updatePasswordOpen} />
