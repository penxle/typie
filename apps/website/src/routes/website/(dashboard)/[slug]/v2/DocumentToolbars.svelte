<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { VerticalDivider } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import ArrowUpDownIcon from '~icons/lucide/arrow-up-down';
  import PlusIcon from '~icons/lucide/plus';
  import RedoIcon from '~icons/lucide/redo';
  import SearchIcon from '~icons/lucide/search';
  import TypeIcon from '~icons/lucide/type';
  import UndoIcon from '~icons/lucide/undo';
  import { FormatToolbarItems, InsertToolbarItems, ToolbarButton } from '$lib/editor-ffi/components';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { getPane, getPaneGroup } from '../@pane/context.svelte';
  import { otherToolbarKind, readPrimaryToolbar, writePrimaryToolbar } from './toolbar-kind';
  import ToolbarHorizontalScrollbar from './ToolbarHorizontalScrollbar.svelte';
  import type { Message } from '@typie/editor-ffi/browser';
  import type { ComponentProps } from 'svelte';
  import type { ToolbarKind } from './toolbar-kind';

  type Props = {
    documentId: string | null;
    fontFamilies?: ComponentProps<typeof FormatToolbarItems>['fontFamilies'];
    onFontUploadClick?: () => void;
    onSearchClick?: () => void;
  };

  let { documentId, fontFamilies = [], onFontUploadClick, onSearchClick }: Props = $props();

  const app = getAppContext();
  const ctx = getEditorContext();
  const paneId = getPane().id;
  const paneGroup = getPaneGroup();
  const primaryToolbarId = `document-toolbar-primary-${paneId}`;
  const expandedToolbarId = `document-toolbar-expanded-${paneId}`;

  let stored = $state<ToolbarKind | null>(null);
  let primaryScrollContainer = $state<HTMLElement>();
  let expandedScrollContainer = $state<HTMLElement>();

  const row = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '10px',
    paddingLeft: '20px',
    paddingRight: '12px',
    paddingY: '8px',
    overflowX: 'auto',
    scrollbar: 'hidden',
    width: 'full',
  });

  const rowShell = css.raw({
    flexShrink: '0',
    borderBottomWidth: '1px',
    borderColor: 'border.subtle',
    position: 'relative',
    backgroundColor: 'surface.default',
  });

  $effect(() => {
    if (documentId === null) return;
    stored = readPrimaryToolbar(documentId);
  });

  const primary = $derived(stored ?? app.preference.current.defaultPrimaryToolbar);
  const expanded = $derived(otherToolbarKind(primary));
  const open = $derived(paneGroup.state.current.toolbarExpandedByPaneId[paneId] ?? false);
  const editingDisabled = $derived(ctx.editor?.terminal === true || (ctx.editor !== undefined && ctx.editor !== ctx.liveEditor));

  const enqueue = (message: Message) => {
    if (editingDisabled) return;
    ctx.editor?.enqueue(message);
    ctx.editor?.focus();
  };

  const toggle = () => {
    const next = !open;
    paneGroup.state.current.toolbarExpandedByPaneId = {
      ...paneGroup.state.current.toolbarExpandedByPaneId,
      [paneId]: next,
    };
    mixpanel.track('toggle_expanded_toolbar', { open: next, kind: expanded });
  };

  const swap = () => {
    if (documentId === null) return;
    const next = expanded;
    writePrimaryToolbar(documentId, next);
    stored = next;
    mixpanel.track('swap_primary_toolbar', { primary: next });
  };
</script>

{#snippet items(kind: ToolbarKind)}
  {#if kind === 'insert'}
    <InsertToolbarItems />
  {:else}
    <FormatToolbarItems {fontFamilies} {onFontUploadClick} />
  {/if}
{/snippet}

<div class={css(rowShell, { zIndex: app.preference.current.zenModeEnabled ? 'underEditor' : 'overEditor' })} role="presentation">
  <div bind:this={primaryScrollContainer} id={primaryToolbarId} class={css(row)} role="toolbar" tabindex="-1">
    <ToolbarButton
      active={open}
      icon={expanded === 'insert' ? PlusIcon : TypeIcon}
      label={expanded === 'insert' ? '삽입 도구' : '서식 도구'}
      onclick={toggle}
      onpointerdown={(e) => e.preventDefault()}
    />

    <VerticalDivider style={css.raw({ height: '12px' })} />

    <div
      class={flex({
        alignItems: 'center',
        gap: '4px',
        opacity: editingDisabled ? '50' : '100',
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

    {@render items(primary)}

    <div class={css({ flexGrow: '1' })}></div>

    <div
      class={flex({
        alignItems: 'center',
        opacity: editingDisabled ? '50' : '100',
        pointerEvents: editingDisabled ? 'none' : 'auto',
      })}
    >
      <ToolbarButton
        icon={SearchIcon}
        keys={['Mod', 'F']}
        label="찾기 및 바꾸기"
        onclick={() => onSearchClick?.()}
        onpointerdown={(e) => e.preventDefault()}
      />
    </div>
  </div>

  <ToolbarHorizontalScrollbar controls={primaryToolbarId} scrollContainer={primaryScrollContainer} />
</div>

{#if open}
  <div class={css(rowShell, { zIndex: app.preference.current.zenModeEnabled ? 'underEditor' : 'overEditor' })} role="presentation">
    <div bind:this={expandedScrollContainer} id={expandedToolbarId} class={css(row)} role="toolbar" tabindex="-1">
      <ToolbarButton
        disabled={documentId === null}
        icon={ArrowUpDownIcon}
        label="기본 툴바와 맞바꾸기"
        onclick={swap}
        onpointerdown={(e) => e.preventDefault()}
      />

      <VerticalDivider style={css.raw({ height: '12px' })} />

      {@render items(expanded)}
    </div>

    <ToolbarHorizontalScrollbar controls={expandedToolbarId} scrollContainer={expandedScrollContainer} />
  </div>
{/if}
