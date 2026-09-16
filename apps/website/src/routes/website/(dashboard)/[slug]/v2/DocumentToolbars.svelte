<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Scrollbar, VerticalDivider } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import RedoIcon from '~icons/lucide/redo';
  import SearchIcon from '~icons/lucide/search';
  import UndoIcon from '~icons/lucide/undo';
  import { FormatToolbarItems, ToolbarButton, ToolbarInsertMenu } from '$lib/editor-ffi/components';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { getPane } from '../@pane/context.svelte';
  import { getZenModePaneChrome } from '../@pane/zen-mode-pane-chrome.svelte';
  import ZenModePaneChromeEffects from '../@pane/ZenModePaneChromeEffects.svelte';
  import ZenModePaneChromeSegment from '../@pane/ZenModePaneChromeSegment.svelte';
  import type { Message } from '@typie/editor-ffi/browser';
  import type { ComponentProps } from 'svelte';

  type Props = {
    fontFamilies?: ComponentProps<typeof FormatToolbarItems>['fontFamilies'];
    onFontUploadClick?: () => void;
    onSearchClick?: () => void;
  };

  let { fontFamilies = [], onFontUploadClick, onSearchClick }: Props = $props();

  const app = getAppContext();
  const ctx = getEditorContext();
  const paneId = getPane().id;
  const paneChrome = getZenModePaneChrome();
  const toolbarId = `document-toolbar-${paneId}`;

  let scrollContainer = $state<HTMLElement>();

  const editingDisabled = $derived(ctx.editor?.terminal === true || (ctx.editor !== undefined && ctx.editor !== ctx.liveEditor));
  const zenModeEnabled = $derived(app.preference.current.zenModeEnabled);
  const registerToolbarLane = paneChrome.registerToolbarLane;

  const enqueue = (message: Message) => {
    if (editingDisabled) return;
    ctx.editor?.enqueue(message);
    ctx.editor?.focus();
  };
</script>

<div
  class={css({
    position: zenModeEnabled ? 'absolute' : 'relative',
    top: zenModeEnabled ? '37px' : undefined,
    left: zenModeEnabled ? '0' : undefined,
    right: zenModeEnabled ? '0' : undefined,
    zIndex: 'overEditor',
    flexShrink: '0',
    pointerEvents: 'none',
  })}
  data-zen-mode-pane-chrome
  use:registerToolbarLane
>
  {#if zenModeEnabled}
    <ZenModePaneChromeEffects lane="toolbar" />
  {/if}

  <ZenModePaneChromeSegment class={css({ position: 'relative' })} active={zenModeEnabled} aria-label="문서 도구" segment="toolbar">
    <div
      class={css({
        flexShrink: '0',
        borderBottomWidth: '1px',
        borderColor: zenModeEnabled ? 'transparent' : 'border.hairline',
        position: 'relative',
        backgroundColor: zenModeEnabled ? 'transparent' : 'surface.default',
      })}
      role="presentation"
    >
      <div
        bind:this={scrollContainer}
        id={toolbarId}
        class={flex({
          alignItems: 'center',
          gap: '8px',
          paddingX: '12px',
          paddingY: '6px',
          overflowX: 'auto',
          scrollbar: 'hidden',
          width: 'full',
        })}
        role="toolbar"
        tabindex="-1"
      >
        <div
          class={flex({
            alignItems: 'center',
            gap: '2px',
            opacity: editingDisabled ? '40' : '100',
            pointerEvents: editingDisabled ? 'none' : 'auto',
          })}
        >
          <ToolbarButton
            style={css.raw({ borderRightRadius: '0' })}
            icon={UndoIcon}
            keys={['Mod', 'Z']}
            label="실행 취소"
            onclick={() => enqueue({ type: 'history', op: { type: 'undo' } })}
          />

          <ToolbarButton
            style={css.raw({ borderLeftRadius: '0' })}
            icon={RedoIcon}
            keys={['Mod', 'Shift', 'Z']}
            label="다시 실행"
            onclick={() => enqueue({ type: 'history', op: { type: 'redo' } })}
          />
        </div>

        <VerticalDivider style={css.raw({ height: '12px' })} />

        <ToolbarInsertMenu disabled={editingDisabled} />

        <VerticalDivider style={css.raw({ height: '12px' })} />

        <FormatToolbarItems {fontFamilies} {onFontUploadClick} />

        <div class={css({ flexGrow: '1' })}></div>

        <div
          class={flex({
            alignItems: 'center',
            opacity: editingDisabled ? '40' : '100',
            pointerEvents: editingDisabled ? 'none' : 'auto',
          })}
        >
          <ToolbarButton icon={SearchIcon} keys={['Mod', 'F']} label="찾기 및 바꾸기" onclick={() => onSearchClick?.()} />
        </div>
      </div>

      <Scrollbar controls={toolbarId} label="툴바 가로 스크롤" orientation="horizontal" {scrollContainer} size="sm" />
    </div>
  </ZenModePaneChromeSegment>
</div>
