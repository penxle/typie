<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { contextMenu, tooltip } from '@typie/ui/actions';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import FileIcon from '~icons/lucide/file';
  import { goto } from '$app/navigation';
  import { navigating, page } from '$app/state';
  import { graphql } from '$mearie';
  import DocumentMenu from '../@context-menu/DocumentMenu.svelte';
  import EntityIcon from '../@context-menu/EntityIcon.svelte';
  import EntityGoalIndicator from '../@goal/EntityGoalIndicator.svelte';
  import EntitySelectionIndicator from './@selection/EntitySelectionIndicator.svelte';
  import MultiEntitiesMenu from './@selection/MultiEntitiesMenu.svelte';
  import DocumentTooltip from './DocumentTooltip.svelte';
  import { entityTreeRevealState, shouldConsumeDocumentRevealRequest } from './entity-reveal.svelte';
  import EntityMenu from './EntityMenu.svelte';
  import EntityName from './EntityName.svelte';
  import { getTreeContext } from './state.svelte';
  import type { Action } from 'svelte/action';
  import type { DashboardLayout_EntityTree_Document_document$key } from '$mearie';
  import type { EntityRowDragController, EntityRowDragItem } from './entity-row-drag.svelte';

  type Props = {
    document$key: DashboardLayout_EntityTree_Document_document$key;
    source?: 'tree' | 'recent' | 'pinned';
    rowDrag?: EntityRowDragController;
  };

  let { document$key, source = 'tree', rowDrag }: Props = $props();

  const noopRowDrag: Action<HTMLElement, EntityRowDragItem | null> = () => ({});
  const dragRow = rowDrag?.drag ?? noopRowDrag;

  const document = createFragment(
    graphql(`
      fragment DashboardLayout_EntityTree_Document_document on Document {
        id
        title
        documentType: type
        characterCount
        createdAt
        updatedAt

        entity {
          id
          depth
          order
          slug
          visibility
          availability
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
        }
      }
    `),
    () => document$key,
  );

  const app = getAppContext();
  const treeState = getTreeContext();
  const active = $derived(app.state.current === document.data.entity.slug);
  const selected = $derived(source === 'tree' && treeState.selectedEntityIds.has(document.data.entity.id));
  const isCut = $derived(app.state.clipboard?.mode === 'cut' && app.state.clipboard.entityIds.includes(document.data.entity.id));
  const rowDragItem = $derived<EntityRowDragItem | null>(
    source === 'tree'
      ? null
      : {
          id: document.data.entity.id,
          type: 'document',
          slug: document.data.entity.slug,
          name: document.data.title,
          icon: document.data.entity.icon ?? undefined,
        },
  );

  $effect(() => {
    if (source !== 'tree') return;

    const entityId = document.data.entity.id;
    const icon = document.data.entity.icon;
    const iconColor = document.data.entity.iconColor;
    untrack(() => {
      const entry = treeState.treeEntityMap.get(entityId);
      if (entry) {
        treeState.treeEntityMap.set(entityId, { ...entry, icon, iconColor });
      }
    });
  });

  let element = $state<HTMLAnchorElement>();

  const handleClick = (event: MouseEvent) => {
    if (rowDrag?.consumeClick(event)) return;

    if (
      source === 'tree' ||
      event.defaultPrevented ||
      event.button !== 0 ||
      event.metaKey ||
      event.ctrlKey ||
      event.shiftKey ||
      event.altKey
    ) {
      return;
    }

    event.preventDefault();
    void goto(`/${document.data.entity.slug}`, { state: { entityTreeReveal: 'preserve' } });
  };

  const navigationTargetSlug = $derived(navigating.to?.params?.slug ?? page.params.slug);

  $effect(() => {
    if (source === 'tree' && active && document.data.entity.slug === navigationTargetSlug && page.state.entityTreeReveal !== 'preserve') {
      element?.scrollIntoView({ behavior: 'instant', block: 'nearest' });
    }
  });

  $effect(() => {
    if (source !== 'tree') return;

    const request = entityTreeRevealState.current;
    if (!request || !shouldConsumeDocumentRevealRequest(request, document.data.entity.id, active, navigating.to === null)) {
      return;
    }

    entityTreeRevealState.consume(request);
  });
