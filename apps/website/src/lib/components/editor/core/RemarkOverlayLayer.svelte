<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { SvelteMap } from 'svelte/reactivity';
  import { fade } from 'svelte/transition';
  import { PAGE_GAP } from '$lib/editor/constants';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import RemarkPopover from './RemarkPopover.svelte';
  import type { RemarkOverlay } from '$lib/editor/slate';

  const REMARK_GAP = 16;

  const { editor } = getEditorContext();

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

  const remarkGroupsByPage = $derived.by(() => {
    const grouped = new SvelteMap<number, RemarkGroup[]>();
    for (const group of remarkGroups) {
      const current = grouped.get(group.pageIdx);
      if (current) {
        current.push(group);
      } else {
        grouped.set(group.pageIdx, [group]);
      }
    }
    return grouped;
  });

  $effect(() => {
    if (editor.remarkFocus) {
      openGroupNodeId = editor.remarkFocus.nodeId;
    }
  });

  const displayZoom = $derived(editor.layout?.layoutMode.type === 'paginated' ? editor.displayZoom : 1);
  const pageGap = $derived(editor.layout?.layoutMode.type === 'paginated' ? PAGE_GAP * displayZoom : 0);
  const currentBlockHasRemarks = $derived(editor.currentBlock ? remarkGroups.some((g) => g.nodeId === editor.currentBlock?.nodeId) : false);
  const showCurrentBlockPopover = $derived(
    !editor.readOnly &&
      (editor.isFocused || openGroupNodeId === editor.currentBlock?.nodeId) &&
      editor.currentBlock &&
      !currentBlockHasRemarks,
  );
</script>

{#if !editor.containerResizing}
  <div
    class={css({
      position: 'absolute',
      inset: '0',
      pointerEvents: 'none',
      zIndex: '2',
    })}
  >
    <div
      style:gap={`${pageGap}px`}
      class={css({
        position: 'relative',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        width: 'full',
        height: 'full',
      })}
    >
      {#each editor.layout?.pages ?? [] as page, pageIdx (`page-${pageIdx}`)}
        {@const groups = remarkGroupsByPage.get(pageIdx) ?? []}
        <div
          style:width={`${page.width * displayZoom}px`}
          style:height={`${page.height * displayZoom}px`}
          class={css({
            position: 'relative',
            pointerEvents: 'none',
          })}
        >
          {#each groups as group (group.nodeId)}
            <div
              style:position="absolute"
              style:left="{page.width * displayZoom + REMARK_GAP}px"
              style:top="{group.boundsY * displayZoom}px"
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
          {/each}

          {#if showCurrentBlockPopover && editor.currentBlock?.pageIdx === pageIdx}
            {@const block = editor.currentBlock}
            {#key block.nodeId}
              <div
                style:position="absolute"
                style:left="{page.width * displayZoom + REMARK_GAP}px"
                style:top="{block.bounds.y * displayZoom}px"
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
        </div>
      {/each}
    </div>
  </div>
{/if}
