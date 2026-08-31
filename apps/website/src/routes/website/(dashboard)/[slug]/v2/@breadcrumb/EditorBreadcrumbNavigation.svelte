<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions, portal } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { entityIconMap } from '@typie/ui/constants';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { onDestroy, tick } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { scale } from 'svelte/transition';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import EntityIcon from '../../../@context-menu/EntityIcon.svelte';
  import { getPaneGroup } from '../../@pane/context.svelte';
  import { BreadcrumbDocumentDragController } from './breadcrumb-document-drag.svelte';
  import BreadcrumbEntityTree from './BreadcrumbEntityTree.svelte';
  import type { EditorContextBarSegmentState } from '$lib/editor-ffi/components/ui/editor-context-bar.svelte';
  import type { EntityIcon_entity$key } from '$mearie';
  import type { BreadcrumbContainer } from './BreadcrumbEntityTree.svelte';

  export type EditorBreadcrumbPathEntity = {
    id: string;
    name: string;
    entity$key: EntityIcon_entity$key;
  };

  type BreadcrumbNavigationSegment = {
    id: string;
    container: BreadcrumbContainer;
  };

  type Props = {
    ancestors: readonly EditorBreadcrumbPathEntity[];
    current: EditorBreadcrumbPathEntity & { slug: string };
    isOwner: boolean;
    onNavigate: (slug: string) => void;
    popupId: string;
    segment: EditorContextBarSegmentState;
    siteId: string;
  };

  let { ancestors, current, isOwner, onNavigate, popupId, segment, siteId }: Props = $props();

  const POPUP_HOLD_REASON = 'breadcrumb-popup';
  const pathEntities = $derived([...ancestors, current]);
  const segments = $derived(
    pathEntities.map((pathEntity, index): BreadcrumbNavigationSegment => ({
      id: pathEntity.id,
      container: index === 0 ? { kind: 'site', siteId } : { kind: 'entity', entityId: pathEntities[index - 1].id },
    })),
  );
  let activeSegmentId = $state<string | null>(null);
  const expandedFolderIds = new SvelteSet<string>();
  const activeSegment = $derived(segments.find((candidate) => candidate.id === activeSegmentId));
  const activeTriggerId = $derived(activeSegment ? triggerId(activeSegment.id) : '');
  let navigationElement = $state<HTMLElement>();
  let activeTrigger = $state<HTMLButtonElement>();
  const paneGroup = getPaneGroup();
  const documentDrag = new BreadcrumbDocumentDragController({ paneGroup, onDropSuccess: () => dismiss(false) });

  const { floating, setReference } = createFloatingActions({
    placement: 'bottom-start',
    offset: 6,
    onClickOutside: () => {
      if (!documentDrag.hasPointerSession) dismiss(false);
    },
  });

  function triggerId(entityId: string) {
    return `${popupId}-trigger-${entityId}`;
  }

  function dismiss(restoreFocus: boolean) {
    documentDrag.cancel();
    if (activeSegmentId === null) return;
    const trigger = activeTrigger;
    activeSegmentId = null;
    expandedFolderIds.clear();
    activeTrigger = undefined;
    setReference(null);
    segment.release(POPUP_HOLD_REASON);
    if (restoreFocus) void tick().then(() => trigger?.isConnected && trigger.focus());
  }

  function activate(event: MouseEvent, segmentId: string) {
    const trigger = event.currentTarget as HTMLButtonElement;
    if (activeSegmentId === segmentId) {
      dismiss(false);
      return;
    }

    const wasClosed = activeSegmentId === null;
    activeSegmentId = segmentId;
    expandedFolderIds.clear();
    activeTrigger = trigger;
    setReference(trigger);
    if (wasClosed) segment.hold(POPUP_HOLD_REASON);
  }

  function toggleFolder(entityId: string) {
    if (expandedFolderIds.has(entityId)) expandedFolderIds.delete(entityId);
    else expandedFolderIds.add(entityId);
  }

  function activateDocument(slug: string) {
    const currentDocument = slug === current.slug;
    dismiss(false);
    if (!currentDocument) onNavigate(slug);
  }

  function handlePopupFocusOut(event: FocusEvent) {
    const popup = event.currentTarget as HTMLElement;
    if (event.relatedTarget instanceof Node && (popup.contains(event.relatedTarget) || navigationElement?.contains(event.relatedTarget)))
      return;
    dismiss(false);
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (activeSegmentId === null || event.defaultPrevented) return;
    if (event.key !== 'Escape') return;
    event.preventDefault();
    if (!documentDrag.cancel({ suppressClick: true })) dismiss(true);
  }

  $effect(() => {
    if (!isOwner) dismiss(false);
  });

  onDestroy(() => {
    dismiss(false);
    documentDrag.destroy();
  });
