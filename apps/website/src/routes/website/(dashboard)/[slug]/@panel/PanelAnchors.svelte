<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { calculateAnchorPositions, findAnchorableNode, getAnchorElements, getLastNodeOffsetTop } from '@typie/ui/anchor';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { clamp } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { onMount, tick } from 'svelte';
  import ArrowDownToLineIcon from '~icons/lucide/arrow-down-to-line';
  import ArrowUpToLineIcon from '~icons/lucide/arrow-up-to-line';
  import BookmarkPlusIcon from '~icons/lucide/bookmark-plus';
  import CheckIcon from '~icons/lucide/check';
  import PenIcon from '~icons/lucide/pen';
  import BookmarkFilledIcon from '~icons/typie/bookmark-filled';
  import { getViewContext } from '../@split-view/context.svelte';
  import type { Editor } from '@tiptap/core';
  import type { AnchorPosition } from '@typie/ui/anchor';
  import type { Ref } from '@typie/ui/utils';
  import type * as Y from 'yjs';

  type Props = {
    editor?: Ref<Editor>;
    doc: Y.Doc;
  };

  type Anchor = AnchorPosition & {
    isAnchor: boolean;
    isCurrent: boolean;
    isSpecial?: boolean;
    icon?: typeof ArrowUpToLineIcon;
  };

  let { editor, doc }: Props = $props();

  const view = getViewContext();

  let anchors = $state<Anchor[]>([]);
  let currentNode = $state<Anchor | null>(null);
  let isLoading = $state(true);
  let editingAnchor = $state<string | null>(null);
  let editingName = $state('');
  let docAnchors = $derived(editor?.current?.storage?.anchors?.current || {});
  let inputEl = $state<HTMLInputElement>();

  const loadAnchors = async () => {
    if (!editor?.current || !doc) return;

    try {
      await tick();

      const anchorElements = getAnchorElements(editor.current, Object.keys(docAnchors));

      const anchorPositions = calculateAnchorPositions(editor.current, anchorElements, docAnchors);

      anchors = anchorPositions.map((pos) => ({
        ...pos,
        isAnchor: true,
        isCurrent: false,
      }));
    } finally {
      isLoading = false;
    }
  };

  const updateCurrentNode = async () => {
    if (!editor?.current) return;

    const { nodeId } = findAnchorableNode(editor.current);

    if (!nodeId) {
      currentNode = null;
      return;
    }

    const element = editor.current.view.dom.querySelector(`[data-node-id="${nodeId}"]`) as HTMLElement;
    if (!element) {
      currentNode = null;
      return;
    }

    const lastNodeOffsetTop = getLastNodeOffsetTop(editor.current.view.dom);
    if (lastNodeOffsetTop === null) {
      currentNode = null;
      return;
    }

    const position = lastNodeOffsetTop > 0 ? clamp(element.offsetTop / lastNodeOffsetTop, 0, 1) : 0;

    currentNode = {
      nodeId,
      element,
      position,
      name: docAnchors[nodeId] || null,
      excerpt: element.textContent
        ? element.textContent.length > 20
          ? element.textContent.slice(0, 20) + '...'
          : element.textContent
        : '(내용 없음)',
      isAnchor: !!docAnchors[nodeId],
      isCurrent: true,
    };
  };

  const dummyElement = null as unknown as HTMLElement;
  const allNodes = $derived.by((): Anchor[] => {
    const nodes: Anchor[] = [];

    for (const anchor of anchors) {
      nodes.push({
        ...anchor,
        isCurrent: currentNode?.nodeId === anchor.nodeId,
      });
    }

    if (currentNode && !anchors.some((a) => a.nodeId === currentNode?.nodeId)) {
      nodes.push(currentNode);
    }

    const middleNodes = nodes.filter((node) => !node.isSpecial).toSorted((a, b) => a.position - b.position);

    return [
      {
        nodeId: 'top',
        name: '첫 줄로 가기',
        excerpt: '',
        position: 0,
        element: dummyElement,
        isAnchor: false,
        isCurrent: false,
        isSpecial: true,
        icon: ArrowUpToLineIcon,
      },
      ...middleNodes,
      {
        nodeId: 'bottom',
        name: '마지막 줄로 가기',
        excerpt: '',
        position: 1,
        element: dummyElement,
        isAnchor: false,
        isCurrent: false,
        isSpecial: true,
        icon: ArrowDownToLineIcon,
      },
    ];
  });

  const handleAnchorClick = async (anchor: Anchor) => {
    if (!editor?.current) return;

    if (anchor.nodeId === 'top') {
      editor.current.commands.focus('start');
      editor.current.commands.scrollIntoView();
      mixpanel.track('anchor_scroll_to_top');
    } else if (anchor.nodeId === 'bottom') {
      editor.current.commands.focus('end');
      editor.current.commands.scrollIntoView();
      mixpanel.track('anchor_scroll_to_bottom');
    } else if (anchor.element) {
      anchor.element.scrollIntoView({
        behavior: 'smooth',
        block: 'center',
      });

      const pos = editor.current.view.posAtDOM(anchor.element, 0);
      editor.current
        .chain()
        .setNodeSelection(pos - 1)
        .run();
      mixpanel.track('anchor_click');
    }
  };

  const toggleBookmark = async (anchor: Anchor) => {
    if (!editor?.current || !doc) return;

    if (anchor.isAnchor) {
      const { [anchor.nodeId]: removed, ...newAnchors } = docAnchors;
      void removed;
      editor.current.storage.anchors.current = newAnchors;
      mixpanel.track('anchor_remove');
    } else {
      const newAnchors = {
        ...docAnchors,
        [anchor.nodeId]: anchor.name || null,
      };
      editor.current.storage.anchors.current = newAnchors;
      mixpanel.track('anchor_add');
    }
  };

  const startEditingName = (anchor: Anchor) => {
    editingAnchor = anchor.nodeId;
    editingName = anchor.name || anchor.excerpt;
  };

  const saveAnchorName = async () => {
    if (!editingAnchor || !editingName.trim() || !editor?.current) {
      editingAnchor = null;
      return;
    }

    const newAnchors = {
      ...docAnchors,
      [editingAnchor]: editingName.trim(),
    };
    editor.current.storage.anchors.current = newAnchors;

    editingAnchor = null;

    mixpanel.track('anchor_rename');
  };

  $effect(() => {
    if (editingAnchor && inputEl) {
      inputEl.select();
    }
  });

  $effect(() => {
    if (currentNode) {
      setTimeout(() => {
        const elementInPanel = document.querySelector(`[data-view-id="${view.id}"] [data-anchor-current="true"]`);
        if (elementInPanel) {
          elementInPanel.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
      });
    }
  });

  $effect(() => {
    void docAnchors;
    loadAnchors();
  });

  onMount(() => {
    updateCurrentNode();

    editor?.current?.on('selectionUpdate', updateCurrentNode);

    return () => {
      editor?.current?.off('selectionUpdate', updateCurrentNode);
    };
  });
</script>

<div
  class={flex({
    flexDirection: 'column',
    minWidth: 'var(--min-width)',
    width: 'var(--width)',
    maxWidth: 'var(--max-width)',
    height: 'full',
  })}
>
  <div
    class={flex({
      flexShrink: '0',
      gap: '6px',
      height: '40px',
      alignItems: 'center',
      paddingX: '20px',
      fontSize: '13px',
      fontWeight: 'semibold',
      color: 'text.subtle',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    북마크
    {#if anchors.length > 0}
      <div
        class={css({
          borderRadius: '4px',
          paddingX: '6px',
          paddingY: '2px',
          fontSize: '11px',
          fontWeight: 'semibold',
          color: 'text.default',
          backgroundColor: 'surface.muted',
        })}
      >
        {anchors.length}
      </div>
    {/if}
  </div>
  {#if isLoading}
    <div class={flex({ justifyContent: 'center', alignItems: 'center', paddingY: '40px' })}>
      <RingSpinner style={css.raw({ size: '24px', color: 'text.faint' })} />
    </div>
  {:else}
    <div
      class={flex({
        flexDirection: 'column',
        gap: '8px',
        paddingX: '12px',
        paddingTop: '16px',
        paddingBottom: '100px',
        overflowY: 'auto',
      })}
      data-panel-anchors-scroll
    >
      {#each allNodes as anchor (anchor.nodeId)}
        <div
          class={css({
            position: 'relative',
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            borderWidth: '1px',
            borderColor: anchor.isCurrent ? 'border.strong' : 'border.default',
            borderRadius: '8px',
            paddingX: '10px',
            paddingY: '6px',
            cursor: 'pointer',
            transition: 'common',
            backgroundColor: anchor.isCurrent ? 'surface.subtle' : 'surface.default',
            _hover: {
              borderColor: 'border.strong',
              backgroundColor: 'surface.subtle',
            },
            _focusVisible: {
              borderColor: 'border.strong',
              backgroundColor: 'surface.subtle',
            },
          })}
          data-anchor-current={anchor.isCurrent}
          onclick={() => handleAnchorClick(anchor)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              handleAnchorClick(anchor);
            }
          }}
          onpointerdown={(e) => e.preventDefault()}
          role="button"
          tabindex="0"
        >
          <div class={center({ size: '40px', flexShrink: '0' })}>
            {#if anchor.isSpecial && anchor.icon}
              <Icon style={css.raw({ color: 'text.default' })} icon={anchor.icon} size={16} />
            {:else}
              <div
                class={css({
                  fontSize: '14px',
                  fontWeight: 'semibold',
                  color: anchor.isCurrent ? 'text.default' : 'text.faint',
                })}
              >
                {anchor.isCurrent ? '현재' : `${Math.round(anchor.position * 100)}%`}
              </div>
            {/if}
          </div>

          <div class={css({ flex: '1', minWidth: '0' })}>
            {#if editingAnchor === anchor.nodeId}
              <input
                bind:this={inputEl}
                class={css({
                  width: 'full',
                  padding: '4px',
                  borderWidth: '1px',
                  borderColor: 'border.strong',
                  borderRadius: '4px',
                  fontSize: '14px',
                  backgroundColor: 'surface.default',
                  outline: 'none',
                  _focus: {
                    borderColor: 'border.strong',
                  },
                })}
                maxlength="20"
                onblur={saveAnchorName}
                onclick={(e: MouseEvent) => e.stopPropagation()}
                onkeydown={(e: KeyboardEvent) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    saveAnchorName();
                  } else if (e.key === 'Escape') {
                    e.preventDefault();
                    editingAnchor = null;
                    mixpanel.track('anchor_reset');
                  }
                }}
                type="text"
                bind:value={editingName}
              />
            {:else}
              <div
                class={css({
                  fontSize: '14px',
                  fontWeight: 'medium',
                  color: 'text.default',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                })}
              >
                {anchor.name || anchor.excerpt}
              </div>
            {/if}
          </div>

          {#if !anchor.isSpecial}
            <div class={flex({ gap: '4px' })}>
              {#if anchor.isAnchor}
                <button
                  class={css({
                    padding: '6px',
                    borderRadius: '4px',
                    color: 'text.faint',
                    transition: 'common',
                    _hover: {
                      backgroundColor: 'interactive.hover',
                      color: 'text.subtle',
                    },
                  })}
                  onclick={(e) => {
                    e.stopPropagation();
                    if (editingAnchor === anchor.nodeId) {
                      saveAnchorName();
                    } else {
                      startEditingName(anchor);
                    }
                  }}
                  type="button"
                  use:tooltip={{
                    message: editingAnchor === anchor.nodeId ? '저장' : '이름 편집',
                    placement: 'top',
                  }}
                >
                  <Icon icon={editingAnchor === anchor.nodeId ? CheckIcon : PenIcon} size={14} />
                </button>
              {/if}

              <button
                class={css({
                  padding: '6px',
                  borderRadius: '4px',
                  transition: 'common',
                  color: anchor.isAnchor ? { base: '[#FACC15]', _dark: '[#B8860B]' } : 'text.faint',
                  _hover: {
                    backgroundColor: 'interactive.hover',
                    color: anchor.isAnchor ? { base: '[#FACC15]', _dark: '[#B8860B]' } : 'text.subtle',
                  },
                })}
                onclick={(e) => {
                  e.stopPropagation();
                  toggleBookmark(anchor);
                }}
                type="button"
                use:tooltip={{
                  message: anchor.isAnchor ? '북마크 제거' : '북마크 추가',
                  placement: 'top',
                }}
              >
                <Icon icon={anchor.isAnchor ? BookmarkFilledIcon : BookmarkPlusIcon} size={14} />
              </button>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
