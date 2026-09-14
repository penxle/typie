<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Modal } from '@typie/ui/components';
  import LibraryIcon from '~icons/lucide/library';
  import OrbitIcon from '~icons/lucide/orbit';
  import SearchIcon from '~icons/lucide/search';
  import TextIcon from '~icons/lucide/text';
  import { graphql } from '$mearie';
  import SpaceSettingsAboutTab from './SpaceSettingsAboutTab.svelte';
  import SpaceSettingsCollectionsTab from './SpaceSettingsCollectionsTab.svelte';
  import SpaceSettingsGeneralTab from './SpaceSettingsGeneralTab.svelte';
  import SpaceSettingsVisibilityTab from './SpaceSettingsVisibilityTab.svelte';
  import type { Component } from 'svelte';

  type Props = {
    spaceId: string;
    open: boolean;
    onclose: () => void;
  };

  type Tab = {
    path: string;
    label: string;
    icon: Component;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    component: Component<any>;
  };

  let { spaceId, open, onclose }: Props = $props();

  const query = createQuery(
    graphql(`
      query UsersiteWildcard_SpaceSettingsModal_Query($spaceId: ID!) {
        space(spaceId: $spaceId) {
          id
          name

          ...UsersiteWildcard_SpaceSettingsGeneralTab_space
          ...UsersiteWildcard_SpaceSettingsAboutTab_space
          ...UsersiteWildcard_SpaceSettingsCollectionsTab_space
          ...UsersiteWildcard_SpaceSettingsVisibilityTab_space
        }
      }
    `),
    () => ({ spaceId }),
    () => ({ skip: !open }),
  );

  const space = $derived(query.data?.space);

  const tabs: Tab[] = [
    {
      path: 'general',
      label: '일반',
      icon: OrbitIcon,
      component: SpaceSettingsGeneralTab,
    },
    {
      path: 'about',
      label: '소개',
      icon: TextIcon,
      component: SpaceSettingsAboutTab,
    },
    {
      path: 'visibility',
      label: '노출',
      icon: SearchIcon,
      component: SpaceSettingsVisibilityTab,
    },
    {
      path: 'collections',
      label: '시리즈',
      icon: LibraryIcon,
      component: SpaceSettingsCollectionsTab,
    },
  ];

  let currentPath = $state('general');
  const currentTab = $derived(tabs.find((tab) => tab.path === currentPath) ?? tabs[0]);
</script>

<Modal style={css.raw({ maxWidth: '900px', height: 'full', maxHeight: '600px', padding: '0' })} loading={!space} {onclose} {open}>
  {#if space}
    <div class={flex({ height: 'full' })}>
      <div
        class={css({
          flex: 'none',
          paddingY: '24px',
          paddingX: '12px',
          width: '200px',
          borderRightWidth: '1px',
          borderColor: 'border.hairline',
          overflowY: 'auto',
        })}
      >
        <nav class={flex({ direction: 'column', gap: '16px' })}>
          <div>
            <div class={css({ paddingX: '10px', paddingY: '4px', marginBottom: '4px' })}>
              <h3
                class={css({
                  fontSize: '11px',
                  fontWeight: 'semibold',
                  color: 'text.muted',
                  textTransform: 'uppercase',
                  letterSpacing: '[0.05em]',
                  lineClamp: '1',
                  wordBreak: 'break-all',
                })}
              >
                {space.name}
              </h3>
            </div>

            <div class={flex({ direction: 'column', gap: '1px' })}>
              {#each tabs as { icon, path, label } (path)}
                <button
                  class={flex({
                    align: 'center',
                    gap: '8px',
                    borderRadius: '6px',
                    paddingX: '10px',
                    paddingY: '8px',
                    fontSize: '13px',
                    color: 'text.muted',
                    transition: 'common',
                    _hover: { backgroundColor: 'surface.hover' },
                    _selected: {
                      color: 'text.default',
                      fontWeight: 'medium',
                      backgroundColor: 'surface.active',
                    },
                  })}
                  aria-selected={currentTab.path === path}
                  onclick={() => {
                    currentPath = path;
                  }}
                  role="tab"
                  type="button"
                >
                  <Icon {icon} size={16} />
                  <span>{label}</span>
                </button>
              {/each}
            </div>
          </div>
        </nav>
      </div>

      <div class={css({ paddingX: '40px', paddingY: '32px', width: 'full', overflowY: 'auto' })}>
        <currentTab.component space$key={space} />
      </div>
    </div>
  {/if}
</Modal>
