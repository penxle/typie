<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { VerticalDivider } from '@typie/ui/components';
  import BoldIcon from '~icons/lucide/bold';
  import ItalicIcon from '~icons/lucide/italic';
  import LinkIcon from '~icons/lucide/link';
  import MessageSquarePlusIcon from '~icons/lucide/message-square-plus';
  import RemoveFormattingIcon from '~icons/lucide/remove-formatting';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import UnderlineIcon from '~icons/lucide/underline';
  import RubyIcon from '~icons/typie/ruby';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarColorDropdown from './ToolbarColorDropdown.svelte';
  import ToolbarFontFamily from './ToolbarFontFamily.svelte';
  import ToolbarFontSize from './ToolbarFontSize.svelte';
  import ToolbarFontWeight from './ToolbarFontWeight.svelte';
  import ToolbarParagraphDropdown from './ToolbarParagraphDropdown.svelte';
  import type { Message, ModifierType, Tri } from '@typie/editor-ffi/browser';

  type Font = { id?: string | null; weight: number; subfamilyDisplayName?: string | null; state: string };
  type FontFamily = { id: string; familyName: string; displayName: string; state: string; fonts: readonly Font[] };

  type Props = {
    fontFamilies?: readonly FontFamily[];
    onFontUploadClick?: () => void;
  };

  let { fontFamilies = [], onFontUploadClick }: Props = $props();

  const ctx = getEditorContext();

  type ToggleState = { active: boolean; indeterminate: boolean };

  const toggleState = (tri: Tri<undefined> | undefined): ToggleState => {
    if (tri?.type === 'uniform') return { active: true, indeterminate: false };
    if (tri?.type === 'mixed') return { active: false, indeterminate: true };
    return { active: false, indeterminate: false };
  };

  const textFormattingDisabled = $derived.by(() => {
    const state = ctx.editor?.modifierState;
    if (!state) return false;
    return [
      state.bold,
      state.italic,
      state.underline,
      state.strikethrough,
      state.font_size,
      state.font_family,
      state.font_weight,
      state.text_color,
      state.background_color,
      state.letter_spacing,
      state.link,
      state.ruby,
    ].every((tri) => tri.type === 'absent');
  });

  const alignmentDisabled = $derived(ctx.editor?.modifierState?.alignment?.type === 'absent');
  const lineHeightDisabled = $derived(ctx.editor?.modifierState?.line_height?.type === 'absent');

  const boldS = $derived(toggleState(ctx.editor?.modifierState?.effective_bold));
  const italicS = $derived(toggleState(ctx.editor?.modifierState?.italic));
  const strikethroughS = $derived(toggleState(ctx.editor?.modifierState?.strikethrough));
  const underlineS = $derived(toggleState(ctx.editor?.modifierState?.underline));

  const isLinkActive = $derived(ctx.editor?.modifierState?.link?.type === 'uniform');
  const isLinkMixed = $derived(ctx.editor?.modifierState?.link?.type === 'mixed');
  const isRubyActive = $derived(ctx.editor?.modifierState?.ruby?.type === 'uniform');
  const isRubyMixed = $derived(ctx.editor?.modifierState?.ruby?.type === 'mixed');

  const isCollapsed = $derived(ctx.editor?.isSelectionCollapsed ?? true);
  const editingDisabled = $derived(ctx.editor?.terminal === true || (ctx.editor !== undefined && ctx.editor !== ctx.liveEditor));

  const enqueue = (message: Message) => {
    if (editingDisabled) return;
    ctx.editor?.enqueue(message);
    ctx.editor?.focus();
  };

  const toggleModifier = (modifier_type: ModifierType) => {
    enqueue({ type: 'modifier', op: { type: 'toggle', modifier_type } });
  };

  // When opening the link/ruby editor with a *collapsed* caret inside a mark,
  // extend the selection over that whole mark span so editing applies to the
  // entire mark and the range is visible. A range selection (partial or spanning
  // several marks/plain text) is always preserved so editing applies to exactly
  // what the user selected.
  const extendSelectionToSpan = (modifier_type: 'link' | 'ruby') => {
    const editor = ctx.editor;
    if (editingDisabled || !editor) return;
    const selection = editor.selection;
    if (!selection || !editor.isSelectionCollapsed) return;

    const span = editor.modifierSpanSelection(selection.head, modifier_type);
    if (!span) return;

    editor.updateNow(() => editor.enqueue({ type: 'selection', op: { type: 'set', selection: span } }));
  };

  const openMarkCard = (kind: 'link' | 'ruby') => {
    if (ctx.markCard?.kind === kind) {
      ctx.markCard = null;
      ctx.editor?.focus();
      return;
    }
    const exists = kind === 'link' ? isLinkActive : isRubyActive;
    extendSelectionToSpan(kind);
    ctx.markCard = { kind, mode: exists ? 'view' : 'edit', anchor: null };
  };

  const group = flex({ alignItems: 'center', gap: '2px' });
