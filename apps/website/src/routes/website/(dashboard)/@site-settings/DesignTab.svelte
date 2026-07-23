<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { SiteDateDisplay } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { Select } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { SettingsCard, SettingsRow } from '$lib/components';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import type { DashboardLayout_SiteSettingsModal_DesignTab_site$key } from '$mearie';

  type Props = {
    site$key: DashboardLayout_SiteSettingsModal_DesignTab_site$key;
  };

  let { site$key }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_DesignTab_site on Site {
        id
        dateDisplay
      }
    `),
    () => site$key,
  );

  const [updateSite] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_DesignTab_UpdateSite_Mutation($input: UpdateSiteInput!) {
        updateSite(input: $input) {
          id
          dateDisplay
        }
      }
    `),
  );
</script>

<div class={css({ maxWidth: '640px' })}>
  <div class={css({ marginBottom: '24px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>디자인</h1>
  </div>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}
        글 목록에 표시할 날짜
      {/snippet}
      {#snippet value()}
        <Select
          items={[
            { label: '최초 생성 시각', value: SiteDateDisplay.CREATED_AT },
            { label: '마지막 수정 시각', value: SiteDateDisplay.UPDATED_AT },
            { label: '미표시', value: SiteDateDisplay.NONE },
          ]}
          onselect={async (value) => {
            if (!SubscribeModal.gate('site_settings')) {
              return;
            }

            await updateSite({ input: { siteId: site.data.id, dateDisplay: value } });
            Toast.success('날짜 표시 설정이 변경됐어요.');
          }}
          value={site.data.dateDisplay}
        />
      {/snippet}
    </SettingsRow>
  </SettingsCard>
</div>