</script>

<a
  bind:this={element}
  style:opacity={isCut ? 0.5 : 1}
  class={cx(
    'group',
    css(
      {
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        paddingX: '8px',
        paddingY: '6px',
        borderRadius: '6px',
        transition: 'common',
        _supportHover: { backgroundColor: 'surface.muted' },
        '&:has([aria-pressed="true"])': { backgroundColor: 'surface.muted' },
        '&[data-context-menu-open="true"]': { backgroundColor: 'surface.muted' },
      },
      source === 'tree' &&
        document.data.entity.depth > 0 && {
          borderLeftWidth: '1px',
          borderLeftRadius: '0',
          marginLeft: '-1px',
          paddingLeft: '14px',
          _supportHover: { borderColor: 'border.strong' },
        },
      source !== 'tree' && { touchAction: 'none' },
      active && {
        backgroundColor: 'surface.muted',
      },
      selected && {
        backgroundColor: 'accent.info.subtle',
        _supportHover: { backgroundColor: 'accent.info.subtle' },
        '&:has([aria-pressed="true"])': { backgroundColor: 'accent.info.subtle' },
        '&[data-context-menu-open="true"]': {
          backgroundColor: 'accent.info.subtle',
        },
      },
    ),
  )}
  aria-selected={source === 'tree' ? 'false' : undefined}
  data-document-type={document.data.documentType}
  data-icon={document.data.entity.icon}
  data-icon-color={document.data.entity.iconColor}
  data-id={document.data.entity.id}
  data-name={document.data.title}
  data-order={document.data.entity.order}
  data-path-depth={document.data.entity.depth}
  data-slug={document.data.entity.slug}
  data-type="document"
  draggable="false"
  href="/{document.data.entity.slug}"
  onclick={handleClick}
  role={source === 'tree' ? 'treeitem' : undefined}
  use:contextMenu={{ content: contextMenuContent }}
  use:dragRow={rowDragItem}
  use:tooltip={{ message: tooltipContent, placement: 'right', delay: 1000 }}
>
  {#if source === 'tree'}
    <EntitySelectionIndicator entityId={document.data.entity.id} visibility={document.data.entity.visibility} />
  {/if}

  <EntityIcon entity$key={document.data.entity} fallback={FileIcon} size={14} />

  <EntityName name={document.data.title} {active} />

  {#if document.data.entity.goal}
    <EntityGoalIndicator
      current={document.data.characterCount}
      dueAt={document.data.entity.goal.dueAt}
      goalCreatedAt={document.data.entity.goal.createdAt}
      onclick={() => {
        app.state.goalOpen = [document.data.entity.id];
        mixpanel.track('open_goal_modal', { via: 'tree_glyph' });
      }}
      targetCharacterCount={document.data.entity.goal.targetCharacterCount}
    />
  {/if}

  <EntityMenu label="문서 메뉴">
    {@render contextMenuContent()}
  </EntityMenu>
</a>

{#snippet contextMenuContent()}
  {#if source === 'tree' && treeState.selectedEntityIds.size > 1 && treeState.selectedEntityIds.has(document.data.entity.id)}
    <MultiEntitiesMenu />
  {:else}
    <DocumentMenu document={document.data} entity={document.data.entity} structuralSource={source} via="tree" />
  {/if}
{/snippet}

{#snippet tooltipContent()}
  <DocumentTooltip
    availability={document.data.entity.availability}
    characterCount={document.data.characterCount}
    createdAt={document.data.createdAt}
    updatedAt={document.data.updatedAt}
    visibility={document.data.entity.visibility}
  />
{/snippet}
