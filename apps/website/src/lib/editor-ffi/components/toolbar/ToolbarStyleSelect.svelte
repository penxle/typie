<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { DropdownMenu, DropdownMenuItem, HorizontalDivider, Icon } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { disassemble } from 'es-hangul';
  import { nanoid } from 'nanoid';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import PlusIcon from '~icons/lucide/plus';
  import RemoveFormattingIcon from '~icons/lucide/remove-formatting';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { THEME_COLORS } from '$lib/editor-ffi/theme';
  import { values } from '$lib/editor-ffi/values';
  import { modifiersToCss } from './modifier-css';
  import ToolbarStyleFormModal from './ToolbarStyleFormModal.svelte';
  import ToolbarStyleSelectItem from './ToolbarStyleSelectItem.svelte';
  import type { Modifier } from '@typie/editor-ffi/browser';
  import type { ThemeVariant } from '$lib/editor-ffi/theme';

  type Font = { id?: string | null; weight: number; subfamilyDisplayName?: string | null; state: string };
  type FontFamily = { id: string; familyName: string; displayName: string; state: string; fonts: readonly Font[] };

  type Props = {
    fontFamilies?: readonly FontFamily[];
  };

  let { fontFamilies = [] }: Props = $props();

  const ctx = getEditorContext();
  const theme = getThemeContext();

  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );
  const tc = $derived(THEME_COLORS[themeVariant]);
  const textColorMap = $derived(new Map<string, string>(values.textColor.map((c) => [c.value, tc[c.themeKey]])));
  const bgColorMap = $derived(
    new Map<string, string | null>(values.textBackgroundColor.map((c) => [c.value, c.themeKey ? tc[c.themeKey] : null])),
  );

  const styleEntries = $derived(ctx.editor?.styleEntries ?? []);
  const appliedStyle = $derived(ctx.editor?.appliedStyle);
  const currentStyleId = $derived(appliedStyle?.type === 'uniform' ? appliedStyle.value.value : undefined);
  const isDefaultActive = $derived(appliedStyle?.type === 'absent');
  const currentStyle = $derived(styleEntries.find((s) => s.id === currentStyleId));
  const styleDivergence = $derived(ctx.editor?.styleDivergence ?? false);

  const disabled = $derived(!ctx.editor || !ctx.editor.blockState);

  let opened = $state(false);
  let createStyleModalOpen = $state(false);
  let editStyleModalOpen = $state(false);
  let editingEntry = $state<(typeof styleEntries)[number] | undefined>();
  let activeDeleteId = $state<string | null>(null);
  let triggerEl = $state<HTMLDivElement>();
  let floatingEl = $state<HTMLDivElement>();
  let inputElement = $state<HTMLInputElement>();
  let inputValue = $state('');
  let isFocused = $state(false);

  const triggerLabel = $derived.by(() => {
    if (currentStyle) return styleDivergence ? `${currentStyle.name} *` : currentStyle.name;
    if (appliedStyle?.type === 'mixed') return '-';
    if (appliedStyle?.type === 'uniform') return '(알 수 없는 스타일)';
    return '기본';
  });

  $effect(() => {
    if (!isFocused && !opened) {
      inputValue = triggerLabel;
    }
  });

  const filteredEntries = $derived.by(() => {
    const query = disassemble(inputValue.toLowerCase().trim());
    if (!query) return styleEntries;
    return styleEntries.filter((s) => disassemble(s.name.toLowerCase()).includes(query));
  });

  const showDefault = $derived.by(() => {
    const query = disassemble(inputValue.toLowerCase().trim());
    if (!query) return true;
    return disassemble('기본').includes(query);
  });

  const previewStyle = (modifiers: Modifier[]) => modifiersToCss(modifiers, { textColorMap, bgColorMap, maxFontSize: 18 });

  const inputPreviewStyle = $derived(!isFocused && currentStyle ? previewStyle(currentStyle.modifiers) : '');

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom-start',
    offset: 4,
    onClickOutside: (event) => {
      if (triggerEl?.contains(event.target as Node)) return;
      opened = false;
    },
  });

  const apply = (styleId: string) => {
    const editor = ctx.editor;
    if (!editor) return;
    editor.enqueue({ type: 'style', op: { type: 'apply_to_selection', style_id: styleId } });
    opened = false;
    inputElement?.blur();
    editor.focus();
  };

  const unapplyCurrent = () => {
    const editor = ctx.editor;
    if (!editor) return;
    editor.enqueue({ type: 'style', op: { type: 'unset_in_selection' } });
    opened = false;
    inputElement?.blur();
    editor.focus();
  };

  const createStyle = (name: string, modifiers: Modifier[]) => {
    const editor = ctx.editor;
    if (!editor) return;
    const styleId = nanoid();
    editor.enqueue({ type: 'style', op: { type: 'define', style_id: styleId, name, modifiers } });
    editor.focus();
  };

  const selectionModifiers = $derived.by((): Modifier[] => {
    const ms = ctx.editor?.modifierState;
    if (!ms) return [];
    const m: Modifier[] = [];
    if (ms.font_size.type === 'uniform') m.push({ type: 'font_size', value: ms.font_size.value.value });
    if (ms.font_family.type === 'uniform') m.push({ type: 'font_family', value: ms.font_family.value.value });
    if (ms.font_weight.type === 'uniform') m.push({ type: 'font_weight', value: ms.font_weight.value.value });
    if (ms.text_color.type === 'uniform') m.push({ type: 'text_color', value: ms.text_color.value.value });
    if (ms.background_color.type === 'uniform') m.push({ type: 'background_color', value: ms.background_color.value.value });
    return m;
  });

  const startEdit = (entry: (typeof styleEntries)[number]) => {
    editingEntry = entry;
    editStyleModalOpen = true;
    opened = false;
    inputElement?.blur();
    activeDeleteId = null;
  };

  const updateStyle = (name: string, modifiers: Modifier[]) => {
    const editor = ctx.editor;
    if (!editor || !editingEntry) return;
    editor.enqueue({ type: 'style', op: { type: 'define', style_id: editingEntry.id, name, modifiers } });
    editor.focus();
  };

  const deleteStyle = (styleId: string) => {
    const editor = ctx.editor;
    if (!editor) return;
    editor.enqueue({ type: 'style', op: { type: 'delete', style_id: styleId } });
  };

  const handleFocus = () => {
    isFocused = true;
    opened = true;
    inputValue = '';
    tick().then(() => inputElement?.select());
  };

  const handleBlur = (e: FocusEvent) => {
    isFocused = false;
    const relatedTarget = e.relatedTarget as Node | null;
    if (relatedTarget instanceof Element) {
      if (floatingEl?.contains(relatedTarget)) return;
      if (relatedTarget.closest('[data-floating-keep-open]')) return;
    }
    opened = false;
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.isComposing) return;
    if (e.key === 'Escape') {
      inputValue = triggerLabel;
      inputElement?.blur();
      opened = false;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      const query = inputValue.trim();
      if (!query) {
        inputElement?.blur();
        opened = false;
        return;
      }
      if (showDefault) {
        unapplyCurrent();
      } else {
        const first = filteredEntries[0];
        if (first) apply(first.id);
      }
    }
  };
