<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem, TimeAgo } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import PencilIcon from '~icons/lucide/pencil';
  import Trash2Icon from '~icons/lucide/trash-2';
  import XIcon from '~icons/lucide/x';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    editor: Editor;
    nodeId: string;
    remarkId: string;
    userId: string;
    text: string;
    createdAt: number;
    readOnly?: boolean;
    onDirtyChange?: (remarkId: string, dirty: boolean) => void;
  };

  let { editor, nodeId, remarkId, userId, text, createdAt, readOnly = false, onDirtyChange }: Props = $props();

  const userQuery = createQuery(
    graphql(`
      query RemarkCard_Query($userId: ID!) {
        userView(id: $userId) {
          id
          name
          avatar {
            id
            ...Img_image
          }
        }
      }
    `),
    () => ({ userId }),
  );

  let menuOpen = $state(false);
  let editing = $state(false);
  let editText = $state(text);
  let textareaEl = $state<HTMLTextAreaElement>();
  const hasEditText = $derived($state.eager(editText).trim().length > 0);
  const hasEditContent = $derived($state.eager(editText.length) > 0);

  $effect(() => {
    if (!(editing && textareaEl)) {
      return;
    }

    textareaEl.focus();
    textareaEl.scrollIntoView({ block: 'nearest' });
  });

  $effect(() => {
    onDirtyChange?.(remarkId, editing && editText !== text);
  });

  const save = () => {
    const trimmed = editText.trim();
    if (!trimmed) return;

    editor.dispatch({ type: 'updateRemark', nodeId, remarkId, text: trimmed });
    editing = false;
  };

  const cancel = () => {
    editText = text;
    editing = false;
  };

  const remove = () => {
    Dialog.confirm({
      title: '코멘트 삭제',
      message: '코멘트를 삭제하시겠어요?',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: () => {
        editor.dispatch({ type: 'removeRemark', nodeId, remarkId });
      },
    });
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      save();
    } else if (e.key === 'Escape') {
      e.stopPropagation();
      if (editText === text) {
        cancel();
      } else {
        Dialog.confirm({
          title: '수정 취소',
          message: '수정을 취소하시겠어요?',
          action: 'danger',
          actionLabel: '수정 취소',
          actionHandler: () => {
            cancel();
          },
        });
      }
    }
  };
</script>

<div
  class={`group ${css({
    display: 'flex',
    gap: '8px',
  })}`}
>
  {#if userQuery.data?.userView.avatar}
    <Img
      style={css.raw({ width: '24px', height: '24px', borderRadius: 'full', flexShrink: '0', marginTop: '1px' })}
      alt={userQuery.data.userView.name}
      image$key={userQuery.data.userView.avatar}
      size={24}
    />
  {:else if !userQuery.data}
    <div
      class={css({
        width: '24px',
        height: '24px',
        borderRadius: 'full',
        flexShrink: '0',
        marginTop: '1px',
        backgroundColor: 'surface.muted',
        animation: 'pulse 1.5s ease-in-out infinite',
      })}
    ></div>
  {/if}

  <div class={css({ flexGrow: '1', minWidth: '0' })}>
    <div class={css({ display: 'flex', alignItems: 'center', gap: '4px', marginBottom: '2px', minHeight: '20px' })}>
      {#if userQuery.data}
        <span
          class={css({
            fontSize: '13px',
            fontWeight: 'semibold',
            color: 'text.default',
            truncate: true,
            flexShrink: '1',
            minWidth: '0',
          })}
        >
          {userQuery.data.userView.name}
        </span>
        <TimeAgo style={css.raw({ fontSize: '11px', color: 'text.faint', flexShrink: '0' })} timestamp={createdAt} />

        {#if !readOnly}
          <div
            style:opacity={menuOpen ? '1' : undefined}
            style:visibility={editing ? 'hidden' : undefined}
            style:pointer-events={editing ? 'none' : undefined}
            class={css({
              marginLeft: 'auto',
              opacity: '0',
              transition: 'common',
              _groupHover: { opacity: '100' },
            })}
          >
            <Menu
              style={css.raw({ display: 'flex', padding: '0', borderWidth: '0', backgroundColor: 'transparent' })}
              listStyle={css.raw({ minWidth: '100px' })}
              offset={4}
              placement="bottom-end"
              bind:open={menuOpen}
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
                    menuOpen && { backgroundColor: 'surface.muted', color: 'text.subtle' },
                  )}
                  type="button"
                >
                  <Icon icon={EllipsisIcon} size={12} />
                </button>
              {/snippet}

              <MenuItem
                icon={PencilIcon}
                onclick={() => {
                  editText = text;
                  editing = true;
                }}
              >
                수정
              </MenuItem>
              <MenuItem icon={Trash2Icon} onclick={remove} variant="danger">삭제</MenuItem>
            </Menu>
          </div>
        {/if}
      {:else}
        <div
          class={css({
            width: '60px',
            height: '[1lh]',
            fontSize: '13px',
            fontWeight: 'semibold',
            borderRadius: '4px',
            backgroundColor: 'surface.muted',
            animation: 'pulse 1.5s ease-in-out infinite',
          })}
        ></div>
      {/if}
    </div>

    {#if editing}
      <div class={css({ position: 'relative', display: 'flex' })}>
        <textarea
          bind:this={textareaEl}
          style:padding-right={hasEditContent ? '10px' : '40px'}
          style:padding-bottom={hasEditContent ? '40px' : '8px'}
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
            transition: 'common',
            _focus: { borderColor: 'accent.brand.default', backgroundColor: 'surface.default' },
          })}
          onkeydown={handleKeydown}
          rows={1}
          bind:value={editText}
          use:autosize></textarea>
        <div
          style:top={hasEditContent ? 'auto' : '50%'}
          style:bottom={hasEditContent ? '6px' : 'auto'}
          style:transform={hasEditContent ? undefined : 'translateY(-50%)'}
          class={css({ position: 'absolute', right: '6px', display: 'flex', alignItems: 'center', gap: '4px', transition: 'common' })}
        >
          <button
            class={css({
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: '22px',
              height: '22px',
              borderRadius: 'full',
              borderWidth: '0',
              cursor: 'pointer',
              backgroundColor: 'surface.subtle',
              color: 'text.faint',
              transition: 'common',
              _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
            })}
            onclick={cancel}
            type="button"
            use:tooltip={{ message: '수정 취소', placement: 'bottom' }}
          >
            <Icon icon={XIcon} size={12} />
          </button>
          <button
            class={css(
              {
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: '22px',
                height: '22px',
                borderRadius: 'full',
                borderWidth: '0',
                transition: 'common',
              },
              hasEditText
                ? {
                    cursor: 'pointer',
                    backgroundColor: 'accent.brand.default',
                    color: 'text.bright',
                    _hover: { backgroundColor: 'accent.brand.hover' },
                    _active: { backgroundColor: 'accent.brand.active' },
                  }
                : { cursor: 'default', backgroundColor: 'surface.muted', color: 'text.disabled' },
            )}
            disabled={!hasEditText}
            onclick={save}
            type="button"
            use:tooltip={{ message: '저장', placement: 'bottom' }}
          >
            <Icon icon={ArrowUpIcon} size={12} />
          </button>
        </div>
      </div>
    {:else}
      <p
        class={css({
          margin: '0',
          fontSize: '13px',
          lineHeight: '[1.4]',
          color: 'text.subtle',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
          userSelect: 'text',
          cursor: 'text',
        })}
      >
        {text}
      </p>
    {/if}
  </div>
</div>
