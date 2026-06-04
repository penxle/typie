<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Menu, MenuItem } from '@typie/ui/components';
  import ClipboardPasteIcon from '~icons/lucide/clipboard-paste';
  import ClipboardTypeIcon from '~icons/lucide/clipboard-type';
  import CopyIcon from '~icons/lucide/copy';
  import ScissorsIcon from '~icons/lucide/scissors';
  import SquareDashedIcon from '~icons/lucide/square-dashed';
  import { IS_MAC } from '$lib/editor-ffi/constants';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';

  const ctx = getEditorContext();

  let open = $state(false);

  $effect(() => {
    open = !!ctx.editor && ctx.editor.contextMenu.isOpen && ctx.editor.contextMenu.source === 'mouse';
  });

  $effect(() => {
    if (!open && ctx.editor?.contextMenu.isOpen && ctx.editor.contextMenu.source === 'mouse') {
      ctx.editor.closeContextMenu();
      ctx.editor.focus();
    }
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
</script>

{#if ctx.editor}
  <Menu {contextMenuPosition} offset={6} placement={contextMenuPlacement} bind:open>
    {#snippet children({ close })}
      <MenuItem
        disabled={(ctx.editor?.isSelectionCollapsed ?? true) || !!(ctx.editor?.readOnly && ctx.editor?.protectContent)}
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
      {#if !ctx.editor?.readOnly}
        <MenuItem
          disabled={ctx.editor?.isSelectionCollapsed ?? true}
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
          icon={ClipboardPasteIcon}
          onclick={() => {
            void ctx.editor?.requestPaste();
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
      {/if}
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
