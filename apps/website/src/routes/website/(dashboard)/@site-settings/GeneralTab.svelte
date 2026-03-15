<script lang="ts">
  import { createFragment, createMutation, createQuery } from '@mearie/svelte';
  import { TypieError } from '@typie/lib/errors';
  import { siteSchema } from '@typie/lib/validation';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, HorizontalDivider, Icon, RingSpinner, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import CheckIcon from '~icons/lucide/check';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import UploadIcon from '~icons/lucide/upload';
  import { env } from '$env/dynamic/public';
  import { LoadableImg, SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { cache } from '$lib/graphql';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import { PlanUpgradeDialog } from '../plan-upgrade-dialog.svelte';
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
        subscription {
          id
        }

        sites {
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

  const [deleteSite, deleteSiteMutationResult] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_GeneralTab_DeleteSite_Mutation($input: DeleteSiteInput!) {
        deleteSite(input: $input) {
          id
        }
      }
    `),
  );

  let deleteOpen = $state(false);

  const siteInfo = createQuery(
    graphql(`
      query DashboardLayout_SiteSettingsModal_GeneralTab_SiteInfo_Query($siteId: ID!) {
        site(siteId: $siteId) {
          id
          folderCount
          documentCount
        }
      }
    `),
    () => ({ siteId: site.data.id }),
    () => ({ skip: !deleteOpen }),
  );

  const canDeleteSite = $derived(user.data.sites.length > 1);

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

  let deleteConfirmInput = $state('');
  let deleteConfirmError = $state('');
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
                  PlanUpgradeDialog.show({
                    message: '스페이스 주소 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.',
                  });
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

  <div class={css({ marginTop: '40px' })}>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>스페이스 삭제</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          스페이스 삭제
        {/snippet}
        {#snippet description()}
          스페이스와 모든 데이터가 영구적으로 삭제되며 되돌릴 수 없어요.
        {/snippet}
        {#snippet value()}
          <div use:tooltip={{ message: canDeleteSite ? '' : '마지막 스페이스는 삭제할 수 없어요' }}>
            <Button
              disabled={!canDeleteSite}
              loading={deleteSiteMutationResult.loading}
              onclick={() => {
                deleteOpen = true;
                deleteConfirmInput = '';
                deleteConfirmError = '';

                Dialog.confirm({
                  title: '정말로 삭제하시겠어요?',
                  message: '스페이스의 모든 글과 데이터가 삭제되며, 복구할 수 없어요.',
                  children: deleteInfoView,
                  action: 'danger',
                  actionLabel: '삭제',
                  actionHandler: async () => {
                    if (!siteInfo.data) return false;

                    const documentCount = siteInfo.data.site.documentCount;
                    if (documentCount > 0 && deleteConfirmInput !== String(documentCount)) {
                      deleteConfirmError = '삭제되는 문서 수를 정확히 입력해주세요.';
                      return false;
                    }

                    await deleteSite({ input: { siteId: site.data.id } });
                    cache.invalidate({ __typename: 'User', id: user.data.id, $field: 'sites' });
                    mixpanel.track('delete_site');
                    history.back();
                  },
                  onclose: () => {
                    deleteOpen = false;
                  },
                });
              }}
              size="sm"
              variant="ghost"
            >
              삭제하기
            </Button>
          </div>
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>
</div>

{#snippet deleteInfoView()}
  {#if siteInfo.loading}
    <div
      class={flex({
        alignItems: 'center',
        gap: '6px',
        borderRadius: '8px',
        paddingX: '12px',
        paddingY: '8px',
        backgroundColor: 'surface.subtle',
      })}
    >
      <RingSpinner style={css.raw({ size: '13px', color: 'text.faint' })} />
      <span class={css({ fontSize: '13px', color: 'text.faint' })}>삭제될 항목 계산중...</span>
    </div>
  {:else if siteInfo.data}
    {@const folders = siteInfo.data.site.folderCount}
    {@const documents = siteInfo.data.site.documentCount}

    {#if folders > 0 || documents > 0}
      {@const items = [folders > 0 && `${folders}개의 폴더`, documents > 0 && `${documents}개의 문서`].filter(Boolean)}
      <div
        class={flex({
          alignItems: 'center',
          gap: '6px',
          borderRadius: '8px',
          paddingX: '12px',
          paddingY: '8px',
          backgroundColor: 'accent.danger.subtle',
        })}
      >
        <Icon style={css.raw({ color: 'text.danger' })} icon={TriangleAlertIcon} size={14} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.danger' })}>
          {items.join('와 ')}가 함께 삭제돼요
        </span>
      </div>
    {:else}
      <div
        class={flex({
          alignItems: 'center',
          gap: '6px',
          borderRadius: '8px',
          paddingX: '12px',
          paddingY: '8px',
          backgroundColor: 'accent.success.subtle',
        })}
      >
        <Icon style={css.raw({ color: 'text.success' })} icon={CheckIcon} size={14} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.success' })}>비어있는 스페이스에요</span>
      </div>
    {/if}

    {#if documents > 0}
      <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />
      <div class={flex({ flexDirection: 'column', gap: '6px' })}>
        <label class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.default' })} for="delete-confirm">
          삭제를 진행하려면 스페이스와 함께 삭제되는 문서 수(
          <span class={css({ fontWeight: 'bold', color: 'text.danger' })}>{documents}</span>
          )를 입력해주세요.
        </label>
        <TextInput
          id="delete-confirm"
          style={css.raw({ fontSize: '13px' })}
          oninput={() => {
            deleteConfirmError = '';
          }}
          placeholder={String(documents)}
          bind:value={deleteConfirmInput}
        />
        {#if deleteConfirmError}
          <p class={css({ fontSize: '12px', color: 'text.danger' })}>{deleteConfirmError}</p>
        {/if}
      </div>
    {/if}
  {/if}
{/snippet}
