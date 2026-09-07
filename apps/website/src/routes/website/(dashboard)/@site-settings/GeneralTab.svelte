<script lang="ts">
  import { createFragment, createMutation, createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, HorizontalDivider, Icon, RingSpinner, TextInput } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import CheckIcon from '~icons/lucide/check';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import UploadIcon from '~icons/lucide/upload';
  import { LoadableImg, SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { cache } from '$lib/graphql';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
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
      name: z.string({ error: '작업실 이름을 입력해주세요.' }).min(1, '작업실 이름을 입력해주세요.'),
      logoId: z.string(),
    }),
    onSubmit: async (data) => {
      if (!SubscribeModal.gate('site_settings')) {
        return;
      }

      await updateSite({ input: { siteId: site.data.id, name: data.name, logoId: data.logoId } });
      mixpanel.track('update_site');
      Toast.success('작업실 설정이 업데이트됐어요.');
    },
    defaultValues: {
      name: site.data.name,
      logoId: site.data.logo.id,
    },
  });

  $effect(() => {
    void form;
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
                  backgroundColor: 'surface.inverse/70',
                  color: 'text.on.inverse',
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

                if (!SubscribeModal.gate('site_settings')) {
                  return;
                }

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
            <p class={css({ fontSize: '12px', color: 'danger.default', textAlign: 'right' })}>{form.errors.name}</p>
          {/if}
        {/snippet}
      </SettingsRow>
    </form>
  </SettingsCard>

  <div class={css({ marginTop: '40px' })}>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>작업실 삭제</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          작업실 삭제
        {/snippet}
        {#snippet description()}
          작업실과 모든 데이터가 영구적으로 삭제되며 되돌릴 수 없어요.
        {/snippet}
        {#snippet value()}
          <div use:tooltip={{ message: canDeleteSite ? '' : '마지막 작업실은 삭제할 수 없어요' }}>
            <Button
              disabled={!canDeleteSite}
              loading={deleteSiteMutationResult.loading}
              onclick={() => {
                deleteOpen = true;
                deleteConfirmInput = '';
                deleteConfirmError = '';

                Dialog.confirm({
                  title: '정말로 삭제하시겠어요?',
                  message: '작업실의 모든 글과 데이터가 삭제되며, 복구할 수 없어요.',
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
        backgroundColor: 'surface.canvas',
      })}
    >
      <RingSpinner style={css.raw({ size: '13px', color: 'text.muted' })} />
      <span class={css({ fontSize: '13px', color: 'text.muted' })}>삭제될 항목 계산중...</span>
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
          backgroundColor: 'danger.subtle',
        })}
      >
        <Icon style={css.raw({ color: 'text.on.danger.subtle' })} icon={TriangleAlertIcon} size={14} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.on.danger.subtle' })}>
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
          backgroundColor: 'success.subtle',
        })}
      >
        <Icon style={css.raw({ color: 'text.on.success.subtle' })} icon={CheckIcon} size={14} />
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.on.success.subtle' })}>비어있는 작업실이에요</span>
      </div>
    {/if}

    {#if documents > 0}
      <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />
      <div class={flex({ flexDirection: 'column', gap: '6px' })}>
        <label class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.default' })} for="delete-confirm">
          삭제를 진행하려면 작업실과 함께 삭제되는 문서 수(
          <span class={css({ fontWeight: 'bold', color: 'danger.default' })}>{documents}</span>
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
          <p class={css({ fontSize: '12px', color: 'danger.default' })}>{deleteConfirmError}</p>
        {/if}
      </div>
    {/if}
  {/if}
{/snippet}
