<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { DropdownMenu, DropdownMenuItem, HorizontalDivider, Icon } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { nanoid } from 'nanoid';
  import { fly } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import PilcrowIcon from '~icons/lucide/pilcrow';
  import PlusIcon from '~icons/lucide/plus';
  import RefreshCwIcon from '~icons/lucide/refresh-cw';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { THEME_COLORS } from '$lib/editor-ffi/theme';
  import { values } from '$lib/editor-ffi/values';
  import CreateStyleModal from './CreateStyleModal.svelte';
  import DocumentPanelStyleSelectItem from './DocumentPanelStyleSelectItem.svelte';
  import type { Modifier } from '@typie/editor-ffi/browser';
  import type { ThemeVariant } from '$lib/editor-ffi/theme';

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
  const showUpdate = $derived(ctx.editor?.styleDivergence ?? false);

  const disabled = $derived(!ctx.editor || !ctx.editor.blockState);

  let opened = $state(false);
  let createStyleModalOpen = $state(false);
  let activeDeleteId = $state<string | null>(null);
  let triggerEl = $state<HTMLButtonElement>();
  let triggerWidth = $state(0);

  $effect(() => {
    if (!opened || !triggerEl) return;
    const update = () => {
      triggerWidth = triggerEl?.getBoundingClientRect().width ?? 0;
    };
    update();
    const ro = new ResizeObserver(update);
    ro.observe(triggerEl);
    return () => ro.disconnect();
  });

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom-start',
    offset: 4,
    onClickOutside: () => {
      opened = false;
    },
  });

  const previewStyle = (modifiers: Modifier[]): string => {
    const parts: string[] = [];
    const decorations: string[] = [];
    for (const mod of modifiers) {
      switch (mod.type) {
        case 'bold': {
          parts.push('font-weight: 700');
          break;
        }
        case 'italic': {
          parts.push('font-style: italic');
          break;
        }
        case 'underline': {
          decorations.push('underline');
          break;
        }
        case 'strikethrough': {
          decorations.push('line-through');
          break;
        }
        case 'font_size': {
          parts.push(`font-size: ${Math.min(mod.value / 100, 18)}px`);
          break;
        }
        case 'font_weight': {
          parts.push(`font-weight: ${mod.value}`);
          break;
        }
        case 'font_family': {
          parts.push(`font-family: ${mod.value}`);
          break;
        }
        case 'text_color': {
          const color = textColorMap.get(mod.value) ?? mod.value;
          parts.push(`color: ${color}`);
          break;
        }
        case 'background_color': {
          const color = bgColorMap.get(mod.value);
          if (color) parts.push(`background-color: ${color}`);
          break;
        }
      }
    }
    if (decorations.length > 0) parts.push(`text-decoration: ${decorations.join(' ')}`);
    return parts.join('; ');
  };

  const apply = (styleId: string) => {
    const editor = ctx.editor;
    if (!editor) return;
    editor.enqueue({ type: 'style', op: { type: 'apply_to_selection', style_id: styleId } });
    opened = false;
    editor.focus();
  };

  const unapplyCurrent = () => {
    const editor = ctx.editor;
    if (!editor) return;
    editor.enqueue({ type: 'style', op: { type: 'unset_in_selection' } });
    opened = false;
    editor.focus();
  };

  const createStyle = (name: string) => {
    const editor = ctx.editor;
    if (!editor) return;
    const styleId = nanoid();
    editor.enqueue({ type: 'style', op: { type: 'create_from_selection', style_id: styleId, name } });
    editor.focus();
  };

  const deleteStyle = (styleId: string) => {
    const editor = ctx.editor;
    if (!editor) return;
    editor.enqueue({ type: 'style', op: { type: 'delete', style_id: styleId } });
  };

  const updateCurrentStyle = () => {
    const editor = ctx.editor;
    if (!editor) return;
    editor.enqueue({ type: 'style', op: { type: 'update_from_selection' } });
    editor.focus();
  };

  const triggerLabel = $derived.by(() => {
    if (currentStyle) return currentStyle.name;
    if (appliedStyle?.type === 'mixed') return '여러 스타일';
    if (appliedStyle?.type === 'uniform') return '(알 수 없는 스타일)';
    return '기본';
  });
</script>

<div class={flex({ flexDirection: 'column', gap: '6px', paddingX: '20px' })}>
  <div class={flex({ alignItems: 'center', gap: '6px' })}>
    <button
      bind:this={triggerEl}
      class={flex({
        alignItems: 'center',
        gap: '8px',
        flexGrow: '1',
        borderWidth: '1px',
        borderColor: opened ? 'border.strong' : 'border.subtle',
        borderRadius: '6px',
        paddingX: '10px',
        height: '32px',
        backgroundColor: 'surface.default',
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? '50' : '100',
        _hover: disabled ? {} : { backgroundColor: 'surface.muted' },
      })}
      {disabled}
      onclick={() => (opened = !opened)}
      type="button"
      use:anchor
    >
      <Icon style={css.raw({ color: 'text.faint', flexShrink: '0' })} icon={PilcrowIcon} size={14} />
      <span class={css({ flexGrow: '1', textAlign: 'left', fontSize: '13px', color: 'text.default', truncate: true })}>
        {triggerLabel}
      </span>
      <Icon
        style={css.raw({
          color: 'text.faint',
          flexShrink: '0',
          transform: opened ? 'rotate(-180deg)' : 'rotate(0deg)',
          transitionDuration: '150ms',
        })}
        icon={ChevronDownIcon}
        size={16}
      />
    </button>
    {#if showUpdate}
      <button
        class={flex({
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: '0',
          borderWidth: '1px',
          borderColor: 'border.subtle',
          borderRadius: '6px',
          width: '32px',
          height: '32px',
          backgroundColor: 'surface.default',
          color: 'text.brand',
          cursor: 'pointer',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        aria-label="현재 스타일에 인라인 변경 업데이트"
        onclick={updateCurrentStyle}
        title="스타일에 변경 사항 반영"
        type="button"
      >
        <Icon icon={RefreshCwIcon} size={14} />
      </button>
    {/if}
  </div>
  {#if opened}
    <div
      style:width="{triggerWidth}px"
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
        {#each styleEntries as entry (entry.id)}
          {@const isActive = entry.id === currentStyleId}
          <DocumentPanelStyleSelectItem
            {entry}
            {isActive}
            onapply={() => apply(entry.id)}
            ondelete={() => deleteStyle(entry.id)}
            oniconhover={() => (activeDeleteId = entry.id)}
            onrowhover={() => (activeDeleteId = null)}
            preview={previewStyle(entry.modifiers)}
            showDelete={activeDeleteId === entry.id}
          />
        {/each}
        <HorizontalDivider color="secondary" />
        <div class={flex({ flexDirection: 'column' })} onmouseenter={() => (activeDeleteId = null)} role="presentation">
          <DropdownMenuItem
            onclick={() => {
              opened = false;
              createStyleModalOpen = true;
            }}
          >
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'text.faint' })} icon={PlusIcon} size={14} />
              <span class={css({ color: 'text.subtle' })}>선택 영역에서 새 스타일 만들기</span>
            </div>
          </DropdownMenuItem>
        </div>
      </DropdownMenu>
    </div>
  {/if}
</div>

<CreateStyleModal onCreate={createStyle} bind:open={createStyleModalOpen} />
