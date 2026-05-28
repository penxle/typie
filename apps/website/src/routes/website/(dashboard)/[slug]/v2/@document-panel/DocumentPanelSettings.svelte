<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import {
    DropdownMenu,
    DropdownMenuItem,
    HorizontalDivider,
    Icon,
    SegmentButtons,
    Select,
    Slider,
    Switch,
    TextInput,
    Tooltip,
  } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import AlignVerticalSpaceAroundIcon from '~icons/lucide/align-vertical-space-around';
  import ArrowRightToLineIcon from '~icons/lucide/arrow-right-to-line';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import FileIcon from '~icons/lucide/file';
  import FileTextIcon from '~icons/lucide/file-text';
  import HighlighterIcon from '~icons/lucide/highlighter';
  import InfoIcon from '~icons/lucide/info';
  import PaletteIcon from '~icons/lucide/palette';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import TypeIcon from '~icons/lucide/type';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import { ToolbarColorGrid } from '$lib/editor-ffi/components/toolbar';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { defaultContinuousLayout, defaultPaginatedLayout, setRootLayoutMode, setRootModifier } from '$lib/editor-ffi/root-attrs';
  import { THEME_COLORS } from '$lib/editor-ffi/theme';
  import { values } from '$lib/editor-ffi/values';
  import type { LayoutMode, Modifier, ModifierType } from '@typie/editor-ffi/browser';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';
  import type { ThemeVariant } from '$lib/editor-ffi/theme';

  type Props = {
    editor: Editor | undefined;
  };

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  let { editor: _editor }: Props = $props();

  const app = getAppContext();
  const theme = getThemeContext();
  const ctx = getEditorContext();

  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );
  const tc = $derived(THEME_COLORS[themeVariant]);

  const mod = <T extends ModifierType>(type: T) =>
    ctx.editor?.rootModifiers?.find((m): m is Extract<Modifier, { type: T }> => m.type === type);

  const layoutMode = $derived(ctx.editor?.rootAttrs?.layout_mode);

  const setMod = (modifier: Modifier) => {
    setRootModifier(ctx.editor, modifier);
  };

  const setLayout = (layout_mode: LayoutMode) => {
    setRootLayoutMode(ctx.editor, layout_mode);
  };

  const patchPaginated = (patch: Partial<Extract<LayoutMode, { type: 'paginated' }>>) => {
    if (layoutMode?.type !== 'paginated') return;
    setLayout({ ...layoutMode, ...patch });
  };

  const textColorItems = $derived(values.textColor.map((c) => ({ label: c.label, value: c.value, color: tc[c.themeKey] })));

  const bgColorItems = $derived(
    values.textBackgroundColor.map((c) => ({ label: c.label, value: c.value, color: c.themeKey ? tc[c.themeKey] : null })),
  );

  const currentTextColor = $derived(mod('text_color')?.value ?? 'default');
  const currentBgColor = $derived(mod('background_color')?.value ?? 'none');
  const bgItem = $derived(bgColorItems.find(({ value }) => value === currentBgColor));

  let textColorOpened = $state(false);
  let bgColorOpened = $state(false);
  let fontSizeOpened = $state(false);

  const { anchor: textColorAnchorAction, floating: textColorFloatingAction } = createFloatingActions({
    placement: 'bottom-start',
    offset: 8,
    onClickOutside: () => {
      textColorOpened = false;
    },
  });

  const { anchor: bgColorAnchorAction, floating: bgColorFloatingAction } = createFloatingActions({
    placement: 'bottom-start',
    offset: 8,
    onClickOutside: () => {
      bgColorOpened = false;
    },
  });

  let fontSizeAnchorElement: HTMLDivElement | undefined = $state();
  let fontSizeFloatingElement: HTMLDivElement | undefined = $state();
  let fontSizeInputElement: HTMLInputElement | undefined = $state();
  let fontSizeInputValue = $state('');
  let fontSizeIsFocused = $state(false);

  const { anchor: fontSizeAnchorAction, floating: fontSizeFloatingAction } = createFloatingActions({
    placement: 'bottom-end',
    offset: 8,
    onClickOutside: (event) => {
      if (fontSizeAnchorElement?.contains(event.target as Node)) {
        return;
      }
      fontSizeOpened = false;
    },
  });

  const currentFontSize = $derived(mod('font_size')?.value ?? 1200);

  $effect(() => {
    if (!fontSizeOpened && !fontSizeIsFocused) {
      fontSizeInputValue = String(currentFontSize / 100);
    }
  });

  const applyFontSize = () => {
    const parsed = Number.parseFloat(fontSizeInputValue);
    if (!Number.isNaN(parsed) && Math.round(parsed * 100) !== currentFontSize) {
      setMod({ type: 'font_size', value: clamp(Math.round(parsed * 100), values.minFontSize, values.maxFontSize) });
    }
  };

  const handleFontSizeFocus = () => {
    fontSizeIsFocused = true;
    fontSizeOpened = true;
    fontSizeInputValue = String(currentFontSize / 100);
    fontSizeInputElement?.select();
  };

  const handleFontSizeBlur = (e: FocusEvent) => {
    fontSizeIsFocused = false;
    const relatedTarget = e.relatedTarget as Node | null;
    if (relatedTarget && fontSizeFloatingElement?.contains(relatedTarget)) {
      return;
    }
    applyFontSize();
    fontSizeOpened = false;
  };

  const handleFontSizeKeydown = (e: KeyboardEvent) => {
    if (e.isComposing) return;

    if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      applyFontSize();
      fontSizeInputElement?.blur();
      fontSizeOpened = false;
    } else if (e.key === 'Escape') {
      fontSizeInputValue = String(currentFontSize / 100);
      fontSizeInputElement?.blur();
      fontSizeOpened = false;
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      const currentInput = Number.parseFloat(fontSizeInputValue);
      const current = (currentInput ? Math.round(currentInput * 100) : currentFontSize) || 1200;
      const sortedSizes = values.fontSize.map(({ value }) => value).toSorted((a, b) => a - b);
      const currentIndex = sortedSizes.findIndex((size) => size >= current);

      let newIndex: number;
      if (e.key === 'ArrowDown') {
        newIndex = currentIndex === -1 || currentIndex >= sortedSizes.length - 1 ? 0 : currentIndex + 1;
      } else {
        newIndex = currentIndex === -1 || currentIndex <= 0 ? sortedSizes.length - 1 : currentIndex - 1;
      }

      const newValue = sortedSizes[newIndex];
      if (newValue !== undefined) {
        fontSizeInputValue = String(newValue / 100);
        setMod({ type: 'font_size', value: newValue });
        tick().then(() => {
          fontSizeInputElement?.select();
        });
      }
    }
  };

  const selectedPagePreset = $derived.by(() => {
    if (layoutMode?.type !== 'paginated') return 'a4';
    return (
      values.pageLayout.find((p) => p.layout.pageWidth === layoutMode.page_width && p.layout.pageHeight === layoutMode.page_height)
        ?.value ?? 'custom'
    );
  });

  const handleLayoutModeChange = (mode: 'continuous' | 'paginated') => {
    if (mode === 'paginated') {
      setLayout(defaultPaginatedLayout());
    } else {
      setLayout(defaultContinuousLayout());
    }
    mixpanel.track('toggle_document_layout_mode', { mode });
  };

  const handlePagePresetChange = (value: string) => {
    if (value === 'custom') return;
    const preset = values.pageLayout.find((p) => p.value === value);
    if (preset && layoutMode?.type === 'paginated') {
      setLayout({
        ...layoutMode,
        type: 'paginated',
        page_width: preset.layout.pageWidth,
        page_height: preset.layout.pageHeight,
        page_margin_top: preset.layout.pageMarginTop,
        page_margin_bottom: preset.layout.pageMarginBottom,
        page_margin_left: preset.layout.pageMarginLeft,
        page_margin_right: preset.layout.pageMarginRight,
      });
      mixpanel.track('change_document_page_size', { preset: value });
    }
  };