</script>

<div
  class={flex({
    alignItems: 'center',
    gap: '8px',
    opacity: editingDisabled ? '40' : '100',
    pointerEvents: editingDisabled ? 'none' : 'auto',
  })}
>
  <div class={group}>
    <ToolbarColorDropdown disabled={textFormattingDisabled} kind="text" />
    <ToolbarColorDropdown disabled={textFormattingDisabled} kind="background" />
    <ToolbarFontFamily disabled={textFormattingDisabled} {fontFamilies} onUploadClick={onFontUploadClick} />
    <ToolbarFontWeight disabled={textFormattingDisabled} {fontFamilies} />
    <ToolbarFontSize disabled={textFormattingDisabled} />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={group}>
    <ToolbarButton
      active={boldS.active}
      disabled={textFormattingDisabled}
      icon={BoldIcon}
      keys={['Mod', 'B']}
      label="굵게"
      onclick={() => toggleModifier('bold')}
    />

    <ToolbarButton
      active={italicS.active}
      disabled={textFormattingDisabled}
      icon={ItalicIcon}
      keys={['Mod', 'I']}
      label="기울임"
      onclick={() => toggleModifier('italic')}
    />

    <ToolbarButton
      active={strikethroughS.active}
      disabled={textFormattingDisabled}
      icon={StrikethroughIcon}
      keys={['Mod', 'Shift', 'S']}
      label="취소선"
      onclick={() => toggleModifier('strikethrough')}
    />

    <ToolbarButton
      active={underlineS.active}
      disabled={textFormattingDisabled}
      icon={UnderlineIcon}
      keys={['Mod', 'U']}
      label="밑줄"
      onclick={() => toggleModifier('underline')}
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={group}>
    <ToolbarParagraphDropdown disabled={alignmentDisabled} kind="align" />
    <ToolbarParagraphDropdown disabled={lineHeightDisabled} kind="lineHeight" />
    <ToolbarParagraphDropdown disabled={textFormattingDisabled} kind="letterSpacing" />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={group}>
    <span class={css({ display: 'contents' })} data-floating-keep-open>
      <ToolbarButton
        active={isLinkActive}
        disabled={textFormattingDisabled || isLinkMixed || (isCollapsed && !isLinkActive)}
        icon={LinkIcon}
        label="링크"
        onclick={() => openMarkCard('link')}
      />

      <ToolbarButton
        active={isRubyActive}
        disabled={textFormattingDisabled || isRubyMixed || (isCollapsed && !isRubyActive)}
        icon={RubyIcon}
        label="루비"
        onclick={() => openMarkCard('ruby')}
      />
    </span>

    <ToolbarButton
      disabled={isCollapsed}
      icon={MessageSquarePlusIcon}
      label="코멘트"
      onclick={() => ctx.editor?.requestCommentCompose?.()}
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <ToolbarButton
    disabled={textFormattingDisabled}
    icon={RemoveFormattingIcon}
    keys={['Mod', '\\']}
    label="서식 지우기"
    onclick={() => enqueue({ type: 'modifier', op: { type: 'clear_all' } })}
  />
</div>
