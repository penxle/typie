<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Switch } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import type { DashboardLayout_SiteSettingsModal_VisibilityTab_site$key } from '$mearie';

  type Props = {
    site$key: DashboardLayout_SiteSettingsModal_VisibilityTab_site$key;
  };

  let { site$key }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_VisibilityTab_site on Site {
        id
        allowIndexing
        allowDiscovery
      }
    `),
    () => site$key,
  );

  const [updateSite] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_VisibilityTab_UpdateSite_Mutation($input: UpdateSiteInput!) {
        updateSite(input: $input) {
          id
          allowIndexing
          allowDiscovery
        }
      }
    `),
  );

  let allowIndexing = $state(site.data.allowIndexing);
  let allowDiscovery = $state(site.data.allowDiscovery);

  $effect(() => {
    allowIndexing = site.data.allowIndexing;
    allowDiscovery = site.data.allowDiscovery;
  });

  const save = async (input: { allowIndexing?: boolean; allowDiscovery?: boolean }, field: string) => {
    if (!SubscribeModal.gate('site_settings')) return false;

    try {
      await updateSite({ input: { siteId: site.data.id, ...input } });
      mixpanel.track('update_site', { field });
      Toast.success('스페이스 설정이 업데이트됐어요.');
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    }
  };

  const setAllowIndexing = async (checked: boolean) => {
    allowIndexing = checked;
    if (!(await save({ allowIndexing: checked }, 'allowIndexing'))) {
      await tick();
      allowIndexing = site.data.allowIndexing;
    }
  };

  const setAllowDiscovery = async (checked: boolean) => {
    allowDiscovery = checked;
    if (!(await save({ allowDiscovery: checked }, 'allowDiscovery'))) {
      await tick();
      allowDiscovery = site.data.allowDiscovery;
    }
  };
</script>

<div class={css({ maxWidth: '640px' })}>
  <div class={css({ marginBottom: '24px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>노출</h1>
  </div>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}
        검색 엔진에 노출
      {/snippet}
      {#snippet description()}
        끄면 검색 엔진이 스페이스와 글을 색인하지 않도록 요청해요.
      {/snippet}
      {#snippet value()}
        <Switch bind:checked={() => allowIndexing, (checked) => void setAllowIndexing(checked)} />
      {/snippet}
    </SettingsRow>

    <SettingsDivider />

    <SettingsRow>
      {#snippet label()}
        타이피 스퀘어에 노출
      {/snippet}
      {#snippet description()}
        끄면 타이피 스퀘어 피드 및 검색에 스페이스와 글이 나오지 않아요.
      {/snippet}
      {#snippet value()}
        <Switch bind:checked={() => allowDiscovery, (checked) => void setAllowDiscovery(checked)} />
      {/snippet}
    </SettingsRow>
  </SettingsCard>
</div>
