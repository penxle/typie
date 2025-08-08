<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import UploadIcon from '~icons/lucide/upload';
  import NaverIcon from '~icons/simple-icons/naver';
  import GoogleIcon from '~icons/typie/google';
  import KakaoIcon from '~icons/typie/kakao';
  import { fragment, graphql } from '$graphql';
  import { Button, Icon, LoadableImg, Switch, TextInput } from '$lib/components';
  import { createForm } from '$lib/form';
  import { Dialog } from '$lib/notification';
  import { uploadBlobAsImage } from '$lib/utils';
  import UpdateEmailModal from './UpdateEmailModal.svelte';
  import UpdatePasswordModal from './UpdatePasswordModal.svelte';
  import type { DashboardLayout_PreferenceModal_AccountTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_AccountTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_AccountTab_user on User {
        id
        name
        email
        marketingConsent
        hasPassword

        singleSignOns {
          id
          email
          provider
        }

        avatar {
          id
          ...Img_image
        }
      }
    `),
  );

  const updateUser = graphql(`
    mutation DashboardLayout_PreferenceModal_UpdateUser_Mutation($input: UpdateUserInput!) {
      updateUser(input: $input) {
        id
        name

        avatar {
          id
        }
      }
    }
  `);

  const updateMarketingConsent = graphql(`
    mutation DashboardLayout_PreferenceModal_UpdateMarketingConsent_Mutation($input: UpdateMarketingConsentInput!) {
      updateMarketingConsent(input: $input) {
        id
        marketingConsent
      }
    }
  `);

  const deleteUser = graphql(`
    mutation DashboardLayout_PreferenceModal_DeleteUser_Mutation {
      deleteUser
    }
  `);

  const form = createForm({
    schema: z.object({
      name: z.string({ error: '이름을 입력해주세요.' }).min(1, '이름을 입력해주세요.'),
      avatarId: z.string(),
    }),
    onSubmit: async (data) => {
      await updateUser({ name: data.name, avatarId: data.avatarId });
      mixpanel.track('update_user');
    },
    defaultValues: {
      name: $user.name,
      avatarId: $user.avatar.id,
    },
  });

  $effect(() => {
    void form;
  });

  let updateEmailOpen = $state(false);
  let updatePasswordOpen = $state(false);
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>계정</h1>

  <div class={flex({ direction: 'column', gap: '16px' })}>
    <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>프로필</h3>

    <form class={flex({ direction: 'column', gap: '20px', width: 'full' })} onsubmit={form.handleSubmit}>
      <div class={flex({ align: 'center', gap: '24px' })}>
        <label class={cx('group', center({ position: 'relative', size: '64px', cursor: 'pointer' }))}>
          <LoadableImg
            id={form.fields.avatarId}
            style={css.raw({ size: '64px', borderWidth: '1px', borderColor: 'border.default', borderRadius: 'full' })}
            alt={`${$user.name}의 아바타`}
            size={128}
          />

          <div
            class={css({
              display: 'none',
              _groupHover: {
                position: 'absolute',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                borderRadius: 'full',
                size: 'full',
                backgroundColor: 'gray.900/40',
                color: 'text.bright',
              },
            })}
          >
            <Icon icon={UploadIcon} size={24} />
          </div>

          <input
            accept="image/*"
            hidden
            onchange={async (event) => {
              const file = event.currentTarget.files?.[0];
              event.currentTarget.value = '';
              if (!file) {
                return;
              }

              const resp = await uploadBlobAsImage(file, {
                resize: { width: 512, height: 512, fit: 'cover', withoutEnlargement: true },
                format: 'png',
              });

              form.fields.avatarId = resp.id;
            }}
            type="file"
          />
        </label>

        <div class={flex({ direction: 'column', gap: '8px', flex: '1' })}>
          <TextInput id="name" style={css.raw({ width: 'full' })} bind:value={form.fields.name} />

          {#if form.errors.name}
            <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.name}</div>
          {/if}
        </div>
      </div>

      <Button style={css.raw({ alignSelf: 'flex-start', height: '36px' })} disabled={!form.state.isDirty} type="submit" variant="secondary">
        변경사항 저장
      </Button>
    </form>
  </div>

  <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>

  <div class={flex({ direction: 'column', gap: '12px' })}>
    <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>이메일 주소</h3>

    <div class={flex({ gap: '12px', width: 'full' })}>
      <TextInput id="email" style={css.raw({ width: 'full', backgroundColor: 'surface.subtle' })} readonly bind:value={$user.email} />

      <Button style={css.raw({ flex: 'none', height: '36px' })} onclick={() => (updateEmailOpen = true)} size="sm" variant="secondary">
        변경
      </Button>
    </div>
  </div>

  <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>

  <div class={flex({ align: 'center', justify: 'space-between' })}>
    <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>비밀번호</h3>

    <Button style={css.raw({ flex: 'none', height: '36px' })} onclick={() => (updatePasswordOpen = true)} size="sm" variant="secondary">
      {$user.hasPassword ? '변경' : '설정하기'}
    </Button>
  </div>

  {#if $user.singleSignOns.length > 0}
    <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>

    <div class={flex({ direction: 'column', gap: '16px' })}>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>연결된 SNS 계정</h3>

      {#each $user.singleSignOns as singleSignOn (singleSignOn.id)}
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
                size: '28px',
              })}
            >
              <Icon icon={NaverIcon} size={16} />
            </div>
          {:else if singleSignOn.provider === 'KAKAO'}
            <div
              class={center({
                borderWidth: '1px',
                borderColor: '[#FEE500]',
                borderRadius: '6px',
                color: '[#000000]',
                backgroundColor: '[#FEE500]',
                size: '28px',
              })}
            >
              <Icon icon={KakaoIcon} size={20} />
            </div>
          {/if}

          <div>
            <p class={css({ fontSize: '15px', textTransform: 'capitalize' })}>{singleSignOn.provider.toLowerCase()}</p>
            <p class={css({ marginTop: '2px', fontSize: '14px', color: 'text.faint' })}>{singleSignOn.email}</p>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>

  <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
    <div>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>마케팅 수신 동의</h3>
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>타이피의 소식과 이벤트 정보를 받아보세요</p>
    </div>

    <Switch
      checked={$user.marketingConsent}
      onchange={async () => {
        await updateMarketingConsent({ marketingConsent: !$user.marketingConsent });

        mixpanel.track('update_marketing_consent', { marketingConsent: !$user.marketingConsent });

        Dialog.alert({
          title: '타이피 마케팅 수신 동의',
          message: `${dayjs().formatAsDate()}에 ${$user.marketingConsent ? '거부' : '동의'}처리되었어요`,
        });
      }}
    />
  </div>

  <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>

  <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
    <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>계정 ID</h3>

    <div class={css({ fontSize: '13px', fontFamily: 'mono', color: 'text.faint', letterSpacing: '[0]' })}>
      {$user.id}
    </div>
  </div>

  <div class={css({ height: '1px', backgroundColor: 'surface.muted' })}></div>

  <button
    class={css({
      alignSelf: 'flex-start',
      paddingX: '8px',
      paddingY: '4px',
      fontSize: '13px',
      color: 'text.faint',
      width: 'fit',
      borderRadius: '4px',
      transition: 'common',
      _hover: { color: 'text.danger', backgroundColor: 'accent.danger.subtle' },
    })}
    onclick={async () => {
      Dialog.confirm({
        title: '정말로 탈퇴하시겠습니까?',
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
    type="button"
  >
    회원 탈퇴
  </button>
</div>

<UpdateEmailModal email={$user.email} bind:open={updateEmailOpen} />
<UpdatePasswordModal hasPassword={$user.hasPassword} bind:open={updatePasswordOpen} />
