<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import MessageSquareTextIcon from '~icons/lucide/message-square-text';
  import DocumentPanelRemarkItem from './DocumentPanelRemarkItem.svelte';
  import type { Editor } from '$lib/editor/editor.svelte';
  import type { RemarkOverlay } from '$lib/editor/slate';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  const sortedRemarks = $derived(
    editor.remarkOverlays.toSorted((a, b) => {
      if (a.pageIdx !== b.pageIdx) return a.pageIdx - b.pageIdx;
      if (a.bounds.y !== b.bounds.y) return a.bounds.y - b.bounds.y;
      return a.createdAt - b.createdAt;
    }),
  );

  function focusRemark(remark: RemarkOverlay) {
    const pageEl = editor.pageContainerEls[remark.pageIdx];
    const scroller = editor.scrollContainerEl;
    if (pageEl && scroller) {
      const pageRect = pageEl.getBoundingClientRect();
      const scrollerRect = scroller.getBoundingClientRect();
      const targetY = pageRect.top + remark.bounds.y - scrollerRect.top + scroller.scrollTop;
      scroller.scrollTo({ top: Math.max(0, targetY - scroller.clientHeight / 3), behavior: 'smooth' });
    }
    editor.remarkFocus = { nodeId: remark.nodeId, remarkId: remark.remarkId };
  }
</script>

<div class={flex({ flexDirection: 'column', flexGrow: '1', height: 'full', overflow: 'hidden' })}>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      height: '41px',
      paddingX: '20px',
      flexShrink: '0',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    <div class={flex({ alignItems: 'center', gap: '6px', fontWeight: 'semibold' })}>
      <div class={css({ fontSize: '13px', color: 'text.subtle' })}>코멘트</div>
      {#if sortedRemarks.length > 0}
        <div
          class={css({
            fontSize: '11px',
            color: 'text.default',
            backgroundColor: 'surface.muted',
            paddingX: '6px',
            paddingY: '2px',
            borderRadius: '4px',
          })}
        >
          {sortedRemarks.length}
        </div>
      {/if}
    </div>
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      flexGrow: '1',
      overflowY: 'auto',
    })}
  >
    {#if sortedRemarks.length === 0}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '20px',
          paddingY: '60px',
        })}
      >
        <div
          class={center({
            size: '64px',
            borderRadius: '16px',
            backgroundColor: 'surface.muted',
            color: 'text.faint',
          })}
        >
          <Icon icon={MessageSquareTextIcon} size={28} />
        </div>

        <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>아직 코멘트가 없어요</p>
      </div>
    {:else}
      {#each sortedRemarks as remark (remark.remarkId)}
        <DocumentPanelRemarkItem onclick={() => focusRemark(remark)} {remark} />
      {/each}
    {/if}
  </div>
</div>
