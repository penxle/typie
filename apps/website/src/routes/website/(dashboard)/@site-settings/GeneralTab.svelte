<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { Icon, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { siteSchema } from '@/validation';
  import UploadIcon from '~icons/lucide/upload';
  import { env } from '$env/dynamic/public';
  import { LoadableImg, SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import type {
    DashboardLayout_SiteSettingsModal_GeneralTab_site$key,
    DashboardLayout_SiteSettingsModal_GeneralTab_user$key,
  } from '$mearie';

  type Props = {
    site$key: DashboardLayout_SiteSettingsModal_GeneralTab_site$key;
    user$key: DashboardLayout_SiteSettingsModal_GeneralTab_user$key;
  };

  let { site$key, user$key }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_GeneralTab_site on Site {
        id
        name
        slug

        logo {
          id
          ...Img_image
        }
      }
    `),
    () => site$key,
  );

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_GeneralTab_user on User {
        id
        ...DashboardLayout_PlanUpgradeModal_user

        subscription {
          id
        }
      }
    `),
    () => user$key,
  );

  const [updateSite] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_GeneralTab_UpdateSite_Mutation($input: UpdateSiteInput!) {
        updateSite(input: $input) {
          id
          name

          logo {
            id
            ...Img_image
          }
        }
      }
    `),
  );

  const [updateSiteSlug] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_GeneralTab_UpdateSiteSlug_Mutation($input: UpdateSiteSlugInput!) {
        updateSiteSlug(input: $input) {
          id
          slug
        }
      }
    `),
  );

  const form = createForm({
    schema: z.object({
      name: z.string({ error: '스페이스 이름을 입력해주세요.' }).min(1, '스페이스 이름을 입력해주세요.'),
      logoId: z.string(),
    }),
    onSubmit: async (data) => {
      await updateSite({ input: { siteId: site.data.id, name: data.name, logoId: data.logoId } });
      mixpanel.track('update_site');
      Toast.success('스페이스 설정이 업데이트됐어요.');
    },
    defaultValues: {
      name: site.data.name,
      logoId: site.data.logo.id,
    },
  });

  const slugForm = createForm({
    schema: z.object({
      slug: siteSchema.slug,
    }),
    onSubmit: async (data) => {
      await updateSiteSlug({ input: { siteId: site.data.id, slug: data.slug } });
      mixpanel.track('update_site_slug');
      Toast.success('스페이스 주소가 변경됐어요.');
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'site_slug_already_exists') {
        throw new FormError('slug', '이미 존재하는 스페이스 주소예요.');
      }
    },
    defaultValues: {
      slug: site.data.slug,
    },
  });

  $effect(() => {
    void form;
    void slugForm;
  });

  let planUpgradeModalOpen = $state(false);
</script>

<div class={css({ maxWidth: '640px' })}>
  <div class={css({ marginBottom: '24px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>일반</h1>
  </div>

  <SettingsCard>
    <form onsubmit={form.handleSubmit}>
      <SettingsRow>
        {#snippet label()}
          로고
        {/snippet}
        {#snippet value()}
          <label class={cx('group', center({ position: 'relative', size: '32px', cursor: 'pointer' }))}>
            <LoadableImg id={form.fields.logoId} style={css.raw({ size: '32px', borderRadius: '4px' })} alt={site.data.name} size={64} />
            <div
              class={css({
                display: 'none',
                _groupHover: {
                  position: 'absolute',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  borderRadius: '4px',
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
                form.fields.logoId = resp.id;
                form.handleSubmit();
              }}
              type="file"
            />
          </label>
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

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

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          주소
        {/snippet}
        {#snippet value()}
          <div class={css({ position: 'relative' })}>
            <TextInput
              style={css.raw({ width: '[280px]', height: '32px', fontSize: '13px' })}
              disabled={!user.data.subscription}
              onblur={() => {
                if (user.data.subscription && slugForm.state.isDirty) {
                  slugForm.handleSubmit();
                }
              }}
              rightItemAttached
              bind:value={slugForm.fields.slug}
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
            {#if !user.data.subscription}
              <button
                class={css({
                  position: 'absolute',
                  inset: '0',
                  cursor: 'pointer',
                  backgroundColor: 'transparent',
                  border: 'none',
                })}
                aria-label="스페이스 주소 기능 업그레이드"
                onclick={() => {
                  planUpgradeModalOpen = true;
                  mixpanel.track('open_plan_upgrade_modal', { via: 'site_address' });
                }}
                type="button"
              ></button>
            {/if}
          </div>
        {/snippet}
        {#snippet error()}
          {#if slugForm.errors.slug}
            <p class={css({ fontSize: '12px', color: 'text.danger', textAlign: 'right' })}>{slugForm.errors.slug}</p>
          {/if}
        {/snippet}
      </SettingsRow>
    </form>
  </SettingsCard>
</div>

<PlanUpgradeModal user$key={user.data} bind:open={planUpgradeModalOpen}>
  스페이스 주소 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.
</PlanUpgradeModal>