</script>

<div
  class={flex({
    flexDirection: 'column',
    minWidth: 'var(--min-width)',
    width: 'var(--width)',
    maxWidth: 'var(--max-width)',
    height: 'full',
  })}
>
  <div
    class={flex({
      flexShrink: '0',
      height: '41px',
      alignItems: 'center',
      paddingX: '20px',
      fontSize: '13px',
      fontWeight: 'semibold',
      color: 'text.subtle',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    본문 설정
  </div>

  <div class={flex({ flexDirection: 'column', gap: '16px', overflowY: 'auto', paddingY: '16px' })}>
    <div class={css({ paddingX: '20px', fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>기본 스타일</div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>폰트 크기</div>
      </div>
      <div
        bind:this={fontSizeAnchorElement}
        class={css({
          position: 'relative',
          display: 'flex',
          alignItems: 'center',
          borderRadius: '4px',
          width: '140px',
          height: '28px',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        use:fontSizeAnchorAction
      >
        <input
          bind:this={fontSizeInputElement}
          class={css({
            flexGrow: '1',
            size: 'full',
            paddingLeft: '8px',
            paddingRight: '24px',
            fontSize: '12px',
            fontWeight: 'medium',
            color: 'text.subtle',
            textAlign: 'right',
            backgroundColor: 'transparent',
            border: 'none',
            outline: 'none',
          })}
          onblur={handleFontSizeBlur}
          onfocus={handleFontSizeFocus}
          onkeydown={handleFontSizeKeydown}
          placeholder={String(currentFontSize / 100)}
          type="text"
          bind:value={fontSizeInputValue}
        />
        <button
          class={css({ pointerEvents: fontSizeOpened ? 'auto' : 'none', cursor: 'pointer' })}
          onclick={() => {
            applyFontSize();
            fontSizeInputElement?.blur();
            fontSizeOpened = false;
          }}
          type="button"
        >
          <Icon
            style={css.raw({
              position: 'absolute',
              right: '4px',
              top: '1/2',
              translate: 'auto',
              translateY: '-1/2',
              color: 'text.faint',
              transform: fontSizeOpened ? 'rotate(-180deg)' : 'rotate(0deg)',
              transitionDuration: '150ms',
            })}
            icon={ChevronDownIcon}
            size={16}
          />
        </button>
      </div>
      {#if fontSizeOpened}
        <div
          bind:this={fontSizeFloatingElement}
          class={css({
            borderWidth: '1px',
            borderColor: 'border.subtle',
            borderBottomRadius: '4px',
            backgroundColor: 'surface.default',
            zIndex: 'menu',
            boxShadow: 'small',
            overflow: 'hidden',
          })}
          use:fontSizeFloatingAction
          in:fly={{ y: -5, duration: 150 }}
        >
          <DropdownMenu autoFocus={false} onclose={() => (fontSizeOpened = false)} opened={fontSizeOpened}>
            {#each values.fontSize as { label, value } (value)}
              <DropdownMenuItem
                active={currentFontSize === value}
                onclick={() => {
                  setMod({ type: 'font_size', value });
                  fontSizeOpened = false;
                }}
              >
                {label}
              </DropdownMenuItem>
            {/each}
          </DropdownMenu>
        </div>
      {/if}
    </div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>폰트 굵기</div>
      </div>
      <Select
        items={values.fontWeight.map((s) => ({ value: s.value, label: s.label }))}
        onselect={(value) => {
          setMod({ type: 'font_weight', value });
        }}
        value={mod('font_weight')?.value ?? 400}
      />
    </div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={LetterSpacingIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>자간</div>
      </div>
      <Select
        items={values.letterSpacing.map((s) => ({ value: s.value, label: s.label }))}
        onselect={(value) => {
          setMod({ type: 'letter_spacing', value });
        }}
        value={mod('letter_spacing')?.value ?? 0}
      />
    </div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={LineHeightIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>행간</div>
      </div>
      <Select
        items={values.lineHeight.map((s) => ({ value: s.value, label: s.label }))}
        onselect={(value) => {
          setMod({ type: 'line_height', value });
        }}
        value={mod('line_height')?.value ?? 160}
      />
    </div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={PaletteIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>글자 색</div>
      </div>
      <button
        class={flex({
          alignItems: 'center',
          gap: '8px',
          borderRadius: '6px',
          paddingX: '8px',
          paddingY: '4px',
          height: '28px',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        onclick={() => (textColorOpened = !textColorOpened)}
        type="button"
        use:textColorAnchorAction
      >
        <div
          style:background-color={textColorItems.find(({ value }) => value === currentTextColor)?.color}
          class={css({ borderWidth: '1px', borderRadius: 'full', size: '16px', flexShrink: '0' })}
        ></div>
        <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
          {textColorItems.find(({ value }) => value === currentTextColor)?.label ?? currentTextColor}
        </span>
      </button>
      {#if textColorOpened}
        <div
          class={css({
            borderWidth: '1px',
            borderColor: 'border.subtle',
            borderRadius: '4px',
            backgroundColor: 'surface.default',
            zIndex: 'overEditor',
            boxShadow: 'small',
            overflow: 'hidden',
          })}
          use:textColorFloatingAction
          in:fly={{ y: -5, duration: 150 }}
        >
          <ToolbarColorGrid
            columns={11}
            currentValue={currentTextColor}
            items={textColorItems}
            onClose={() => (textColorOpened = false)}
            onSelect={(value) => {
              setMod({ type: 'text_color', value });
              textColorOpened = false;
            }}
            opened={textColorOpened}
          />
        </div>
      {/if}
    </div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={PaletteIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>배경 색</div>
      </div>
      <button
        class={flex({
          alignItems: 'center',
          gap: '8px',
          borderRadius: '6px',
          paddingX: '8px',
          paddingY: '4px',
          height: '28px',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        onclick={() => (bgColorOpened = !bgColorOpened)}
        type="button"
        use:bgColorAnchorAction
      >
        <div
          style:background-color={bgItem?.color ?? 'transparent'}
          class={css({ borderWidth: '1px', borderRadius: '4px', size: '16px', flexShrink: '0', position: 'relative' })}
        >
          {#if currentBgColor === 'none'}
            <div
              class={css({
                position: 'absolute',
                inset: '0',
                margin: 'auto',
                width: '1px',
                height: '12px',
                backgroundColor: 'text.disabled',
                transform: 'rotate(45deg)',
              })}
            ></div>
          {/if}
        </div>
        <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
          {bgItem?.label ?? currentBgColor}
        </span>
      </button>
      {#if bgColorOpened}
        <div
          class={css({
            borderWidth: '1px',
            borderColor: 'border.subtle',
            borderRadius: '4px',
            backgroundColor: 'surface.default',
            zIndex: 'overEditor',
            boxShadow: 'small',
            overflow: 'hidden',
          })}
          use:bgColorFloatingAction
          in:fly={{ y: -5, duration: 150 }}
        >
          <ToolbarColorGrid
            columns={8}
            currentValue={currentBgColor}
            items={bgColorItems}
            onClose={() => (bgColorOpened = false)}
            onSelect={(value) => {
              setMod({ type: 'background_color', value });
              bgColorOpened = false;
            }}
            opened={bgColorOpened}
            shape="square"
          />
        </div>
      {/if}
    </div>

    <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />

    {#if layoutMode}
      <div class={css({ paddingX: '20px', fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>레이아웃</div>

      <div class={flex({ flexDirection: 'column', gap: '6px', paddingX: '20px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={FileTextIcon} />
          <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>레이아웃 모드</div>
        </div>
        <div class={css({ width: '140px' })}>
          <SegmentButtons
            items={[
              { label: '스크롤', value: 'continuous' },
              { label: '페이지', value: 'paginated' },
            ]}
            onselect={handleLayoutModeChange}
            size="sm"
            value={layoutMode.type}
          />
        </div>
      </div>

      {#if layoutMode?.type === 'paginated'}
        <div class={flex({ flexDirection: 'column', gap: '6px', paddingX: '20px' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} />
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>페이지 크기 (mm)</div>
          </div>
          <Select
            items={[...values.pageLayout, { label: '직접 지정', value: 'custom' }]}
            onselect={handlePagePresetChange}
            value={selectedPagePreset}
          />
          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <div class={grid({ columns: 2, columnGap: '12px', rowGap: '8px', paddingLeft: '8px' })}>
              <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>너비</div>
                <TextInput
                  style={css.raw({ width: '80px' })}
                  min="100"
                  onchange={(e) => {
                    const value = Math.max(100, Number((e.target as HTMLInputElement).value));
                    (e.target as HTMLInputElement).value = String(value);
                    patchPaginated({ page_width: value });
                  }}
                  size="sm"
                  type="number"
                  value={layoutMode.page_width}
                />
              </div>
              <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>높이</div>
                <TextInput
                  style={css.raw({ width: '80px' })}
                  min="100"
                  onchange={(e) => {
                    const value = Math.max(100, Number((e.target as HTMLInputElement).value));
                    (e.target as HTMLInputElement).value = String(value);
                    patchPaginated({ page_height: value });
                  }}
                  size="sm"
                  type="number"
                  value={layoutMode.page_height}
                />
              </div>
            </div>
          </div>
        </div>

        <div class={flex({ flexDirection: 'column', gap: '6px', paddingX: '20px' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={RulerDimensionLineIcon} />
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>여백 (mm)</div>
          </div>
          <div class={grid({ columns: 2, columnGap: '12px', rowGap: '8px', paddingLeft: '8px' })}>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle' })}>상단</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                min="0"
                onchange={(e) => patchPaginated({ page_margin_top: Math.max(0, Number((e.target as HTMLInputElement).value)) })}
                size="sm"
                type="number"
                value={layoutMode.page_margin_top}
              />
            </div>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle' })}>하단</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                min="0"
                onchange={(e) => patchPaginated({ page_margin_bottom: Math.max(0, Number((e.target as HTMLInputElement).value)) })}
                size="sm"
                type="number"
                value={layoutMode.page_margin_bottom}
              />
            </div>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle' })}>왼쪽</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                min="0"
                onchange={(e) => patchPaginated({ page_margin_left: Math.max(0, Number((e.target as HTMLInputElement).value)) })}
                size="sm"
                type="number"
                value={layoutMode.page_margin_left}
              />
            </div>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle' })}>오른쪽</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                min="0"
                onchange={(e) => patchPaginated({ page_margin_right: Math.max(0, Number((e.target as HTMLInputElement).value)) })}
                size="sm"
                type="number"
                value={layoutMode.page_margin_right}
              />
            </div>
          </div>
        </div>
      {:else}
        <div class={flex({ flexDirection: 'column', gap: '8px', paddingX: '20px' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={RulerDimensionLineIcon} />
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>본문 폭</div>
          </div>
          <div class={css({ width: '200px' })}>
            <SegmentButtons
              items={[
                { label: '400px', value: 400 },
                { label: '600px', value: 600 },
                { label: '800px', value: 800 },
              ]}
              onselect={(value) => {
                setLayout({ type: 'continuous', max_width: value });
                mixpanel.track('change_document_max_width', { maxWidth: value });
              }}
              size="sm"
              value={layoutMode.type === 'continuous' ? layoutMode.max_width : 600}
            />
          </div>
        </div>
      {/if}

      <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />
    {/if}

    <div class={css({ paddingX: '20px', fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>세부 레이아웃</div>

    <div class={flex({ flexDirection: 'column', gap: '8px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={ArrowRightToLineIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>첫 줄 들여쓰기</div>
      </div>
      <div class={css({ width: '200px' })}>
        <SegmentButtons
          items={[
            { label: '없음', value: 0 },
            { label: '0.5칸', value: 50 },
            { label: '1칸', value: 100 },
            { label: '2칸', value: 200 },
          ]}
          onselect={(value) => {
            setMod({ type: 'paragraph_indent', value });
          }}
          size="sm"
          value={mod('paragraph_indent')?.value ?? 0}
        />
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '8px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={AlignVerticalSpaceAroundIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>문단 사이 간격</div>
      </div>
      <div class={css({ width: '200px' })}>
        <SegmentButtons
          items={[
            { label: '없음', value: 0 },
            { label: '0.5줄', value: 50 },
            { label: '1줄', value: 100 },
            { label: '2줄', value: 200 },
          ]}
          onselect={(value) => {
            setMod({ type: 'block_gap', value });
          }}
          size="sm"
          value={mod('block_gap')?.value ?? 0}
        />
      </div>
    </div>

    <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />

    <div class={css({ paddingX: '20px', fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>편집 경험</div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>타자기 모드</div>
        <Tooltip message="현재 작성 중인 줄을 항상 화면의 특정 위치에 고정합니다." placement="top">
          <Icon style={css.raw({ color: 'text.faint' })} icon={InfoIcon} />
        </Tooltip>
      </div>
      <Switch
        onchange={() => {
          mixpanel.track('toggle_typewriter', {
            enabled: app.preference.current.typewriterEnabled,
          });
        }}
        bind:checked={app.preference.current.typewriterEnabled}
      />
    </div>

    {#if app.preference.current.typewriterEnabled}
      <div class={flex({ width: 'full', align: 'center', gap: '16px', paddingX: '20px' })}>
        <div class={css({ flexShrink: '0', fontSize: '11px', color: 'text.muted' })}>상단</div>
        <Slider
          max={1}
          min={0}
          onchange={() => {
            mixpanel.track('change_typewriter_position', {
              position: Math.round(app.preference.current.typewriterPosition * 100),
            });
          }}
          step={0.05}
          tooltipFormatter={(v) => `${Math.round(v * 100)}% 위치에 고정`}
          bind:value={app.preference.current.typewriterPosition}
        />
        <div class={css({ flexShrink: '0', fontSize: '11px', color: 'text.muted' })}>하단</div>
      </div>
    {/if}

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={HighlighterIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>현재 줄 강조</div>
      </div>
      <Switch
        onchange={() => {
          mixpanel.track('toggle_line_highlight', {
            enabled: app.preference.current.lineHighlightEnabled,
          });
        }}
        bind:checked={app.preference.current.lineHighlightEnabled}
      />
    </div>
  </div>
</div>
