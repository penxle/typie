<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { contextMenu } from '@typie/ui/actions';
  import { Icon, Menu } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { DocumentType } from '@/enums';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileIcon from '~icons/lucide/file';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import { graphql } from '$mearie';
  import DocumentMenu from '../@context-menu/DocumentMenu.svelte';
  import EntitySelectionIndicator from './@selection/EntitySelectionIndicator.svelte';
  import MultiEntitiesMenu from './@selection/MultiEntitiesMenu.svelte';
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
        }
      }
    `),
    () => document$key,
  );

  const app = getAppContext();
  const treeState = getTreeContext();
  const active = $derived(app.state.current === document.data.entity.slug);
  const selected = $derived(treeState.selectedEntityIds.has(document.data.entity.id));

  let element = $state<HTMLAnchorElement>();

  $effect(() => {
    if (active) {
      element?.scrollIntoView({ behavior: 'instant', block: 'nearest' });
    }
  });
</script>

<a
  bind:this={element}
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
        backgroundColor: 'accent.brand.subtle',
        _supportHover: { backgroundColor: 'accent.brand.subtle' },
        '&:has([aria-pressed="true"])': { backgroundColor: 'accent.brand.subtle' },
        '&[data-context-menu-open="true"]': { backgroundColor: 'accent.brand.subtle' },
      },
    ),
  )}
  aria-selected="false"
  data-id={document.data.entity.id}
  data-order={document.data.entity.order}
  data-path-depth={document.data.entity.depth}
  data-slug={document.data.entity.slug}
  data-type="document"
  draggable="false"
  href="/{document.data.entity.slug}"
  role="treeitem"
  use:contextMenu={{ content: contextMenuContent }}
>
  <EntitySelectionIndicator entityId={document.data.entity.id} visibility={document.data.entity.visibility} />

  <Icon
    style={css.raw({ color: 'text.faint' })}
    icon={document.data.documentType === DocumentType.TEMPLATE ? LayoutTemplateIcon : FileIcon}
    size={14}
  />

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
