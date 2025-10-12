<script lang="ts">
  import * as PortOne from '@portone/browser-sdk/v2';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Switch, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Dialog, Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { siteSchema } from '@/validation';
  import CheckCircle2Icon from '~icons/lucide/check-circle-2';
  import PencilIcon from '~icons/lucide/pencil';
  import UploadIcon from '~icons/lucide/upload';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import { LoadableImg, SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { uploadBlobAsImage } from '$lib/utils';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import UpdateEmailModal from './UpdateEmailModal.svelte';
  import type { DashboardLayout_PreferenceModal_ProfileTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_ProfileTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_ProfileTab_user on User {
        id
        name
        email
        marketingConsent

        avatar {
          id
          ...Img_image
        }

        sites {
          id
          slug
        }

        subscription {
          id
        }

        personalIdentity {
          id
          expiresAt
        }
      }
    `),
  );

  const updateUser = graphql(`
    mutation DashboardLayout_PreferenceModal_ProfileTab_UpdateUser_Mutation($input: UpdateUserInput!) {
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
    mutation DashboardLayout_PreferenceModal_ProfileTab_UpdateMarketingConsent_Mutation($input: UpdateMarketingConsentInput!) {
      updateMarketingConsent(input: $input) {
        id
        marketingConsent
      }
    }
  `);

  const updateSiteSlug = graphql(`
    mutation DashboardLayout_PreferenceModal_ProfileTab_UpdateSiteSlug_Mutation($input: UpdateSiteSlugInput!) {
      updateSiteSlug(input: $input) {
        id
        slug
      }
    }
  `);

  const verifyPersonalIdentity = graphql(`
    mutation DashboardLayout_PreferenceModal_ProfileTab_VerifyPersonalIdentity_Mutation($input: VerifyPersonalIdentityInput!) {
      verifyPersonalIdentity(input: $input) {
        id

        personalIdentity {
          id
          expiresAt
        }
      }
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
      Toast.success('프로필이 업데이트됐어요.');
    },
    defaultValues: {
      name: $user.name,
      avatarId: $user.avatar.id,
    },
  });

  const siteForm = createForm({
    schema: z.object({
      slug: siteSchema.slug,
    }),
    onSubmit: async (data) => {
      await updateSiteSlug({ siteId: $user.sites[0].id, slug: data.slug });

      mixpanel.track('update_site_slug');
      Toast.success('사이트 주소가 변경됐어요.');
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'site_slug_already_exists') {
        throw new FormError('slug', '이미 존재하는 사이트 주소예요.');
      }
    },
    defaultValues: {
      slug: $user.sites[0]?.slug ?? '',
    },
  });

  $effect(() => {
    void form;
    void siteForm;
  });

  const handleVerification = async () => {
    try {
      mixpanel.track('verify_personal_identity_start');

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

      mixpanel.track('verify_personal_identity_success');
    } catch (err) {
      const errorMessages: Record<string, string> = {
        identity_verification_failed: '인증에 실패했어요.',
        same_identity_exists: '이미 다른 계정에 인증된 정보예요.',
      };

      if (err instanceof TypieError) {
        const message = errorMessages[err.code] || err.code;
        Toast.error(message);
      }
    }
  };

  let updateEmailOpen = $state(false);
  let planUpgradeModalOpen = $state(false);
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>프로필</h1>
  </div>

  <!-- Profile Section -->
  <div>
    <SettingsCard>
      <form onsubmit={form.handleSubmit}>
        <!-- Profile Picture Row -->
        <SettingsRow>
          {#snippet label()}
            프로필 사진
          {/snippet}
          {#snippet value()}
            <label class={cx('group', center({ position: 'relative', size: '32px', cursor: 'pointer' }))}>
              <LoadableImg
                id={form.fields.avatarId}
                style={css.raw({ size: '32px', borderRadius: 'full' })}
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
                    borderRadius: 'full',
                    size: 'full',
                    backgroundColor: 'gray.900/60',
                    color: 'text.bright',
                  },
                })}
              >
                <Icon icon={UploadIcon} size={14} />
              </div>
              <input
                accept="image/*"
                hidden
                onchange={async (event) => {
                  const file = event.currentTarget.files?.[0];
                  event.currentTarget.value = '';
                  if (!file) return;

                  const resp = await uploadBlobAsImage(file, {
                    resize: { width: 512, height: 512, fit: 'cover', withoutEnlargement: true },
                    format: 'png',
                  });
                  form.fields.avatarId = resp.id;
                }}
                type="file"
              />
            </label>
          {/snippet}
        </SettingsRow>

        <SettingsDivider />

        <!-- Email Row -->
        <SettingsRow>
          {#snippet label()}
            이메일
          {/snippet}
          {#snippet value()}
            <div class={flex({ align: 'center', gap: '10px' })}>
              <span>{$user.email}</span>
              <button
                class={css({
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  size: '22px',
                  color: 'text.subtle',
                  cursor: 'pointer',
                  borderRadius: '4px',
                  transition: 'common',
                  _hover: { color: 'text.default', backgroundColor: 'surface.muted' },
                })}
                onclick={() => (updateEmailOpen = true)}
                type="button"
              >
                <Icon icon={PencilIcon} size={12} />
              </button>
            </div>
          {/snippet}
        </SettingsRow>

        <SettingsDivider />

        <!-- Full Name Row -->
        <SettingsRow>
          {#snippet label()}
            이름
          {/snippet}
          {#snippet value()}
            <TextInput
              style={css.raw({ width: '[200px]', height: '32px', fontSize: '13px' })}
              onblur={() => {
                if (form.state.isDirty) {
                  form.handleSubmit();
                }
              }}
              bind:value={form.fields.name}
            />
          {/snippet}
          {#snippet error()}
            {#if form.errors.name}
              <p class={css({ fontSize: '12px', color: 'text.danger', textAlign: 'right' })}>{form.errors.name}</p>
            {/if}
          {/snippet}
        </SettingsRow>

        {#if $user.sites.length > 0}
          <SettingsDivider />

          <!-- Site URL Row -->
          <SettingsRow>
            {#snippet label()}
              사이트 주소
            {/snippet}
            {#snippet description()}
              포스트를 공유할 때 사용할 주소예요.
            {/snippet}
            {#snippet value()}
              <div class={css({ position: 'relative' })}>
                <TextInput
                  style={css.raw({ width: '[280px]', height: '32px', fontSize: '13px' })}
                  disabled={!$user.subscription}
                  onblur={() => {
                    if ($user.subscription && siteForm.state.isDirty) {
                      siteForm.handleSubmit();
                    }
                  }}
                  rightItemAttached
                  bind:value={siteForm.fields.slug}
                >
                  {#snippet rightItem()}
                    <span
                      class={css({
                        fontSize: '13px',
                        color: 'text.subtle',
                        backgroundColor: 'surface.muted',
                        paddingX: '12px',
                        height: 'full',
                        display: 'flex',
                        alignItems: 'center',
                      })}
                    >
                      .{env.PUBLIC_USERSITE_HOST}
                    </span>
                  {/snippet}
                </TextInput>
                {#if !$user.subscription}
                  <button
                    class={css({
                      position: 'absolute',
                      inset: '0',
                      cursor: 'pointer',
                      backgroundColor: 'transparent',
                      border: 'none',
                    })}
                    aria-label="사이트 주소 기능 업그레이드"
                    onclick={() => {
                      planUpgradeModalOpen = true;
                    }}
                    type="button"
                  ></button>
                {/if}
              </div>
            {/snippet}
            {#snippet error()}
              {#if siteForm.errors.slug}
                <p class={css({ fontSize: '12px', color: 'text.danger', textAlign: 'right' })}>{siteForm.errors.slug}</p>
              {/if}
            {/snippet}
          </SettingsRow>
        {/if}
      </form>
    </SettingsCard>
  </div>

  <!-- Account Security Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>인증</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          본인 인증
        {/snippet}
        {#snippet description()}
          일부 서비스 이용 시 실명 확인이 요구될 수 있어요.
        {/snippet}
        {#snippet value()}
          {#if $user.personalIdentity}
            <div class={flex({ align: 'center', gap: '6px' })}>
              <Icon style={css.raw({ color: 'text.success' })} icon={CheckCircle2Icon} size={12} />
              <span class={css({ color: 'text.success' })}>인증 완료</span>
            </div>
          {:else}
            <Button onclick={handleVerification} size="sm" variant="secondary">인증하기</Button>
          {/if}
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>

  <!-- Notifications Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>알림</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          마케팅 수신
        {/snippet}
        {#snippet description()}
          새로운 기능과 이벤트 소식을 받아요.
        {/snippet}
        {#snippet value()}
          <Switch
            checked={$user.marketingConsent}
            onchange={async () => {
              await updateMarketingConsent({ marketingConsent: !$user.marketingConsent });
              mixpanel.track('update_marketing_consent', { marketingConsent: !$user.marketingConsent });
              Dialog.alert({
                title: '마케팅 수신 동의',
                message: `${dayjs().formatAsDate()}에 ${$user.marketingConsent ? '거부' : '동의'}처리됐어요.`,
              });
            }}
          />
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>

  <!-- Support Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>지원</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          계정 ID
        {/snippet}
        {#snippet description()}
          문의나 지원 요청 시 이 ID를 알려주시면 더 빠르게 도와드릴 수 있어요.
        {/snippet}
        {#snippet value()}
          <div class={css({ fontSize: '12px', fontFamily: 'mono', color: 'text.subtle', letterSpacing: '[0]' })}>
            {$user.id}
          </div>
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>
</div>

<UpdateEmailModal email={$user.email} bind:open={updateEmailOpen} />
<PlanUpgradeModal bind:open={planUpgradeModalOpen}>사이트 주소 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.</PlanUpgradeModal>
