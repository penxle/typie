<script lang="ts">
  import { createFragment, createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import { graphql } from '$mearie';
  import EntityIcon from '../../../@context-menu/EntityIcon.svelte';
  import EntityName from '../../../@tree/EntityName.svelte';
  import BreadcrumbEntityNode from './BreadcrumbEntityNode.svelte';
  import type { EditorBreadcrumbNavigation_Entity_entity$key } from '$mearie';
  import type { EntityRowDragController } from '../../../@tree/entity-row-drag.svelte';

  const entityTreeRowStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    paddingX: '8px',
    borderRadius: '6px',
    transition: 'common',
    _supportHover: { backgroundColor: 'surface.hover' },
  });

  const nestedEntityTreeRowStyle = css.raw({
    borderLeftWidth: '1px',
    borderLeftRadius: '0',
    marginLeft: '-1px',
    paddingLeft: '14px',
    _supportHover: { borderColor: 'border.emphasis' },
  });

  type Props = {
    currentDocumentEntityId: string | null;
    entity$key: EditorBreadcrumbNavigation_Entity_entity$key;
    expandedFolderIds: ReadonlySet<string>;
    highlightedEntityId: string | null;
    level: number;
    nested?: boolean;
    parentEntityId?: string;
    onActivateDocument: (slug: string) => void;
    documentDrag: EntityRowDragController;
    onToggleFolder: (entityId: string) => void;
  };

  let {
    currentDocumentEntityId,
    entity$key,
    expandedFolderIds,
    highlightedEntityId,
    level,
    nested = false,
    parentEntityId,
    onActivateDocument,
    documentDrag,
    onToggleFolder,
  }: Props = $props();

  const entity = createFragment(
    graphql(`
      fragment EditorBreadcrumbNavigation_Entity_entity on Entity {
        id
        slug
        icon
        iconColor
        ...EntityIcon_entity

        node {
          __typename

          ... on Folder {
            id
            name
          }

          ... on Document {
            id
            title
          }

          ... on Divider {
            id
          }
        }
      }
    `),
    () => entity$key,
  );

  const folder = $derived(entity.data.node.__typename === 'Folder');
  const document = $derived(entity.data.node.__typename === 'Document');
  const currentDocument = $derived(document && entity.data.id === currentDocumentEntityId);
  const highlighted = $derived(entity.data.id === highlightedEntityId);
  const expanded = $derived(folder && expandedFolderIds.has(entity.data.id));
  const dragDocument = documentDrag.drag;
  const dragItem = $derived(
    entity.data.node.__typename === 'Document'
      ? {
          id: entity.data.id,
          type: 'document' as const,
          slug: entity.data.slug,
          name: entity.data.node.title,
          icon: entity.data.icon ?? undefined,
        }
      : null,
  );

  const children = createQuery(
    graphql(`
      query EditorBreadcrumbNavigation_FolderChildren_Query($entityId: ID!) {
        entity(entityId: $entityId) {
          id

          children {
            id
            ...EditorBreadcrumbNavigation_Entity_entity
          }
        }
      }
    `),
    () => ({ entityId: entity.data.id }),
    () => ({ skip: !expanded }),
  );

  let row = $state<HTMLElement>();

  function targetsCurrentTreeItem(event: MouseEvent | KeyboardEvent) {
    return (
      event.composedPath().find((target) => target instanceof HTMLElement && target.getAttribute('role') === 'treeitem') ===
      event.currentTarget
    );
  }

  function activate() {
    row?.focus();
    if (folder) onToggleFolder(entity.data.id);
    else if (document) onActivateDocument(entity.data.slug);
  }

  function handleClick(event: MouseEvent) {
    if (!targetsCurrentTreeItem(event)) return;
    if (documentDrag.consumeClick(event)) return;
    activate();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!targetsCurrentTreeItem(event)) return;

    if ((folder && (event.key === 'Enter' || event.key === ' ')) || (document && event.key === 'Enter')) {
      event.preventDefault();
      event.stopPropagation();
      activate();
    } else if (document && event.key === ' ') {
      event.preventDefault();
      event.stopPropagation();
    }
  }
