<script lang="ts">
  import { createFragment, createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { contextMenu, tooltip } from '@typie/ui/actions';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import FolderIcon from '~icons/lucide/folder';
  import { graphql } from '$mearie';
  import EntityIcon from '../@context-menu/EntityIcon.svelte';
  import FolderMenu from '../@context-menu/FolderMenu.svelte';
  import EntityGoalIndicator from '../@goal/EntityGoalIndicator.svelte';
  import { createEntityTreeRevealRequest, entityTreeRevealState } from './entity-reveal.svelte';
  import EntityMenu from './EntityMenu.svelte';
  import EntityName from './EntityName.svelte';
  import FolderTooltip from './FolderTooltip.svelte';
  import type { Action } from 'svelte/action';
  import type { DashboardLayout_PinnedFolder_folder$key } from '$mearie';
  import type { EntityRowDragController, EntityRowDragItem } from './entity-row-drag.svelte';

  type Props = {
    folder$key: DashboardLayout_PinnedFolder_folder$key;
    rowDrag?: EntityRowDragController;
  };

  let { folder$key, rowDrag }: Props = $props();

  const noopRowDrag: Action<HTMLElement, EntityRowDragItem | null> = () => ({});
  const dragRow = rowDrag?.drag ?? noopRowDrag;

  const folder = createFragment(
    graphql(`
      fragment DashboardLayout_PinnedFolder_folder on Folder {
        id
        name
        characterCount
        createdAt

        entity {
          id
          slug
          order
          depth
          visibility
          url
          icon
          iconColor
          pinnedOrder

          ...EntityIcon_entity

          goal {
            id
            targetCharacterCount
            dueAt
            createdAt
          }

          parent {
            id
          }

          ancestors {
            id
          }

          lastChild {
            id
            order
          }

          site {
            id
          }
        }
      }
    `),
    () => folder$key,
  );

  const app = getAppContext();
  const active = $derived(app.state.ancestors.includes(folder.data.entity.id));
  const isCut = $derived(app.state.clipboard?.mode === 'cut' && app.state.clipboard.entityIds.includes(folder.data.entity.id));

  let tooltipRequested = $state(false);

  const tooltipInfo = createQuery(
    graphql(`
      query DashboardLayout_PinnedFolder_Tooltip_Query($folderId: ID!) {
        folder(id: $folderId) {
          id
          characterCount
          folderCount
          documentCount
        }
      }
    `),
    () => ({ folderId: folder.data.id }),
    () => ({ skip: !tooltipRequested }),
  );

  const rowDragItem = $derived<EntityRowDragItem>({
    id: folder.data.entity.id,
    type: 'folder',
    slug: folder.data.entity.slug,
    name: folder.data.name,
    icon: folder.data.entity.icon ?? undefined,
  });

  const reveal = () => {
    const entity = folder.data.entity;
    app.preference.current.sidebarAllDocumentsOpen = true;
    entityTreeRevealState.set(
      createEntityTreeRevealRequest(entity.id, [...entity.ancestors.map((ancestor) => ancestor.id), entity.id], false),
    );
  };

  const handleClick = (event: MouseEvent) => {
    if (rowDrag?.consumeClick(event)) return;
    if (event.defaultPrevented) return;
    reveal();
  };

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.target !== event.currentTarget) return;
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    reveal();
  };
</script>

<div
  style:opacity={isCut ? 0.5 : 1}
  class={cx(
    'group',
    css(
      {
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        width: 'full',
        paddingX: '8px',
        paddingY: '6px',
        borderRadius: '6px',
        cursor: 'pointer',
        touchAction: 'none',
        transition: 'common',
        _supportHover: { backgroundColor: 'surface.muted' },
        '&:has([aria-pressed="true"])': { backgroundColor: 'surface.muted' },
        '&[data-context-menu-open="true"]': { backgroundColor: 'surface.muted' },
      },
      active && { backgroundColor: 'surface.muted' },
    ),
  )}
  data-icon={folder.data.entity.icon}
  data-icon-color={folder.data.entity.iconColor}
  data-id={folder.data.entity.id}
  data-name={folder.data.name}
  data-slug={folder.data.entity.slug}
  data-type="folder"
  onclick={handleClick}
  onkeydown={handleKeydown}
  onpointerenter={() => (tooltipRequested = true)}
  role="button"
  tabindex="0"
  use:contextMenu={{ content: contextMenuContent }}
  use:dragRow={rowDragItem}
  use:tooltip={{ message: tooltipContent, placement: 'right', delay: 1000 }}
>
  <EntityIcon entity$key={folder.data.entity} fallback={FolderIcon} size={14} />

  <EntityName name={folder.data.name} {active} />

  {#if folder.data.entity.goal}
    <EntityGoalIndicator
      current={folder.data.characterCount}
      dueAt={folder.data.entity.goal.dueAt}
      goalCreatedAt={folder.data.entity.goal.createdAt}
      onclick={() => {
        app.state.goalOpen = [folder.data.entity.id];
        mixpanel.track('open_goal_modal', { via: 'tree_glyph' });
      }}
      targetCharacterCount={folder.data.entity.goal.targetCharacterCount}
    />
  {/if}

  <EntityMenu label="폴더 메뉴">
    {@render contextMenuContent()}
  </EntityMenu>
</div>

{#snippet contextMenuContent()}
  <FolderMenu entity={folder.data.entity} folder={folder.data} via="pinned" />
{/snippet}

{#snippet tooltipContent()}
  <FolderTooltip
    characterCount={tooltipInfo.data?.folder.characterCount}
    createdAt={folder.data.createdAt}
    documentCount={tooltipInfo.data?.folder.documentCount}
    folderCount={tooltipInfo.data?.folder.folderCount}
    loading={tooltipInfo.loading}
    visibility={folder.data.entity.visibility}
  />
{/snippet}
