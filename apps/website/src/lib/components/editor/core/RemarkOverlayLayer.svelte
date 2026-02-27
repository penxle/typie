<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tick } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { fade } from 'svelte/transition';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import RemarkPopover from './RemarkPopover.svelte';
  import type { RemarkOverlay } from '$lib/editor/slate';

  const REMARK_GAP = 16;

  const { editor } = getEditorContext();

  let layoutRefreshVersion = $state(0);
  let openGroupNodeId = $state<string | null>(null);

  function toggleGroup(nodeId: string) {
    openGroupNodeId = openGroupNodeId === nodeId ? null : nodeId;
  }

  let prevBlockNodeId: string | undefined;
  $effect(() => {
    const currentNodeId = editor.currentBlock?.nodeId;
    if (prevBlockNodeId !== undefined && prevBlockNodeId !== currentNodeId) {
      openGroupNodeId = null;
    }
    prevBlockNodeId = currentNodeId;
  });

  $effect(() => {
    void editor.layout?.layoutMode;

    let disposed = false;

    void (async () => {
      await tick();
      if (disposed) return;
      layoutRefreshVersion += 1;
    })();

    return () => {
      disposed = true;
    };
  });

  function pageOffset(pageIdx: number): { left: number; top: number } | null {
    const pageEl = editor.pageContainerEls[pageIdx];
    const containerEl = editor.extensionArea.containerEl;
    if (!pageEl || !containerEl) {
      return null;
    }
    const pageRect = pageEl.getBoundingClientRect();
    const containerRect = containerEl.getBoundingClientRect();
    return {
      left: pageRect.left - containerRect.left,
      top: pageRect.top - containerRect.top,
    };
  }

  type RemarkGroup = {
    nodeId: string;
    pageIdx: number;
    boundsY: number;
    remarks: RemarkOverlay[];
  };

  const remarkGroups = $derived.by(() => {
    const groups = new SvelteMap<string, RemarkGroup>();
    for (const overlay of editor.remarkOverlays) {
      const existing = groups.get(overlay.nodeId);
      if (existing) {
        existing.remarks.push(overlay);
      } else {
        groups.set(overlay.nodeId, {
          nodeId: overlay.nodeId,
          pageIdx: overlay.pageIdx,
          boundsY: overlay.bounds.y,
          remarks: [overlay],
        });
      }
    }
    for (const group of groups.values()) {
      group.remarks.sort((a, b) => a.createdAt - b.createdAt);
    }
    return [...groups.values()];
  });

  $effect(() => {
    if (editor.remarkFocus) {
      openGroupNodeId = editor.remarkFocus.nodeId;
    }
  });

  const currentBlockHasRemarks = $derived(editor.currentBlock ? remarkGroups.some((g) => g.nodeId === editor.currentBlock?.nodeId) : false);
</script>

<div
  class={css({
    position: 'absolute',
    inset: '0',
    pointerEvents: 'none',
    zIndex: '2',
  })}
>
  {#key layoutRefreshVersion}
    {#each remarkGroups as group (group.nodeId)}
      {@const page = editor.layout?.pages[group.pageIdx]}
      {@const offset = pageOffset(group.pageIdx)}
      {#if page && offset}
        <div
          style:position="absolute"
          style:left="{offset.left + page.width + REMARK_GAP}px"
          style:top="{offset.top + group.boundsY}px"
          style:pointer-events="auto"
          data-external-element
        >
          <RemarkPopover
            {editor}
            nodeId={group.nodeId}
            onToggle={() => toggleGroup(group.nodeId)}
            open={openGroupNodeId === group.nodeId}
            remarks={group.remarks}
          />
        </div>
      {/if}
    {/each}

    {#if !editor.readOnly && editor.currentBlock && !currentBlockHasRemarks}
      {@const block = editor.currentBlock}
      {@const page = editor.layout?.pages[block.pageIdx]}
      {@const offset = pageOffset(block.pageIdx)}
      {#if page && offset}
        {#key block.nodeId}
          <div
            style:position="absolute"
            style:left="{offset.left + page.width + REMARK_GAP}px"
            style:top="{offset.top + block.bounds.y}px"
            style:pointer-events="auto"
            data-external-element
            transition:fade|global={{ duration: 100 }}
          >
            <RemarkPopover
              {editor}
              nodeId={block.nodeId}
              onToggle={() => toggleGroup(block.nodeId)}
              open={openGroupNodeId === block.nodeId}
              remarks={[]}
            />
          </div>
        {/key}
      {/if}
    {/if}
  {/key}
</div>