</script>

<svelte:window oncontextmenu={(event) => documentDrag.contextMenu(event)} onkeydown={handleWindowKeydown} />

<nav
  bind:this={navigationElement}
  class={css({
    display: 'flex',
    alignItems: 'center',
    height: '32px',
    paddingLeft: '8px',
    paddingRight: '16px',
    fontSize: '12px',
    fontWeight: 'medium',
    color: 'text.subtle',
  })}
  aria-label="문서 경로"
>
  <ol class={flex({ alignItems: 'center', gap: '2px', listStyle: 'none', whiteSpace: 'nowrap' })}>
    {#each pathEntities as pathEntity, index (pathEntity.id)}
      {@const currentEntity = index === pathEntities.length - 1}
      <li aria-current={currentEntity ? 'page' : undefined}>
        {#if isOwner}
          <button
            id={triggerId(pathEntity.id)}
            class={flex({
              alignItems: 'center',
              gap: '4px',
              height: '24px',
              paddingX: '4px',
              borderRadius: '5px',
              cursor: 'pointer',
              transition: 'common',
              _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
              _expanded: { backgroundColor: 'interactive.hover', color: 'text.default' },
            })}
            aria-controls={popupId}
            aria-expanded={activeSegmentId === pathEntity.id}
            aria-haspopup="tree"
            onclick={(event) => activate(event, pathEntity.id)}
            type="button"
          >
            <EntityIcon entity$key={pathEntity.entity$key} fallback={currentEntity ? undefined : FolderIcon} size={14} />
            <span>{pathEntity.name}</span>
            {#if !currentEntity}
              <span class={css({ display: 'grid', placeItems: 'center', color: 'text.faint' })} aria-hidden="true">
                <Icon icon={ChevronRightIcon} size={14} />
              </span>
            {/if}
          </button>
        {:else}
          <span class={flex({ alignItems: 'center', gap: '4px' })}>
            <EntityIcon entity$key={pathEntity.entity$key} fallback={currentEntity ? undefined : FolderIcon} size={14} />
            <span>{pathEntity.name}</span>
          </span>
        {/if}
      </li>
      {#if !isOwner && !currentEntity}
        <li class={css({ display: 'grid', placeItems: 'center', color: 'text.faint' })} aria-hidden="true">
          <Icon icon={ChevronRightIcon} size={14} />
        </li>
      {/if}
    {/each}
  </ol>
</nav>

{#if isOwner && activeSegment}
  <div
    class={css({
      zIndex: 'menu',
      display: 'flex',
      flexDirection: 'column',
      gap: '2px',
      borderRadius: '8px',
      width: '240px',
      paddingX: '0',
      paddingY: '0',
      overflow: 'hidden',
      backgroundColor: 'surface.subtle',
      boxShadow: 'menu',
    })}
    onfocusout={handlePopupFocusOut}
    use:floating
    transition:scale={{ start: 0.95, duration: prefersReducedMotion.current ? 0 : 150 }}
  >
    <BreadcrumbEntityTree
      container={activeSegment.container}
      currentDocumentEntityId={current.id}
      {documentDrag}
      {expandedFolderIds}
      highlightedEntityId={activeSegment.id}
      labelledBy={activeTriggerId}
      onActivateDocument={activateDocument}
      onDismiss={() => dismiss(true)}
      onToggleFolder={toggleFolder}
      treeId={popupId}
    />
  </div>
{/if}

{#if documentDrag.ghost}
  <div
    style:left={`${documentDrag.ghost.x + 8}px`}
    style:top={`${documentDrag.ghost.y}px`}
    style:max-width={`${documentDrag.ghost.width}px`}
    class={flex({
      position: 'fixed',
      alignItems: 'center',
      gap: '2px',
      minWidth: '0',
      paddingX: '8px',
      paddingY: '4px',
      borderRadius: 'full',
      backgroundColor: 'accent.info.default',
      color: 'text.bright',
      fontSize: '14px',
      fontWeight: 'bold',
      pointerEvents: 'none',
      userSelect: 'none',
      zIndex: 'ghost',
    })}
    aria-hidden="true"
    role="presentation"
    use:portal
  >
    <Icon
      style={css.raw({ flexShrink: '0', color: 'text.bright' })}
      icon={entityIconMap.get(documentDrag.ghost.icon ?? '') ?? FileIcon}
      size={14}
    />
    <span class={css({ minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
      {documentDrag.ghost.name}
    </span>
  </div>
{/if}
