<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { autosize, createFloatingActions, tooltip } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { tick } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { scale } from 'svelte/transition';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import MessageSquarePlusIcon from '~icons/lucide/message-square-plus';
  import MessageSquareTextIcon from '~icons/lucide/message-square-text';
  import Trash2Icon from '~icons/lucide/trash-2';
  import XIcon from '~icons/lucide/x';
  import { fragment, graphql } from '$graphql';
  import { Img } from '$lib/components';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import RemarkCard from './RemarkCard.svelte';
  import type { RemarkPopover_user } from '$graphql';
  import type { Editor } from '$lib/editor/editor.svelte';
  import type { RemarkOverlay } from '$lib/editor/slate';

  const REMARK_WIDTH = 280;

  type Props = {
    editor: Editor;
    nodeId: string;
    remarks: RemarkOverlay[];
    open: boolean;
    onToggle: () => void;
  };

  let { editor, nodeId, remarks, open, onToggle }: Props = $props();

  const ctx = getEditorContext();

  const user = fragment(
    ctx.user as RemarkPopover_user | null,
    graphql(`
      fragment RemarkPopover_user on User {
        id
        name
        avatar {
          ...Img_image
        }
      }
    `),
  );

  let newRemarkText = $state('');
  let textareaEl = $state<HTMLTextAreaElement>();
  let listEl = $state<HTMLDivElement>();
  let headerMenuOpen = $state(false);
  const hasText = $derived($state.eager(newRemarkText).trim().length > 0);
  const hasContent = $derived($state.eager(newRemarkText).length > 0);

  const dirtyRemarkIds = new SvelteSet<string>();
  const hasUnsavedChanges = $derived(newRemarkText.trim().length > 0 || dirtyRemarkIds.size > 0);

  $effect(() => {
    if (open) {
      tick()
        .then(tick)
        .then(() => {
          if (listEl) {
            listEl.scrollTop = listEl.scrollHeight;
          }
          textareaEl?.focus();
        });
    }
  });

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom-start',
    offset: 4,
    onClickOutside: () => {
      close();
    },
  });

  $effect(() => {
    if (open) {
      return pushEscapeHandler(() => {
        if (open) {
          close();
          return true;
        }
        return false;
      });
    }
  });

  function tryClose() {
    if (hasUnsavedChanges) {
      Dialog.confirm({
        title: '작성 중인 내용 삭제',
        message: '작성 중인 내용이 사라집니다. 닫으시겠어요?',
        action: 'danger',
        actionLabel: '닫기',
        actionHandler: () => {
          onToggle();
        },
      });
    } else {
      onToggle();
    }
  }

  function toggle(e: MouseEvent) {
    e.stopPropagation();
    if (open) {
      tryClose();
    } else {
      onToggle();
    }
  }

  function close() {
    if (open) {
      tryClose();
    }
  }

  $effect(() => {
    if (!open) {
      newRemarkText = '';
      dirtyRemarkIds.clear();
    }
  });

  function removeAllRemarks() {
    Dialog.confirm({
      title: '전체 코멘트 삭제',
      message: '이 블록의 코멘트를 모두 삭제하시겠어요? 되돌릴 수 없어요.',
      action: 'danger',
      actionLabel: '전체 삭제',
      actionHandler: () => {
        for (const remark of remarks) {
          editor.dispatch({ type: 'removeRemark', nodeId, remarkId: remark.remarkId });
        }
      },
    });
  }

  function addRemark() {
    const trimmed = newRemarkText.trim();
    if (!trimmed) return;

    editor.dispatch({ type: 'addRemark', nodeId, userId: $user?.id ?? '', text: trimmed, createdAt: Date.now() });
    newRemarkText = '';
    editor
      .settled()
      .then(tick)
      .then(() => {
        if (listEl) {
          listEl.scrollTo({ top: listEl.scrollHeight, behavior: 'smooth' });
        }
      });
  }
</script>

