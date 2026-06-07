<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { DropdownMenu, DropdownMenuItem, HorizontalDivider, Icon } from '@typie/ui/components';
  import { disassemble } from 'es-hangul';
  import { nanoid } from 'nanoid';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import PlusIcon from '~icons/lucide/plus';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import ToolbarStyleFormModal from './ToolbarStyleFormModal.svelte';
  import ToolbarStyleSelectItem from './ToolbarStyleSelectItem.svelte';
  import type { Modifier } from '@typie/editor-ffi/browser';

  type Font = { id?: string | null; weight: number; subfamilyDisplayName?: string | null; state: string };
  type FontFamily = { id: string; familyName: string; displayName: string; state: string; fonts: readonly Font[] };

  type Props = {
    fontFamilies?: readonly FontFamily[];
  };

  let { fontFamilies = [] }: Props = $props();

  const ctx = getEditorContext();

  const styleEntries = $derived(
    (ctx.editor?.styleEntries ?? []).toSorted((a, b) => {
      if (a.id === 'base') return -1;
      if (b.id === 'base') return 1;
      return a.name.localeCompare(b.name);
    }),
  );
  const appliedStyle = $derived(ctx.editor?.appliedStyle);
  const currentStyleId = $derived(appliedStyle?.type === 'uniform' ? appliedStyle.value.value : undefined);
  const currentStyle = $derived(styleEntries.find((s) => s.id === currentStyleId));
  const styleDivergence = $derived(ctx.editor?.styleDivergence ?? false);

  const disabled = $derived(!ctx.editor || !ctx.editor.blockState);

  let opened = $state(false);
  let createStyleModalOpen = $state(false);
  let editStyleModalOpen = $state(false);
  let editingEntry = $state<(typeof styleEntries)[number] | undefined>();
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
    if (ms.letter_spacing.type === 'uniform') m.push({ type: 'letter_spacing', value: ms.letter_spacing.value.value });
    if (ms.text_color.type === 'uniform') m.push({ type: 'text_color', value: ms.text_color.value.value });
    if (ms.background_color.type === 'uniform') m.push({ type: 'background_color', value: ms.background_color.value.value });
    return m;
  });

  const startEdit = (entry: (typeof styleEntries)[number]) => {
    editingEntry = entry;
    editStyleModalOpen = true;
    opened = false;
    inputElement?.blur();
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
      const first = filteredEntries[0];
      if (first) apply(first.id);
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
        {#each filteredEntries as entry (entry.id)}
          {@const isActive = entry.id === currentStyleId}
          <ToolbarStyleSelectItem {entry} {isActive} onapply={() => apply(entry.id)} onedit={() => startEdit(entry)} />
        {/each}
        {#if filteredEntries.length > 0}
          <HorizontalDivider color="secondary" />
        {/if}
        <div class={flex({ flexDirection: 'column' })}>
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
  onDelete={() => {
    if (editingEntry) deleteStyle(editingEntry.id);
  }}
  onSubmit={updateStyle}
  styleId={editingEntry?.id}
  bind:open={editStyleModalOpen}
/>
