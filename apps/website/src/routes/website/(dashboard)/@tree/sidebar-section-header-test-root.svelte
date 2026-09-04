<script lang="ts">
  import SidebarSectionHeader from './SidebarSectionHeader.svelte';

  let open = $state(true);
  let activeTab = $state('RECENT');
  let toggles = $state(0);

  const tabs = [
    { value: 'RECENT', label: '최근' },
    { value: 'PINNED', label: '고정', dropTarget: 'pin' },
  ];
</script>

<SidebarSectionHeader
  {activeTab}
  dividerVisible={false}
  onSelectTab={(value) => (activeTab = value)}
  onToggle={() => {
    open = !open;
    toggles += 1;
  }}
  {open}
  {tabs}
>
  {#snippet actions()}
    {#if activeTab === 'RECENT'}
      <span data-sort-menu>정렬</span>
    {/if}
  {/snippet}
</SidebarSectionHeader>

<output data-open>{open}</output>
<output data-active-tab>{activeTab}</output>
<output data-toggles>{toggles}</output>
