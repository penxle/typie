<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Menu, MenuItem } from '@typie/ui/components';
  import ClipboardPasteIcon from '~icons/lucide/clipboard-paste';
  import CopyIcon from '~icons/lucide/copy';
  import ScissorsIcon from '~icons/lucide/scissors';
  import SquareDashedIcon from '~icons/lucide/square-dashed';
  import { IS_MAC } from '$lib/editor/constants';
  import { getEditor } from '$lib/editor/context';

  const editor = getEditor();

  let open = $state(false);

  $effect(() => {
    open = editor.contextMenu.isOpen;
  });

  const contextMenuPosition = $derived(editor.contextMenu.isOpen ? { x: editor.contextMenu.x, y: editor.contextMenu.y } : null);

  const modKey = IS_MAC ? '⌘' : 'Ctrl+';
  const modKeyStyle = IS_MAC ? css.raw({ fontSize: '14px' }) : undefined;
  const shortcutStyle = flex.raw({ alignItems: 'center', marginLeft: 'auto', color: 'text.faint', fontSize: '12px' });
</script>

<Menu
  {contextMenuPosition}
  onclose={() => {
    editor.closeContextMenu();
    editor.focus();
  }}
  placement="bottom-start"
  bind:open
>
  {#snippet children({ close })}
    <MenuItem
      disabled={editor.selection.collapsed}
      icon={CopyIcon}
      onclick={() => {
        editor.handleCopy();
        close();
      }}
    >
      {#snippet suffix()}<span class={css(shortcutStyle)}>
          <span class={css(modKeyStyle)}>{modKey}</span>
          C
        </span>{/snippet}
      복사
    </MenuItem>
    {#if !editor.readOnly}
      <MenuItem
        disabled={editor.selection.collapsed}
        icon={ScissorsIcon}
        onclick={() => {
          editor.handleCut();
          close();
        }}
      >
        {#snippet suffix()}<span class={css(shortcutStyle)}>
            <span class={css(modKeyStyle)}>{modKey}</span>
            X
          </span>{/snippet}
        잘라내기
      </MenuItem>
      <MenuItem
        icon={ClipboardPasteIcon}
        onclick={() => {
          editor.handlePaste();
          close();
        }}
      >
        {#snippet suffix()}<span class={css(shortcutStyle)}>
            <span class={css(modKeyStyle)}>{modKey}</span>
            V
          </span>{/snippet}
        붙여넣기
      </MenuItem>
      <HorizontalDivider color="secondary" />
    {/if}
    <MenuItem
      icon={SquareDashedIcon}
      onclick={() => {
        editor.handleSelectAll();
        close();
      }}
    >
      {#snippet suffix()}<span class={css(shortcutStyle)}>
          <span class={css(modKeyStyle)}>{modKey}</span>
          A
        </span>{/snippet}
      전체 선택
    </MenuItem>
  {/snippet}
</Menu>
