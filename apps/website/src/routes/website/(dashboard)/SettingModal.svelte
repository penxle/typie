<script lang="ts">
  import * as PortOne from '@portone/browser-sdk/v2';
  import dayjs from 'dayjs';
  import { TypieError } from '@/errors';
  import UserIcon from '~icons/lucide/user';
  import { fragment, graphql } from '$graphql';
  import { Button, Dialog, Icon } from '$lib/components';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { DashboardLayout_SettingModal_user } from '$graphql';

  type Props = {
    open: boolean;
    $user: DashboardLayout_SettingModal_user;
  };

  let { open = $bindable(), $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_SettingModal_user on User {
        id

        personalIdentity {
          id
          expiresAt
        }
      }
    `),
  );

  const verifyPersonalIdentity = graphql(`
    mutation DashboardLayout_VerifyPersonalIdentity_Mutation($input: VerifyPersonalIdentityInput!) {
      verifyPersonalIdentity(input: $input) {
        id
        expiresAt
      }
    }
  `);

  const handleVerification = async () => {
    try {
      const resp = await PortOne.requestIdentityVerification({
        storeId: 'store-e1e69136-38bb-42dd-b226-3c78e03c1ff1',
        identityVerificationId: `identity-verification-${crypto.randomUUID()}`,
        channelKey: 'channel-key-31e03361-26cb-4810-86ed-801cce4f570f',
      });

      if (resp === undefined) {
        console.log('error');
        return;
      }

      await verifyPersonalIdentity({ identityVerificationId: resp.identityVerificationId });
    } catch (err) {
      const errorMessages: Record<string, string> = {
        identity_verification_failed: '인증에 실패했습니다.',
        identity_already_verified_by_other: '이미 다른 계정에 인증된 정보입니다.',
      };

      if (err instanceof TypieError) {
        const message = errorMessages[err.code] || err.code;
        Toast.error(message);
      }
    }
  };
</script>

<Dialog style={css.raw({ padding: '0', maxWidth: '1080px' })} bind:open>
  <div class={flex({ minHeight: '520px' })}>
    <div class={css({ flex: 'none', paddingY: '28px', paddingX: '20px', width: '240px', backgroundColor: 'gray.50' })}>
      <div class={flex({ align: 'center', gap: '4px', fontSize: '14px', color: 'gray.600' })}>
        <Icon icon={UserIcon} size={14} />
        계정
      </div>
    </div>

    <div class={css({ paddingY: '28px', paddingX: '24px', width: 'full' })}>
      <p class={css({ fontSize: '22px', fontWeight: '[700]' })}>계정 설정</p>

      <div class={css({ marginTop: '24px' })}>
        <p class={css({ fontWeight: 'medium' })}>본인 인증</p>

        <div class={css({ marginTop: '16px' })}>
          {#if $user.personalIdentity}
            <div class={css({ borderWidth: '1px', borderColor: 'gray.200', borderRadius: '4px', maxWidth: '360px' })}>
              <div
                class={css({
                  display: 'flex',
                  borderBottomWidth: '1px',
                  borderBottomColor: 'gray.200',
                  fontSize: '14px',
                  fontWeight: 'medium',
                  color: 'gray.800',
                })}
              >
                <div
                  class={css({
                    borderRightWidth: '1px',
                    borderRightColor: 'gray.200',
                    paddingY: '7px',
                    paddingX: '12px',
                    textAlign: 'center',
                    width: '80px',
                  })}
                >
                  인증 상태
                </div>

                <div class={css({ paddingY: '7px', paddingX: '12px' })}>만료 일자</div>
              </div>

              <div class={css({ display: 'flex' })}>
                <div
                  class={css({
                    display: 'flex',
                    justifyContent: 'center',
                    borderRightWidth: '1px',
                    borderRightColor: 'gray.200',
                    paddingY: '7px',
                    paddingX: '10px',
                    width: '80px',
                  })}
                >
                  <span
                    class={css({
                      display: 'block',
                      borderRadius: '2px',
                      paddingY: '2px',
                      paddingX: '6px',
                      fontSize: '12px',
                      fontWeight: 'medium',
                      backgroundColor: 'green.50',
                      color: 'green.800',
                    })}
                  >
                    인증 완료
                  </span>
                </div>
                <date
                  class={css({ display: 'flex', alignItems: 'center', paddingY: '4px', paddingX: '12px', fontSize: '13px' })}
                  datetime={$user.personalIdentity.expiresAt}
                >
                  {dayjs($user.personalIdentity.expiresAt).formatAsDate()}
                </date>
              </div>
            </div>
          {:else}
            <Button onclick={handleVerification} size="lg">본인인증</Button>
          {/if}
        </div>
      </div>
    </div>
  </div>
</Dialog>