</script>

{#if entity.data.node.__typename === 'Divider'}
  <div
    class={css(
      entityTreeRowStyle,
      {
        minHeight: '32px',
        paddingY: '6px',
      },
      nested && nestedEntityTreeRowStyle,
    )}
    aria-hidden="true"
    data-breadcrumb-entity-id={entity.data.id}
    role="presentation"
  >
    <div class={css({ width: 'full', height: '1px', backgroundColor: 'border.default' })}></div>
  </div>
{:else}
  <div
    bind:this={row}
    class={css(
      {
        '&:focus > [data-breadcrumb-tree-row]': { backgroundColor: 'surface.hover' },
      },
      document && { touchAction: 'none' },
    )}
    aria-current={currentDocument ? 'page' : undefined}
    aria-expanded={folder ? expanded : undefined}
    aria-level={level}
    aria-selected={entity.data.id === highlightedEntityId}
    data-breadcrumb-entity-id={entity.data.id}
    data-breadcrumb-entity-kind={folder ? 'folder' : 'document'}
    data-breadcrumb-tree-item-key={entity.data.id}
    data-parent-entity-id={parentEntityId}
    onclick={handleClick}
    onkeydown={handleKeydown}
    role="treeitem"
    tabindex={highlighted ? 0 : -1}
    use:dragDocument={dragItem}
  >
    <div
      class={cx(
        'group',
        css(
          entityTreeRowStyle,
          {
            width: 'full',
            minWidth: '0',
            paddingY: '6px',
            color: 'text.muted',
            cursor: 'pointer',
          },
          nested && nestedEntityTreeRowStyle,
        ),
        highlighted && css({ backgroundColor: 'surface.hover' }),
        !highlighted && currentDocument && css({ backgroundColor: 'surface.active' }),
      )}
      data-breadcrumb-tree-row
    >
      {#if folder}
        <Icon style={css.raw({ flexShrink: '0', color: 'text.muted' })} icon={expanded ? ChevronDownIcon : ChevronRightIcon} size={14} />
        <EntityIcon entity$key={entity.data} fallback={FolderIcon} size={14} />
        <EntityName name={entity.data.node.__typename === 'Folder' ? entity.data.node.name : ''} active={currentDocument} />
      {:else}
        <EntityIcon entity$key={entity.data} fallback={FileIcon} size={14} />
        <EntityName name={entity.data.node.__typename === 'Document' ? entity.data.node.title : ''} active={currentDocument} />
      {/if}
    </div>

    {#if folder && expanded}
      <div class={flex({ flexDirection: 'column', borderLeftWidth: '1px', marginLeft: '24px' })} role="group">
        {#if children.error}
          <div
            class={css({
              paddingLeft: '14px',
              paddingRight: '8px',
              paddingY: '6px',
              fontSize: '14px',
              fontWeight: 'medium',
              color: 'text.hint',
            })}
          >
            폴더 내용을 불러오지 못했어요
          </div>
        {:else if !children.data}
          <div class={css({ paddingLeft: '14px', paddingRight: '8px', paddingY: '6px', color: 'text.muted' })}>
            <RingSpinner style={css.raw({ size: '14px' })} />
          </div>
        {:else}
          {#each children.data.entity.children as child (child.id)}
            <BreadcrumbEntityNode
              {currentDocumentEntityId}
              {documentDrag}
              entity$key={child}
              {expandedFolderIds}
              {highlightedEntityId}
              level={level + 1}
              nested
              {onActivateDocument}
              {onToggleFolder}
              parentEntityId={entity.data.id}
            />
          {:else}
            <div
              class={css({
                paddingLeft: '14px',
                paddingRight: '8px',
                paddingY: '6px',
                fontSize: '14px',
                fontWeight: 'medium',
                color: 'text.hint',
              })}
            >
              폴더가 비어있어요
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
{/if}
