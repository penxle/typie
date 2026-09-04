<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { untrack } from 'svelte';
  import ArrowUpDownIcon from '~icons/lucide/arrow-up-down';
  import CheckIcon from '~icons/lucide/check';
  import { invalidateRecentDocuments } from '$lib/graphql/recent-documents';
  import { graphql } from '$mearie';
  import PinnedEntities from './PinnedEntities.svelte';
  import RecentDocuments from './RecentDocuments.svelte';
  import SidebarSectionHeader from './SidebarSectionHeader.svelte';
  import type { RecentDocumentSort } from '$lib/graphql/recent-documents';
  import type { DashboardLayout_QuickAccess_site$key } from '$mearie';
  import type { SidebarSectionTab } from './SidebarSectionHeader.svelte';

  type QuickAccessTab = 'RECENT' | 'PINNED';

  type Props = {
    site$key: DashboardLayout_QuickAccess_site$key;
    canScrollUp: boolean;
    headerHeight?: number;
  };

  let { site$key, canScrollUp, headerHeight = $bindable(0) }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_QuickAccess_site on Site {
        id
        ...DashboardLayout_RecentDocuments_site
        ...DashboardLayout_PinnedEntities_site
      }
    `),
    () => site$key,
  );

  const app = getAppContext();
  const open = $derived(app.preference.current.sidebarRecentDocumentsOpen);
  const tab = $derived(app.preference.current.sidebarQuickAccessTab);
  const sort = $derived(app.preference.current.sidebarRecentDocumentsSort);
  let collapsed = $state(!app.preference.current.sidebarRecentDocumentsOpen);
  let sectionElement = $state<HTMLElement>();
  let activeSiteId = untrack(() => site.data.id);

  $effect(() => {
    const siteId = site.data.id;

    untrack(() => {
      if (siteId === activeSiteId) return;

      activeSiteId = siteId;
      invalidateRecentDocuments(siteId);
    });
  });

  const tabs: readonly SidebarSectionTab[] = [
    { value: 'PINNED', label: '고정', dropTarget: 'pin' },
    { value: 'RECENT', label: '최근' },
  ];

  const sortOptions: { value: RecentDocumentSort; label: string }[] = [
    { value: 'VIEWED_AT', label: '최근 본 순서' },
    { value: 'UPDATED_AT', label: '최근 수정한 순서' },
  ];

  const toggleOpen = () => {
    app.preference.current.sidebarRecentDocumentsOpen = !open;
    if (!open && prefersReducedMotion.current) collapsed = true;
  };

  $effect(() => {
    if (open) collapsed = false;
  });

  const handleRevealTransitionEnd = (event: TransitionEvent) => {
    if (!open && event.target === event.currentTarget && event.propertyName === 'grid-template-rows') {
      collapsed = true;
    }
  };
</script>

<section bind:this={sectionElement} class={css({ flexShrink: '0', marginBottom: '4px' })}>
  <SidebarSectionHeader
    activeTab={tab}
    dividerVisible={canScrollUp}
    onSelectTab={(value) => (app.preference.current.sidebarQuickAccessTab = value as QuickAccessTab)}
    onToggle={toggleOpen}
    {open}
    {tabs}
    bind:height={headerHeight}
  >
    {#snippet actions()}
      {#if tab === 'RECENT'}
        <Menu
          style={css.raw({
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: '4px',
            size: '24px',
            color: 'text.faint',
            opacity: '50',
            transition: 'common',
            _hover: { color: 'text.subtle', opacity: '100' },
            _focusVisible: { opacity: '100' },
            _expanded: { color: 'text.subtle', backgroundColor: 'surface.muted', opacity: '100' },
          })}
          buttonAriaLabel="최근 문서 정렬"
          placement="bottom-start"
        >
          {#snippet button()}
            <Icon icon={ArrowUpDownIcon} size={14} />
          {/snippet}

          <div
            class={css({ paddingX: '10px', paddingY: '4px', fontSize: '12px', fontWeight: 'medium', color: 'text.disabled' })}
            role="presentation"
          >
            정렬 기준
          </div>

          {#each sortOptions as option (option.value)}
            <MenuItem
              aria-checked={sort === option.value}
              onclick={() => (app.preference.current.sidebarRecentDocumentsSort = option.value)}
              role="menuitemradio"
            >
              {option.label}
              {#if sort === option.value}
                <Icon style={css.raw({ marginLeft: 'auto', color: 'text.brand' })} icon={CheckIcon} size={14} />
              {/if}
            </MenuItem>
          {/each}
        </Menu>
      {/if}
    {/snippet}
  </SidebarSectionHeader>

  <div
    class={css({
      display: 'grid',
      gridTemplateRows: open ? '1fr' : '0fr',
      transition: '[grid-template-rows 160ms ease-out]',
      _motionReduce: { transition: '[none]' },
    })}
    aria-hidden={!open}
    inert={!open}
    ontransitionend={handleRevealTransitionEnd}
  >
    <div
      style:opacity={open ? '1' : '0'}
      class={css({
        minHeight: '0',
        overflow: 'hidden',
        transition: '[opacity 120ms ease-out]',
        _motionReduce: { transition: '[none]' },
      })}
    >
      {#if tab === 'RECENT'}
        <RecentDocuments {collapsed} {open} site$key={site.data} />
      {:else}
        <PinnedEntities {collapsed} {sectionElement} site$key={site.data} />
      {/if}
    </div>
  </div>
</section>
