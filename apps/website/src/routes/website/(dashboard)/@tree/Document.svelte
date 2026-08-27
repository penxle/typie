<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { contextMenu, tooltip } from '@typie/ui/actions';
  import { Icon, Menu } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileIcon from '~icons/lucide/file';
  import { navigating } from '$app/state';
  import { graphql } from '$mearie';
  import DocumentMenu from '../@context-menu/DocumentMenu.svelte';
  import EntityIcon from '../@context-menu/EntityIcon.svelte';
  import EntityGoalIndicator from '../@goal/EntityGoalIndicator.svelte';
  import EntitySelectionIndicator from './@selection/EntitySelectionIndicator.svelte';
  import MultiEntitiesMenu from './@selection/MultiEntitiesMenu.svelte';
  import DocumentTooltip from './DocumentTooltip.svelte';
  import { entityTreeRevealState, shouldConsumeDocumentRevealRequest } from './entity-reveal.svelte';
  import { getTreeContext } from './state.svelte';
  import type { DashboardLayout_EntityTree_Document_document$key } from '$mearie';

  type Props = {
    document$key: DashboardLayout_EntityTree_Document_document$key;
  };

  let { document$key }: Props = $props();

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
  const selected = $derived(treeState.selectedEntityIds.has(document.data.entity.id));
  const isCut = $derived(app.state.clipboard?.mode === 'cut' && app.state.clipboard.entityIds.includes(document.data.entity.id));

  $effect(() => {
    const entityId = document.data.entity.id;
    const icon = document.data.entity.icon;
    const iconColor = document.data.entity.iconColor;
    untrack(() => {
      const entry = treeState.entityMap.get(entityId);
      if (entry) {
        treeState.entityMap.set(entityId, { ...entry, icon, iconColor });
      }
    });
  });

  let element = $state<HTMLAnchorElement>();

  $effect(() => {
    if (active) {
      element?.scrollIntoView({ behavior: 'instant', block: 'nearest' });
    }
  });

  $effect(() => {
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
      document.data.entity.depth > 0 && {
        borderLeftWidth: '1px',
        borderLeftRadius: '0',
        marginLeft: '-1px',
        paddingLeft: '14px',
        _supportHover: { borderColor: 'border.strong' },
      },
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
  aria-selected="false"
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
  role="treeitem"
  use:contextMenu={{ content: contextMenuContent }}
  use:tooltip={{ message: tooltipContent, placement: 'right', delay: 1000 }}
>
  <EntitySelectionIndicator entityId={document.data.entity.id} visibility={document.data.entity.visibility} />

  <EntityIcon entity$key={document.data.entity} fallback={FileIcon} size={14} />

  <span
    class={css(
      {
        flexGrow: '1',
        fontSize: '14px',
        fontWeight: 'medium',
        color: 'text.muted',
        wordBreak: 'break-all',
        lineClamp: '1',
      },
      active && { fontWeight: 'bold', color: 'text.default' },
    )}
  >
    {document.data.title}
  </span>

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

  <Menu placement="bottom-start">
    {#snippet button({ open })}
      <div
        class={center({
          borderRadius: '4px',
          size: '16px',
          color: 'text.disabled',
          opacity: '0',
          transition: 'common',
          _hover: { backgroundColor: 'interactive.hover' },
          _groupHover: { opacity: '100' },
          _pressed: { backgroundColor: 'interactive.hover', opacity: '100' },
        })}
        aria-pressed={open}
      >
        <Icon icon={EllipsisIcon} size={14} />
      </div>
    {/snippet}

    {@render contextMenuContent()}
  </Menu>
</a>

{#snippet contextMenuContent()}
  {#if treeState.selectedEntityIds.size > 1 && treeState.selectedEntityIds.has(document.data.entity.id)}
    <MultiEntitiesMenu />
  {:else}
    <DocumentMenu document={document.data} entity={document.data.entity} via="tree" />
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