<button
  class={css(
    {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: '6px',
      height: '24px',
      paddingX: '4px',
      borderRadius: '4px',
      cursor: 'pointer',
      borderWidth: '0',
      transition: 'common',
    },
    remarks.length > 0
      ? {
          backgroundColor: 'transparent',
          color: 'text.faint',
          _hover: { backgroundColor: 'surface.muted', color: 'text.muted' },
        }
      : {
          backgroundColor: 'transparent',
          color: 'text.faint',
          _hover: { backgroundColor: 'surface.muted', color: 'text.muted' },
        },
    open && { backgroundColor: 'surface.muted', color: 'text.subtle' },
  )}
  aria-label={remarks.length > 0 ? 'Show remarks' : 'Add remark'}
  onclick={toggle}
  onpointerdown={(e) => e.stopPropagation()}
  type="button"
  use:anchor
>
  {#if remarks.length > 0}
    <Icon icon={MessageSquareTextIcon} size={14} />
    <span class={css({ fontSize: '12px', fontWeight: 'bold', lineHeight: 'none' })}>
      {remarks.length}
    </span>
  {:else}
    <Icon icon={MessageSquarePlusIcon} size={14} />
  {/if}
</button>

{#if open}
  <div
    style:width="{REMARK_WIDTH}px"
    class={css({
      borderWidth: '1px',
      borderColor: 'border.subtle',
      borderRadius: '8px',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
      zIndex: 'menu',
      pointerEvents: 'auto',
      overflow: 'hidden',
      transformOrigin: 'top left',
    })}
    onclick={(e) => e.stopPropagation()}
    role="presentation"
    use:floating
    transition:scale={{ start: 0.95, duration: 150 }}
  >
    <div
      class={css({
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        paddingX: '10px',
        paddingY: '8px',
        borderBottomWidth: '1px',
        borderColor: 'border.subtle',
      })}
    >
      <div class={css({ display: 'flex', alignItems: 'center', gap: '6px' })}>
        <span class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default' })}>코멘트</span>
        {#if remarks.length > 0}
          <span
            class={css({
              fontSize: '11px',
              fontWeight: 'bold',
              lineHeight: 'none',
              color: 'text.subtle',
              backgroundColor: 'surface.muted',
              borderRadius: '4px',
              paddingX: '6px',
              paddingY: '2px',
            })}
          >
            {remarks.length}
          </span>
        {/if}
      </div>
      <div class={css({ display: 'flex', alignItems: 'center', gap: '2px' })}>
        {#if !editor.readOnly && remarks.length > 0}
          <Menu
            style={{ display: 'flex', padding: '0', borderWidth: '0', backgroundColor: 'transparent' }}
            listStyle={{ minWidth: '120px' }}
            offset={4}
            placement="bottom-end"
            bind:open={headerMenuOpen}
          >
            {#snippet button()}
              <button
                class={css(
                  {
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    width: '20px',
                    height: '20px',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    backgroundColor: 'transparent',
                    borderWidth: '0',
                    color: 'text.faint',
                    transition: 'common',
                    _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
                  },
                  headerMenuOpen && { backgroundColor: 'surface.muted', color: 'text.subtle' },
                )}
                type="button"
              >
                <Icon icon={EllipsisIcon} size={12} />
              </button>
            {/snippet}

            <MenuItem icon={Trash2Icon} onclick={removeAllRemarks} variant="danger">전체 삭제</MenuItem>
          </Menu>
        {/if}
        <button
          class={css({
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: '20px',
            height: '20px',
            borderRadius: '4px',
            borderWidth: '0',
            cursor: 'pointer',
            backgroundColor: 'transparent',
            color: 'text.faint',
            transition: 'common',
            _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
          })}
          onclick={() => tryClose()}
          type="button"
        >
          <Icon icon={XIcon} size={12} />
        </button>
      </div>
    </div>

    {#if remarks.length > 0}
      <div
        bind:this={listEl}
        class={css({
          display: 'flex',
          flexDirection: 'column',
          gap: '8px',
          paddingY: '8px',
          paddingLeft: '10px',
          paddingRight: '8px',
          maxHeight: '300px',
          overflowY: 'auto',
        })}
      >
        {#each remarks as remark (remark.remarkId)}
          <RemarkCard
            createdAt={remark.createdAt}
            {editor}
            nodeId={remark.nodeId}
            onDirtyChange={(id, dirty) => {
              if (dirty) {
                dirtyRemarkIds.add(id);
              } else {
                dirtyRemarkIds.delete(id);
              }
            }}
            readOnly={editor.readOnly}
            remarkId={remark.remarkId}
            text={remark.text}
            userId={remark.userId}
          />
        {/each}
      </div>
    {/if}

    {#if !editor.readOnly}
      {#if remarks.length > 0}
        <div class={css({ borderTopWidth: '1px', borderColor: 'border.subtle' })}></div>
      {/if}
      <div
        class={css({ display: 'flex', gap: '8px', alignItems: 'flex-start', paddingY: '8px', paddingLeft: '10px', paddingRight: '8px' })}
      >
        {#if $user?.avatar}
          <Img
            style={css.raw({ width: '24px', height: '24px', borderRadius: 'full', flexShrink: '0', marginTop: '1px' })}
            $image={$user.avatar}
            alt={$user.name}
            size={24}
          />
        {/if}
        <div class={css({ position: 'relative', display: 'flex', flexGrow: '1', minWidth: '0' })}>
          <textarea
            bind:this={textareaEl}
            style:padding-right={hasContent ? '10px' : '40px'}
            style:padding-bottom={hasContent ? '40px' : '8px'}
            class={css({
              width: 'full',
              borderWidth: '1px',
              borderColor: 'border.default',
              borderRadius: '6px',
              paddingLeft: '10px',
              paddingTop: '8px',
              fontSize: '13px',
              lineHeight: '[1.4]',
              color: 'text.default',
              backgroundColor: 'surface.subtle',
              resize: 'none',
              minHeight: '36px',
              maxHeight: '120px',
              outline: 'none',
              transition: 'colors',
              _focus: { borderColor: 'accent.brand.default', backgroundColor: 'surface.default' },
            })}
            onkeydown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
                e.preventDefault();
                addRemark();
              } else if (e.key === 'Escape' && newRemarkText.trim() !== '') {
                e.stopPropagation();
                Dialog.confirm({
                  title: '작성 중인 내용 삭제',
                  message: '작성 중인 내용을 지우시겠어요?',
                  action: 'danger',
                  actionLabel: '지우기',
                  actionHandler: () => {
                    newRemarkText = '';
                  },
                });
              }
            }}
            placeholder="코멘트 입력..."
            rows={1}
            bind:value={newRemarkText}
            use:autosize
          ></textarea>
          <button
            style:top={hasContent ? 'auto' : '50%'}
            style:bottom={hasContent ? '6px' : 'auto'}
            style:transform={hasContent ? undefined : 'translateY(-50%)'}
            class={css(
              {
                position: 'absolute',
                right: '6px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: '22px',
                height: '22px',
                borderRadius: 'full',
                borderWidth: '0',
                transition: 'common',
              },
              hasText
                ? {
                    cursor: 'pointer',
                    backgroundColor: 'accent.brand.default',
                    color: 'text.bright',
                    _hover: { backgroundColor: 'accent.brand.hover' },
                    _active: { backgroundColor: 'accent.brand.active' },
                  }
                : { cursor: 'default', backgroundColor: 'surface.muted', color: 'text.disabled' },
            )}
            disabled={!hasText}
            onclick={addRemark}
            type="button"
            use:tooltip={{ message: '보내기', placement: 'bottom' }}
          >
            <Icon icon={ArrowUpIcon} size={12} />
          </button>
        </div>
      </div>
    {/if}
  </div>
{/if}
