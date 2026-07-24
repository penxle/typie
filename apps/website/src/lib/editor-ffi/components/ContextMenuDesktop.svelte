<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Menu, MenuItem } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import ClipboardPasteIcon from '~icons/lucide/clipboard-paste';
  import ClipboardTypeIcon from '~icons/lucide/clipboard-type';
  import CopyIcon from '~icons/lucide/copy';
  import ScissorsIcon from '~icons/lucide/scissors';
  import SquareDashedIcon from '~icons/lucide/square-dashed';
  import { IS_MAC } from '$lib/editor-ffi/constants';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { requestPaste } from '../handlers/clipboard';
  import { getContextMenuCapabilityState } from './context-menu-state';

  const ctx = getEditorContext();

  let open = $state(false);

  $effect(() => {
    open = !!ctx.editor && ctx.editor.contextMenu.isOpen && ctx.editor.contextMenu.source === 'mouse';
  });

  $effect(() => {
    if (!(!open && ctx.editor?.contextMenu.isOpen && ctx.editor.contextMenu.source === 'mouse')) {
      return;
    }

    ctx.editor.closeContextMenu();
    ctx.editor.focus();
  });

  const contextMenuPosition = $derived(
    ctx.editor?.contextMenu.isOpen && ctx.editor.contextMenu.source === 'mouse'
      ? { x: ctx.editor.contextMenu.x, y: ctx.editor.contextMenu.y }
      : null,
  );
  const contextMenuPlacement = $derived(ctx.editor?.contextMenu.placement ?? 'bottom-start');

  const modKey = IS_MAC ? '⌘' : 'Ctrl+';
  const modKeyStyle = IS_MAC ? css.raw({ fontSize: '14px' }) : undefined;
  const shortcutStyle = flex.raw({ alignItems: 'center', marginLeft: 'auto', color: 'text.faint', fontSize: '12px' });

  const shiftKey = IS_MAC ? '⇧' : 'Shift+';

  const extraItems = $derived(ctx.editor?.contextMenu.extraItems ?? []);
  const capabilityState = $derived(
    getContextMenuCapabilityState({
      isSelectionCollapsed: ctx.editor?.isSelectionCollapsed ?? true,
      readOnly: ctx.editor?.readOnly ?? false,
      protectContent: ctx.editor?.protectContent ?? false,
    }),
  );
</script>

{#if ctx.editor}
  <Menu {contextMenuPosition} offset={6} placement={contextMenuPlacement} bind:open>
    {#snippet children({ close })}
      <MenuItem
        disabled={capabilityState.copyDisabled}
        icon={CopyIcon}
        onclick={() => {
          void ctx.editor?.requestCopy();
          close();
        }}
      >
        {#snippet suffix()}
          <span class={css(shortcutStyle)}>
            <span class={css(modKeyStyle)}>{modKey}</span>
            C
          </span>
        {/snippet}
        복사
      </MenuItem>
      <MenuItem
        disabled={capabilityState.cutDisabled}
        icon={ScissorsIcon}
        onclick={() => {
          void ctx.editor?.requestCut();
          close();
        }}
      >
        {#snippet suffix()}
          <span class={css(shortcutStyle)}>
            <span class={css(modKeyStyle)}>{modKey}</span>
            X
          </span>
        {/snippet}
        잘라내기
      </MenuItem>
      <MenuItem
        disabled={capabilityState.pasteDisabled}
        icon={ClipboardPasteIcon}
        onclick={() => {
          void requestPaste(ctx, ({ file, kind }) => {
            Toast.error(`${file.name} ${kind === 'image' ? '이미지' : '파일'} 업로드에 실패했습니다.`);
          });
          close();
        }}
      >
        {#snippet suffix()}
          <span class={css(shortcutStyle)}>
            <span class={css(modKeyStyle)}>{modKey}</span>
            V
          </span>
        {/snippet}
        붙여넣기
      </MenuItem>
      <MenuItem
        disabled={capabilityState.pasteDisabled}
        icon={ClipboardTypeIcon}
        onclick={() => {
          void ctx.editor?.requestPasteTextOnly();
          close();
        }}
      >
        {#snippet suffix()}
          <span class={css(shortcutStyle)}>
            <span class={css(modKeyStyle)}>{modKey}</span>
            <span>{shiftKey}</span>
            V
          </span>
        {/snippet}
        서식 없이 붙여넣기
      </MenuItem>
      <HorizontalDivider color="secondary" />
      <MenuItem
        icon={SquareDashedIcon}
        onclick={() => {
          ctx.editor?.requestSelectAll();
          close();
        }}
      >
        {#snippet suffix()}
          <span class={css(shortcutStyle)}>
            <span class={css(modKeyStyle)}>{modKey}</span>
            A
          </span>
        {/snippet}
        전체 선택
      </MenuItem>

      {#if extraItems.length > 0}
        <HorizontalDivider color="secondary" />
        {#each extraItems as item, i (i)}
          <MenuItem
            icon={item.icon}
            onclick={() => {
              void item.onclick();
              close();
            }}
            variant={item.variant}
          >
            {item.label}
          </MenuItem>
        {/each}
      {/if}
    {/snippet}
  </Menu>
{/if}
