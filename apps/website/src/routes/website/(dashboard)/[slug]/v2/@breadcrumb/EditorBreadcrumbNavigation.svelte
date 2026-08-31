<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions, portal } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { entityIconMap } from '@typie/ui/constants';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { onDestroy, tick } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { scale } from 'svelte/transition';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import HomeIcon from '~icons/lucide/house';
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

  export type EditorBreadcrumbTarget = { kind: 'home' } | { kind: 'entity'; slug: string };

  export type EditorBreadcrumbCurrent = { kind: 'home' } | ({ kind: 'entity' } & EditorBreadcrumbPathEntity & { slug: string });

  type BreadcrumbPathItem =
    Extract<EditorBreadcrumbCurrent, { kind: 'home' }> | ({ kind: 'entity' } & EditorBreadcrumbPathEntity & { slug?: string });

  type BreadcrumbNavigationSegment = {
    id: string;
    kind: BreadcrumbPathItem['kind'];
    container: BreadcrumbContainer;
  };

  type Props = {
    ancestors: readonly EditorBreadcrumbPathEntity[];
    current: EditorBreadcrumbCurrent;
    isOwner: boolean;
    onNavigate: (target: EditorBreadcrumbTarget) => void;
    popupId: string;
    segment?: EditorContextBarSegmentState;
    siteId: string | null;
  };

  let { ancestors, current, isOwner, onNavigate, popupId, segment, siteId }: Props = $props();

  const POPUP_HOLD_REASON = 'breadcrumb-popup';
  const HOME_SEGMENT_ID = 'home';

  function pathItemId(pathItem: BreadcrumbPathItem) {
    return pathItem.kind === 'home' ? HOME_SEGMENT_ID : pathItem.id;
  }

  function pathItemName(pathItem: BreadcrumbPathItem) {
    return pathItem.kind === 'home' ? '홈' : pathItem.name;
  }

  const pathItems = $derived<BreadcrumbPathItem[]>(
    current.kind === 'home' ? [current] : [...ancestors.map((entity) => ({ kind: 'entity' as const, ...entity })), current],
  );
  const interactive = $derived(isOwner && siteId !== null);
  const segments = $derived.by<BreadcrumbNavigationSegment[]>(() => {
    if (siteId === null) return [];

    return pathItems.map((pathItem, index): BreadcrumbNavigationSegment => {
      const previous = pathItems[index - 1];
      let container: BreadcrumbContainer;

      if (index === 0) {
        container = { kind: 'site', siteId };
      } else {
        if (previous?.kind !== 'entity') throw new Error('Home cannot be a breadcrumb ancestor');
        container = { kind: 'entity', entityId: previous.id };
      }

      return {
        id: pathItemId(pathItem),
        kind: pathItem.kind,
        container,
      };
    });
  });
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
    segment?.release(POPUP_HOLD_REASON);
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
    if (wasClosed) segment?.hold(POPUP_HOLD_REASON);
  }

  function toggleFolder(entityId: string) {
    if (expandedFolderIds.has(entityId)) expandedFolderIds.delete(entityId);
    else expandedFolderIds.add(entityId);
  }

  function activateDocument(slug: string) {
    const currentDocument = current.kind === 'entity' && slug === current.slug;
    dismiss(false);
    if (!currentDocument) onNavigate({ kind: 'entity', slug });
  }

  function activateHome() {
    dismiss(false);
    onNavigate({ kind: 'home' });
  }

  function handlePopupFocusOut(event: FocusEvent) {
    const popup = event.currentTarget as HTMLElement;
    if (event.relatedTarget instanceof Node && (popup.contains(event.relatedTarget) || navigationElement?.contains(event.relatedTarget)))
      return;
    dismiss(false);
  }

  $effect(() => {
    if (!interactive) dismiss(false);
  });

  $effect(() => {
    if (activeSegmentId === null) return;

    return pushEscapeHandler(() => {
      dismiss(true);
      return true;
    });
  });

  onDestroy(() => {
    dismiss(false);
    documentDrag.destroy();
  });
</script>

<svelte:window oncontextmenu={(event) => documentDrag.contextMenu(event)} />

<nav
  bind:this={navigationElement}
  class={css({
    display: 'flex',
    alignItems: 'center',
    height: '32px',
    paddingLeft: '4px',
    paddingRight: '4px',
    fontSize: '12px',
    fontWeight: 'medium',
    color: 'text.subtle',
  })}
  aria-label="문서 경로"
>
  <ol class={flex({ alignItems: 'center', gap: '2px', listStyle: 'none', whiteSpace: 'nowrap' })}>
    {#each pathItems as pathItem, index (pathItemId(pathItem))}
      {@const currentEntity = index === pathItems.length - 1}
      {@const itemId = pathItemId(pathItem)}
      <li aria-current={currentEntity ? 'page' : undefined}>
        {#if interactive}
          <button
            id={triggerId(itemId)}
            class={flex({
              alignItems: 'center',
              gap: '4px',
              height: '24px',
              paddingX: '4px',
              borderRadius: '5px',
              cursor: 'pointer',
              transition: 'common',
              _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
              _expanded: { backgroundColor: 'surface.muted', color: 'text.default' },
            })}
            aria-controls={popupId}
            aria-expanded={activeSegmentId === itemId}
            aria-haspopup="tree"
            onclick={(event) => activate(event, itemId)}
            type="button"
          >
            {#if pathItem.kind === 'home'}
              <Icon icon={HomeIcon} size={14} />
            {:else}
              <EntityIcon entity$key={pathItem.entity$key} fallback={currentEntity ? undefined : FolderIcon} size={14} />
            {/if}
            <span>{pathItemName(pathItem)}</span>
            {#if !currentEntity}
              <span class={css({ display: 'grid', placeItems: 'center', color: 'text.faint' })} aria-hidden="true">
                <Icon icon={ChevronRightIcon} size={14} />
              </span>
            {/if}
          </button>
        {:else}
          <span
            class={flex({
              alignItems: 'center',
              gap: '4px',
              height: pathItem.kind === 'home' ? '24px' : undefined,
              paddingX: pathItem.kind === 'home' ? '4px' : undefined,
              borderRadius: pathItem.kind === 'home' ? '5px' : undefined,
            })}
          >
            {#if pathItem.kind === 'home'}
              <Icon icon={HomeIcon} size={14} />
            {:else}
              <EntityIcon entity$key={pathItem.entity$key} fallback={currentEntity ? undefined : FolderIcon} size={14} />
            {/if}
            <span>{pathItemName(pathItem)}</span>
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

{#if interactive && activeSegment}
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
      currentDocumentEntityId={current.kind === 'entity' ? current.id : null}
      {documentDrag}
      {expandedFolderIds}
      highlightedEntityId={activeSegment.kind === 'entity' ? activeSegment.id : null}
      labelledBy={activeTriggerId}
      onActivateDocument={activateDocument}
      onActivateHome={activateHome}
      onToggleFolder={toggleFolder}
      showHomeItem={activeSegment.kind === 'entity' && activeSegment.container.kind === 'site'}
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
