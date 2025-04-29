<script lang="ts">
  import dayjs from 'dayjs';
  import { z } from 'zod';
  import UploadIcon from '~icons/lucide/upload';
  import NaverIcon from '~icons/simple-icons/naver';
  import GoogleIcon from '~icons/typie/google';
  import KakaoIcon from '~icons/typie/kakao';
  import { fragment, graphql } from '$graphql';
  import { Button, HorizontalDivider, Icon, LoadableImg, Switch, TextInput } from '$lib/components';
  import { createForm } from '$lib/form';
  import { Dialog } from '$lib/notification';
  import { uploadBlobAsImage } from '$lib/utils';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import UpdateEmailModal from './UpdateEmailModal.svelte';
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
      name: z.string({ required_error: '이름을 입력해주세요.' }).nonempty('이름을 입력해주세요.'),
      avatarId: z.string(),
    }),
    onSubmit: async (data) => {
      await updateUser({ name: data.name, avatarId: data.avatarId });
    },
    defaultValues: {
      name: $user.name,
      avatarId: $user.avatar.id,
    },
  });

  let updateEmailOpen = $state(false);
</script>

<div class={flex({ direction: 'column', gap: '24px' })}>
  <p class={css({ fontSize: '20px', fontWeight: 'bold' })}>계정 설정</p>

  <div class={flex({ direction: 'column', gap: '8px' })}>
    <p class={css({ fontWeight: 'medium' })}>프로필</p>

    <form class={flex({ direction: 'column', gap: '12px', width: 'full', maxWidth: '500px' })} onsubmit={form.handleSubmit}>
      <label class={cx('group', center({ position: 'relative', size: '64px', cursor: 'pointer' }))}>
        <LoadableImg
          id={form.fields.avatarId}
          style={css.raw({ size: '64px', borderWidth: '1px', borderColor: 'gray.100', borderRadius: '12px' })}
          alt={`${$user.name}의 아바타`}
          size={64}
        />

        <div
          class={css({
            display: 'none',
            _groupHover: {
              position: 'absolute',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              borderRadius: '12px',
              size: 'full',
              backgroundColor: 'gray.900/16',
              color: 'white',
            },
          })}
        >
          <Icon icon={UploadIcon} size={28} />
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
              ensureAlpha: true,
              resize: { width: 512, height: 512, fit: 'contain', background: '#00000000' },
              format: 'png',
            });

            form.fields.avatarId = resp.id;
          }}
          type="file"
        />
      </label>

      <TextInput id="name" style={css.raw({ width: 'full' })} bind:value={form.fields.name} />

      {#if form.errors.name}
        <div class={css({ marginTop: '4px', paddingLeft: '4px', fontSize: '12px', color: 'red.500' })}>{form.errors.name}</div>
      {/if}

      <Button
        style={css.raw({ flex: 'none', marginLeft: 'auto', width: '104px', height: '38px' })}
        disabled={!form.state.isDirty}
        type="submit"
      >
        변경
      </Button>
    </form>
  </div>

  <HorizontalDivider color="secondary" />

  <div class={flex({ direction: 'column', gap: '8px' })}>
    <p class={css({ fontWeight: 'medium' })}>이메일 주소</p>

    <div class={flex({ gap: '12px', width: 'full', maxWidth: '500px' })}>
      <TextInput id="name" style={css.raw({ width: 'full' })} readonly bind:value={$user.email} />

      <Button style={css.raw({ flex: 'none', width: '104px', height: '38px' })} onclick={() => (updateEmailOpen = true)}>
        이메일 변경
      </Button>
    </div>
  </div>

  <HorizontalDivider color="secondary" />

  <div class={flex({ direction: 'column', gap: '12px' })}>
    <p class={css({ fontWeight: 'medium' })}>연결된 SNS 계정</p>

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
              color: 'white',
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
              color: 'black',
              backgroundColor: '[#FEE500]',
              size: '28px',
            })}
          >
            <Icon icon={KakaoIcon} size={20} />
          </div>
        {/if}

        <div>
          <p class={css({ fontSize: '15px', textTransform: 'capitalize' })}>{singleSignOn.provider.toLowerCase()}</p>
          <p class={css({ marginTop: '2px', fontSize: '14px', color: 'gray.500' })}>{singleSignOn.email}</p>
        </div>
      </div>
    {/each}
  </div>

  <HorizontalDivider color="secondary" />

  <div class={flex({ align: 'center', justify: 'space-between', width: 'full', maxWidth: '500px' })}>
    <p class={css({ fontWeight: 'medium' })}>마케팅 수신 동의</p>

    <Switch
      checked={$user.marketingConsent}
      onchange={async () => {
        await updateMarketingConsent({ marketingConsent: !$user.marketingConsent });

        Dialog.alert({
          title: '타이피 마케팅 수신 동의',
          message: `${dayjs().formatAsDate()}에 ${$user.marketingConsent ? '거부' : '동의'}처리되었어요`,
        });
      }}
    />
  </div>

  <HorizontalDivider color="secondary" />

  <button
    class={css({ padding: '4px', fontSize: '13px', color: 'gray.400', width: 'fit' })}
    onclick={async () => {
      Dialog.confirm({
        title: '정말로 탈퇴하시겠습니까?',
        message: '탈퇴 시 모든 정보가 삭제되며, 복구할 수 없어요.',
        action: 'danger',
        actionLabel: '탈퇴',
        actionHandler: async () => {
          await deleteUser();
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
