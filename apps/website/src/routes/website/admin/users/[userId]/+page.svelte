<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '@typie/ui/components';
  import { QueryString } from '@typie/ui/state';
  import { userRoleLabels, userRoleTones, userStateLabels, userStateTones } from '$lib/admin-labels';
  import { AdminBadge, AdminPageHeader, AdminTabs } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';
  import { graphql } from '$mearie';
  import BillingTab from './BillingTab.svelte';
  import ContentsTab from './ContentsTab.svelte';
  import OverviewTab from './OverviewTab.svelte';
  import SessionsTab from './SessionsTab.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const currentTab = new QueryString('tab', 'overview');

  let impersonateModalOpen = $state(false);

  const [adminImpersonate] = createMutation(
    graphql(`
      mutation AdminUserDetail_AdminImpersonate_Mutation($input: AdminImpersonateInput!) {
        adminImpersonate(input: $input)
      }
    `),
  );

  const handleImpersonate = async () => {
    await adminImpersonate({ input: { userId: query.data.adminUser.id } });
    location.assign('/initial');
  };
</script>

<AdminPageHeader description={query.data.adminUser.email} title={query.data.adminUser.name}>
  {#snippet badges()}
    <AdminBadge label={userStateLabels[query.data.adminUser.state]} tone={userStateTones[query.data.adminUser.state]} />
    <AdminBadge label={userRoleLabels[query.data.adminUser.role]} tone={userRoleTones[query.data.adminUser.role]} />
    <AdminBadge
      label={query.data.adminUser.hasActiveSubscription ? '구독 권한 있음' : '구독 권한 없음'}
      tone={query.data.adminUser.hasActiveSubscription ? 'success' : 'neutral'}
    />
  {/snippet}
  {#snippet actions()}
    <Button onclick={() => (impersonateModalOpen = true)} size="sm" variant="secondary">이 유저로 접속</Button>
  {/snippet}
</AdminPageHeader>

<AdminTabs
  tabs={[
    { key: 'overview', label: '개요' },
    { key: 'contents', label: '콘텐츠' },
    { key: 'billing', label: '결제' },
    { key: 'sessions', label: '접속' },
  ]}
  bind:current={currentTab.current}
/>

{#if currentTab.current === 'overview'}
  <OverviewTab user$key={query.data.adminUser} />
{:else if currentTab.current === 'contents'}
  <ContentsTab user$key={query.data.adminUser} />
{:else if currentTab.current === 'billing'}
  <BillingTab user$key={query.data.adminUser} />
{:else if currentTab.current === 'sessions'}
  <SessionsTab user$key={query.data.adminUser} />
{/if}

<Modal style={css.raw({ padding: '24px', maxWidth: '400px' })} bind:open={impersonateModalOpen}>
  <div class={flex({ flexDirection: 'column', gap: '24px' })}>
    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', color: 'text.default' })}>이 유저로 접속할까요?</div>
      <div class={css({ fontSize: '13px', color: 'text.faint', wordBreak: 'keep-all' })}>
        {query.data.adminUser.name} ({query.data.adminUser.email}) 계정으로 즉시 전환돼요.
      </div>
    </div>

    <div class={flex({ justifyContent: 'flex-end', gap: '10px' })}>
      <Button onclick={() => (impersonateModalOpen = false)} size="sm" type="button" variant="secondary">취소</Button>
      <Button
        onclick={async () => {
          impersonateModalOpen = false;
          await handleImpersonate();
        }}
        size="sm"
        type="button"
        variant="danger"
      >
        접속
      </Button>
    </div>
  </div>
</Modal>
