<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { SvelteMap } from 'svelte/reactivity';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import MessageSquareTextIcon from '~icons/lucide/message-square-text';
  import DocumentPanelRemarkItem from './DocumentPanelRemarkItem.svelte';
  import type { Editor } from '$lib/editor/editor.svelte';
  import type { RemarkOverlay } from '$lib/editor/slate';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();
  let collapsedGroupsByNodeId = $state<Record<string, boolean>>({});

  const sortedRemarks = $derived(
    editor.remarkOverlays.toSorted((a, b) => {
      if (a.pageIdx !== b.pageIdx) return a.pageIdx - b.pageIdx;
      if (a.bounds.y !== b.bounds.y) return a.bounds.y - b.bounds.y;
      return a.createdAt - b.createdAt;
    }),
  );

  type RemarkGroup = {
    nodeId: string;
    remarks: RemarkOverlay[];
  };

  const REMARK_NODE_LABELS: Record<string, string> = {
    image: '이미지',
    file: '파일',
    embed: '임베드',
    archived: '보관된 블록',
    horizontal_rule: '구분선',
    blockquote: '인용구',
    callout: '강조',
    bullet_list: '순서 없는 목록',
    ordered_list: '순서 있는 목록',
    fold: '접기',
    table: '표',
  };

  const remarkGroups = $derived.by(() => {
    const groups = new SvelteMap<string, RemarkGroup>();
    const orderedGroups: RemarkGroup[] = [];

    for (const remark of sortedRemarks) {
      const existing = groups.get(remark.nodeId);
      if (existing) {
        existing.remarks.push(remark);
        continue;
      }

      const group: RemarkGroup = {
        nodeId: remark.nodeId,
        remarks: [remark],
      };
      groups.set(remark.nodeId, group);
      orderedGroups.push(group);
    }

    return orderedGroups;
  });

  const allGroupsCollapsed = $derived(
    remarkGroups.length > 0 && remarkGroups.every((group) => collapsedGroupsByNodeId[group.nodeId] === true),
  );

  function getGroupTitle(group: RemarkGroup): string {
    const primary = group.remarks[0];
    if (!primary) return '';

    if (primary.isTextblock) {
      const normalized = primary.nodeText.replaceAll(/\s+/g, ' ').trim();
      return normalized.length > 0 ? normalized : '빈 텍스트';
    }

    return REMARK_NODE_LABELS[primary.nodeType] ?? '';
  }

  function scrollToRemark(remark: RemarkOverlay) {
    const pageEl = editor.pageContainerEls[remark.pageIdx];
    const scroller = editor.scrollContainerEl;
    if (pageEl && scroller) {
      const pageRect = pageEl.getBoundingClientRect();
      const scrollerRect = scroller.getBoundingClientRect();
      const targetY = pageRect.top + remark.bounds.y - scrollerRect.top + scroller.scrollTop;
      scroller.scrollTo({ top: Math.max(0, targetY - scroller.clientHeight / 3), behavior: 'smooth' });
    }
  }

  function focusRemark(remark: RemarkOverlay, source: 'panel-item' | 'panel-group' = 'panel-item') {
    scrollToRemark(remark);
    editor.remarkFocus = { nodeId: remark.nodeId, remarkId: remark.remarkId, source };
  }

  function focusRemarkGroup(group: RemarkGroup) {
    const primary = group.remarks[0];
    if (!primary) return;
    scrollToRemark(primary);
    editor.remarkFocus = { nodeId: primary.nodeId, source: 'panel-group' };
  }

  function toggleGroup(nodeId: string) {
    collapsedGroupsByNodeId = {
      ...collapsedGroupsByNodeId,
      [nodeId]: collapsedGroupsByNodeId[nodeId] !== true,
    };
  }

  function toggleAllGroups() {
    const nextCollapsed = !allGroupsCollapsed;
    const next = { ...collapsedGroupsByNodeId };
    for (const group of remarkGroups) {
      next[group.nodeId] = nextCollapsed;
    }
    collapsedGroupsByNodeId = next;
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

    {#if remarkGroups.length >= 2}
      <button
        class={css({
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.faint',
          transition: 'common',
          _hover: { color: 'text.subtle' },
        })}
        onclick={toggleAllGroups}
        type="button"
      >
        {allGroupsCollapsed ? '모두 열기' : '모두 닫기'}
      </button>
    {/if}
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
      {#each remarkGroups as group (group.nodeId)}
        {@const collapsed = collapsedGroupsByNodeId[group.nodeId] === true}
        <div class={flex({ flexDirection: 'column' })}>
          <div
            class={css({
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              gap: '8px',
              width: 'full',
              height: '36px',
              backgroundColor: 'surface.subtle',
              paddingX: '20px',
              borderBottomWidth: '1px',
              borderColor: 'surface.muted',
            })}
          >
            <button
              class={css({
                display: 'flex',
                alignItems: 'center',
                flexGrow: '1',
                minWidth: '0',
                fontSize: '12px',
                fontWeight: 'semibold',
                color: 'text.subtle',
                borderWidth: '0',
                backgroundColor: 'transparent',
                textAlign: 'left',
                padding: '0',
                cursor: 'pointer',
                transition: 'common',
                _hover: { color: 'text.default' },
              })}
              onclick={() => focusRemarkGroup(group)}
              title={getGroupTitle(group)}
              type="button"
            >
              <span
                class={css({
                  display: 'block',
                  width: 'full',
                  overflow: 'hidden',
                  whiteSpace: 'nowrap',
                  textOverflow: 'ellipsis',
                })}
              >
                {getGroupTitle(group)}
              </span>
            </button>
            <button
              class={css({
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '28px',
                minWidth: '28px',
                paddingX: '12px',
                gap: '2px',
                borderRadius: '8px',
                borderWidth: '0',
                backgroundColor: 'transparent',
                color: 'text.subtle',
                cursor: 'pointer',
                transition: 'common',
                _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
              })}
              aria-label={collapsed ? '그룹 펼치기' : '그룹 접기'}
              onclick={() => toggleGroup(group.nodeId)}
              type="button"
            >
              {#if group.remarks.length >= 2}
                <span class={css({ fontSize: '11px', fontWeight: 'semibold', lineHeight: 'none' })}>{group.remarks.length}</span>
              {/if}
              <span
                style:transform={collapsed ? 'rotate(-90deg)' : 'rotate(0deg)'}
                class={css({ display: 'flex', transition: 'transform' })}
              >
                <Icon icon={ChevronDownIcon} size={14} />
              </span>
            </button>
          </div>

          {#if !collapsed}
            {#each group.remarks as remark (remark.remarkId)}
              <DocumentPanelRemarkItem onclick={() => focusRemark(remark, 'panel-item')} {remark} />
            {/each}
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>