</script>

<div class={flex({ flexDirection: 'column', gap: '6px' })}>
  <div class={flex({ alignItems: 'center', gap: '6px' })}>
    <div
      bind:this={triggerEl}
      class={css({
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        width: '140px',
        paddingX: '10px',
        borderColor: opened ? 'border.strong' : 'border.subtle',
        borderRadius: '6px',
        height: '24px',
        backgroundColor: 'surface.default',
        cursor: disabled ? 'not-allowed' : 'text',
        opacity: disabled ? '50' : '100',
        _hover: disabled ? {} : { backgroundColor: 'surface.muted' },
        _focusWithin: { backgroundColor: 'surface.muted' },
      })}
      use:anchor
    >
      <input
        bind:this={inputElement}
        style={inputPreviewStyle}
        class={css({
          minWidth: '0',
          height: 'full',
          fontSize: '13px',
          color: 'text.default',
          backgroundColor: 'transparent',
          marginRight: '10px',
          border: 'none',
          outline: 'none',
          textOverflow: 'ellipsis',
          cursor: disabled ? 'not-allowed' : 'text',
          _placeholder: { color: 'text.default' },
        })}
        {disabled}
        onblur={handleBlur}
        onfocus={handleFocus}
        oninput={(e) => (inputValue = (e.currentTarget as HTMLInputElement).value)}
        onkeydown={handleKeydown}
        placeholder={triggerLabel}
        type="text"
        value={inputValue}
      />
      <button
        class={css({ pointerEvents: opened ? 'auto' : 'none', cursor: disabled ? 'not-allowed' : 'pointer' })}
        onclick={() => {
          inputElement?.blur();
          opened = false;
        }}
        type="button"
      >
        <Icon
          style={css.raw({
            position: 'absolute',
            right: '10px',
            top: '1/2',
            translate: 'auto',
            translateY: '-1/2',
            color: 'text.faint',
            transform: opened ? 'rotate(-180deg)' : 'rotate(0deg)',
            transitionDuration: '150ms',
          })}
          icon={ChevronDownIcon}
          size={16}
        />
      </button>
    </div>
  </div>
  {#if opened}
    <div
      bind:this={floatingEl}
      class={css({
        borderWidth: '1px',
        borderColor: 'border.subtle',
        borderRadius: '6px',
        backgroundColor: 'surface.default',
        zIndex: 'menu',
        boxShadow: 'small',
        overflow: 'hidden',
      })}
      use:floating
      in:fly={{ y: -5, duration: 150 }}
    >
      <DropdownMenu autoFocus={false} onclose={() => (opened = false)} {opened}>
        {#if showDefault}
          <button
            class={css({
              display: 'flex',
              alignItems: 'center',
              paddingX: '16px',
              paddingY: '8px',
              textAlign: 'left',
              fontSize: '13px',
              color: isDefaultActive ? 'text.brand' : 'text.subtle',
              backgroundColor: isDefaultActive ? 'surface.subtle' : 'transparent',
              cursor: 'pointer',
              _hover: { color: 'text.brand', backgroundColor: 'surface.subtle' },
              _focus: { color: 'text.brand', backgroundColor: 'surface.subtle' },
            })}
            data-active={isDefaultActive}
            onclick={unapplyCurrent}
            onmouseenter={() => (activeDeleteId = null)}
            type="button"
          >
            기본
          </button>
        {/if}
        {#each filteredEntries as entry (entry.id)}
          {@const isActive = entry.id === currentStyleId}
          <ToolbarStyleSelectItem
            {entry}
            {isActive}
            onapply={() => apply(entry.id)}
            ondelete={() => deleteStyle(entry.id)}
            onedit={() => startEdit(entry)}
            oniconhover={() => (activeDeleteId = entry.id)}
            onrowhover={() => (activeDeleteId = null)}
            preview={previewStyle(entry.modifiers)}
            showDelete={activeDeleteId === entry.id}
          />
        {/each}
        {#if showDefault || filteredEntries.length > 0}
          <HorizontalDivider color="secondary" />
        {/if}
        <div class={flex({ flexDirection: 'column' })} onmouseenter={() => (activeDeleteId = null)} role="presentation">
          <DropdownMenuItem
            onclick={() => {
              opened = false;
              inputElement?.blur();
              createStyleModalOpen = true;
            }}
          >
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'text.faint' })} icon={PlusIcon} size={14} />
              <span class={css({ color: 'text.subtle' })}>새 스타일 만들기</span>
            </div>
          </DropdownMenuItem>
          {#if !isDefaultActive}
            <DropdownMenuItem onclick={unapplyCurrent}>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ color: 'text.faint' })} icon={RemoveFormattingIcon} size={14} />
                <span class={css({ color: 'text.subtle' })}>스타일 해제</span>
              </div>
            </DropdownMenuItem>
          {/if}
        </div>
      </DropdownMenu>
    </div>
  {/if}
</div>

<ToolbarStyleFormModal
  {fontFamilies}
  initialModifiers={selectionModifiers}
  mode="create"
  onSubmit={createStyle}
  bind:open={createStyleModalOpen}
/>

<ToolbarStyleFormModal
  {fontFamilies}
  initialModifiers={editingEntry?.modifiers ?? []}
  initialName={editingEntry?.name ?? ''}
  mode="edit"
  onSubmit={updateStyle}
  bind:open={editStyleModalOpen}
/>
