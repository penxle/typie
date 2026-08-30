<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { RingSpinner, Scrollbar } from '@typie/ui/components';
  import { tick } from 'svelte';
  import { graphql } from '$mearie';
  import BreadcrumbEntityNode from './BreadcrumbEntityNode.svelte';
  import type { BreadcrumbDocumentDragController } from './breadcrumb-document-drag.svelte';

  export type BreadcrumbContainer = { kind: 'site'; siteId: string } | { kind: 'entity'; entityId: string };

  type Props = {
    container: BreadcrumbContainer;
    currentDocumentEntityId: string;
    expandedFolderIds: ReadonlySet<string>;
    highlightedEntityId: string;
    labelledBy: string;
    onActivateDocument: (slug: string) => void;
    documentDrag: BreadcrumbDocumentDragController;
    onDismiss: () => void;
    onToggleFolder: (entityId: string) => void;
    treeId: string;
  };

  let {
    container,
    currentDocumentEntityId,
    documentDrag,
    expandedFolderIds,
    highlightedEntityId,
    labelledBy,
    onActivateDocument,
    onDismiss,
    onToggleFolder,
    treeId,
  }: Props = $props();

  const siteEntities = createQuery(
    graphql(`
      query EditorBreadcrumbNavigation_SiteEntities_Query($siteId: ID!) {
        site(siteId: $siteId) {
          id

          entities {
            id
            ...EditorBreadcrumbNavigation_Entity_entity
          }
        }
      }
    `),
    () => ({ siteId: container.kind === 'site' ? container.siteId : '' }),
    () => ({ skip: container.kind !== 'site' }),
  );

  const entityChildren = createQuery(
    graphql(`
      query EditorBreadcrumbNavigation_EntityChildren_Query($entityId: ID!) {
        entity(entityId: $entityId) {
          id

          children {
            id
            ...EditorBreadcrumbNavigation_Entity_entity
          }
        }
      }
    `),
    () => ({ entityId: container.kind === 'entity' ? container.entityId : '' }),
    () => ({ skip: container.kind !== 'entity' }),
  );

  const activeQuery = $derived(container.kind === 'site' ? siteEntities : entityChildren);
  const entities = $derived(container.kind === 'site' ? siteEntities.data?.site.entities : entityChildren.data?.entity.children);
  let tree = $state<HTMLElement>();

  function visibleItems(): HTMLElement[] {
    return tree ? [...tree.querySelectorAll<HTMLElement>('[role="treeitem"]')] : [];
  }

  function setTabStop(item: HTMLElement) {
    const previous = tree?.querySelector<HTMLElement>('[role="treeitem"][tabindex="0"]');
    if (previous !== item) previous?.setAttribute('tabindex', '-1');
    item.tabIndex = 0;
  }

  function focusItem(item: HTMLElement | undefined) {
    if (!item) return;
    setTabStop(item);
    item.focus({ focusVisible: false });
  }

  function handleFocusIn(event: FocusEvent) {
    const item = event.target instanceof HTMLElement ? event.target.closest<HTMLElement>('[role="treeitem"]') : null;
    if (item && tree?.contains(item)) setTabStop(item);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      if (!documentDrag.cancel({ suppressClick: true })) onDismiss();
      return;
    }

    const current = event.target instanceof HTMLElement ? event.target.closest<HTMLElement>('[role="treeitem"]') : null;
    if (!current) return;
    const items = visibleItems();
    const index = items.indexOf(current);
    const kind = current.dataset.breadcrumbEntityKind;
    const entityId = current.dataset.breadcrumbEntityId;
    if (!entityId) return;

    if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault();
      focusItem(event.key === 'Home' ? items[0] : items.at(-1));
      return;
    }

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      focusItem(items[index + (event.key === 'ArrowDown' ? 1 : -1)]);
      return;
    }

    if (kind === 'folder' && event.key === 'ArrowRight') {
      event.preventDefault();
      if (current.getAttribute('aria-expanded') === 'false') {
        onToggleFolder(entityId);
      } else {
        focusItem(items.find((item) => item.dataset.parentEntityId === entityId));
      }
      return;
    }

    if (event.key === 'ArrowLeft') {
      if (kind === 'folder' && current.getAttribute('aria-expanded') === 'true') {
        event.preventDefault();
        onToggleFolder(entityId);
      } else if (current.dataset.parentEntityId) {
        event.preventDefault();
        focusItem(items.find((item) => item.dataset.breadcrumbEntityId === current.dataset.parentEntityId));
      }
      return;
    }
  }

  $effect(() => {
    void container;
    const treeElement = tree;
    const entityId = highlightedEntityId;
    void entities;
    if (!treeElement) return;

    void tick().then(() => {
      if (!treeElement.isConnected) return;
      const target = visibleItems().find((item) => item.dataset.breadcrumbEntityId === entityId) ?? visibleItems()[0];
      if (target) focusItem(target);
      else treeElement.focus();
    });
  });
</script>

<div class={css({ position: 'relative' })}>
  <div
    bind:this={tree}
    id={treeId}
    class={css({
      maxHeight: '[min(392px, var(--floating-available-height, 392px))]',
      padding: '4px',
      overflowY: 'auto',
      overscrollBehavior: 'contain',
      scrollbarWidth: 'none',
      touchAction: 'pan-y',
      userSelect: 'none',
    })}
    aria-labelledby={labelledBy}
    onfocusin={handleFocusIn}
    onkeydown={handleKeydown}
    role="tree"
    tabindex="-1"
  >
    {#if activeQuery.error}
      <div class={css({ paddingX: '8px', paddingY: '6px', fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>
        폴더 내용을 불러오지 못했어요
      </div>
    {:else if !entities}
      <div class={css({ paddingX: '8px', paddingY: '6px', color: 'text.disabled' })}>
        <RingSpinner style={css.raw({ size: '14px' })} />
      </div>
    {:else}
      {#each entities as entity (entity.id)}
        <BreadcrumbEntityNode
          {currentDocumentEntityId}
          {documentDrag}
          entity$key={entity}
          {expandedFolderIds}
          {highlightedEntityId}
          level={1}
          {onActivateDocument}
          {onToggleFolder}
        />
      {:else}
        <div class={css({ paddingX: '8px', paddingY: '6px', fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>
          폴더가 비어있어요
        </div>
      {/each}
    {/if}
  </div>

  <Scrollbar controls={treeId} label="문서 경로 트리 세로 스크롤" orientation="vertical" scrollContainer={tree} size="md" />
</div>
