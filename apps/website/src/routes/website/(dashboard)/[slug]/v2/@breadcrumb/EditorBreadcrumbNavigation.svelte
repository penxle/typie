<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { onDestroy, tick } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { scale } from 'svelte/transition';
  import { EntityRowDragController } from '../../../@tree/entity-row-drag.svelte';
  import EntityRowDragGhost from '../../../@tree/EntityRowDragGhost.svelte';
  import { getPaneGroup } from '../../@pane/context.svelte';
  import BreadcrumbEntityTree from './BreadcrumbEntityTree.svelte';
  import EditorBreadcrumbSegment from './EditorBreadcrumbSegment.svelte';
  import type { EntityIcon_entity$key } from '$mearie';
  import type { BreadcrumbContainer } from './BreadcrumbEntityTree.svelte';

  export type EditorBreadcrumbPathEntity = {
    id: string;
    name: string;
    entity$key: EntityIcon_entity$key;
  };

  export type EditorBreadcrumbTarget = { kind: 'home' } | { kind: 'entity'; slug: string };

  export type EditorBreadcrumbHoldHandle = {
    hold(reason: string): void;
    release(reason: string): void;
  };

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
    segment?: EditorBreadcrumbHoldHandle;
    siteId: string | null;
  };

  let { ancestors, current, isOwner, onNavigate, popupId, segment, siteId }: Props = $props();

  const POPUP_HOLD_REASON = 'breadcrumb-popup';
  const HOME_SEGMENT_ID = 'home';
  const interactiveSegmentClass = css({
    cursor: 'pointer',
    transition: 'common',
    _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
    _expanded: { backgroundColor: 'surface.active', color: 'text.default' },
  });

  function pathItemId(pathItem: BreadcrumbPathItem) {
    return pathItem.kind === 'home' ? HOME_SEGMENT_ID : pathItem.id;
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
  const documentDrag = new EntityRowDragController({ paneGroup, onDropSuccess: () => dismiss(false) });

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
    color: 'text.muted',
  })}
  aria-label="문서 경로"
>
  <ol class={flex({ alignItems: 'center', listStyle: 'none', whiteSpace: 'nowrap' })}>
    {#each pathItems as pathItem, index (pathItemId(pathItem))}
      {@const isCurrent = index === pathItems.length - 1}
      {@const itemId = pathItemId(pathItem)}
      {@const segmentClass = flex({
        alignItems: 'center',
        gap: '4px',
        height: '24px',
        paddingLeft: '4px',
        paddingRight: isCurrent ? '4px' : '0',
        borderRadius: '5px',
      })}
      <li aria-current={isCurrent ? 'page' : undefined}>
        {#if interactive}
          <button
            id={triggerId(itemId)}
            class={cx(segmentClass, interactiveSegmentClass)}
            aria-controls={popupId}
            aria-expanded={activeSegmentId === itemId}
            aria-haspopup="tree"
            onclick={(event) => activate(event, itemId)}
            type="button"
          >
            <EditorBreadcrumbSegment {isCurrent} item={pathItem} />
          </button>
        {:else}
          <span class={segmentClass}>
            <EditorBreadcrumbSegment {isCurrent} item={pathItem} />
          </span>
        {/if}
      </li>
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
      backgroundColor: 'surface.default',
      boxShadow: 'lg',
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
  <EntityRowDragGhost ghost={documentDrag.ghost} />
{/if}
