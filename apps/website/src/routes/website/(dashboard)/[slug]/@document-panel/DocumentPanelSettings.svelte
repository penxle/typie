<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import {
    DropdownMenu,
    DropdownMenuItem,
    HorizontalDivider,
    Icon,
    SearchableDropdown,
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
  import { FontSpecimen } from '$lib/components';
  import { ToolbarColorGrid } from '$lib/components/editor/toolbar';
  import { getRepresentativeFont } from '$lib/editor/fonts';
  import { THEME_COLORS } from '$lib/editor/theme';
  import { createPaginatedLayout, getMaxMargin, mmToPx, pxToMm } from '$lib/editor/utils';
  import { values } from '$lib/editor/values';
  import type { Editor } from '$lib/editor/editor.svelte';
  import type { ThemeVariant } from '$lib/editor/theme';
  import type { PageLayoutPreset } from '$lib/editor/utils';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  const app = getAppContext();
  const theme = getThemeContext();

  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );
  const tc = $derived(THEME_COLORS[themeVariant]);

  const defaultAttrs = $derived(editor.defaultAttrs);

  const currentFontFamilyFonts = $derived.by(() => {
    const docFont = editor.fontFamilies.find((f) => f.familyName === defaultAttrs?.fontFamily);
    if (!docFont) return [];
    return [...new Map(docFont.fonts.filter((f) => f.state === 'ACTIVE').map((f) => [f.weight, f])).values()].toSorted(
      (a, b) => a.weight - b.weight,
    );
  });

  const fontWeightItems = $derived(
    currentFontFamilyFonts.map((font) => ({
      value: font.weight,
      label:
        values.fontWeight.find((f) => f.value === font.weight)?.label ??
        (font.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${font.weight})` : String(font.weight)),
    })),
  );

  const representativeFontMap = $derived(new Map(editor.fontFamilies.map((f) => [f.familyName, getRepresentativeFont(f.fonts)])));

  const weightFontIdMap = $derived(new Map(currentFontFamilyFonts.map((f) => [f.weight, f.id])));

  const getClosestWeight = (familyName: string, targetWeight: number) => {
    const docFont = editor.fontFamilies.find((f) => f.familyName === familyName);
    if (!docFont) return targetWeight;

    const weights = [...new Set(docFont.fonts.filter((f) => f.state === 'ACTIVE').map((f) => f.weight))].toSorted((a, b) => a - b);
    if (weights.length === 0) return targetWeight;
    if (weights.includes(targetWeight)) return targetWeight;

    let closest = weights[0];
    let minDiff = Math.abs(targetWeight - weights[0]);
    for (const w of weights) {
      const diff = Math.abs(targetWeight - w);
      if (diff <= minDiff) {
        minDiff = diff;
        closest = w;
      }
    }
    return closest;
  };

  const handleDefaultAttrChange = (updates: Partial<NonNullable<typeof editor.defaultAttrs>>) => {
    const current = editor.defaultAttrs;
    if (!current) return;

    const merged = {
      fontFamily: updates.fontFamily ?? current.fontFamily,
      fontSize: updates.fontSize ?? current.fontSize,
      fontWeight: updates.fontWeight ?? current.fontWeight,
      textColor: updates.textColor ?? current.textColor,
      backgroundColor: updates.backgroundColor ?? current.backgroundColor,
      letterSpacing: updates.letterSpacing ?? current.letterSpacing,
      lineHeight: updates.lineHeight ?? current.lineHeight,
    };

    editor.dispatch({
      type: 'setDefaultAttrs',
      attrs: [
        { attr: 'style', type: 'font_family', family: merged.fontFamily },
        { attr: 'style', type: 'font_size', size: merged.fontSize },
        { attr: 'style', type: 'font_weight', weight: merged.fontWeight },
        { attr: 'style', type: 'text_color', color: merged.textColor },
        { attr: 'style', type: 'background_color', color: merged.backgroundColor },
        { attr: 'style', type: 'letter_spacing', spacing: merged.letterSpacing },
        { attr: 'paragraph', lineHeight: merged.lineHeight },
      ],
    });
  };

  const textColorItems = $derived(values.textColor.map((c) => ({ label: c.label, value: c.value, color: tc[c.themeKey] })));

  const bgColorItems = $derived(
    values.textBackgroundColor.map((c) => ({ label: c.label, value: c.value, color: c.themeKey ? tc[c.themeKey] : null })),
  );

  const bgItem = $derived(bgColorItems.find(({ value }) => value === defaultAttrs?.backgroundColor));

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

  $effect(() => {
    if (defaultAttrs && !fontSizeOpened && !fontSizeIsFocused) {
      fontSizeInputValue = String(defaultAttrs.fontSize / 100);
    }
  });

  const applyFontSize = () => {
    const parsed = Number.parseFloat(fontSizeInputValue);
    if (!Number.isNaN(parsed) && defaultAttrs && Math.round(parsed * 100) !== defaultAttrs.fontSize) {
      handleDefaultAttrChange({ fontSize: clamp(Math.round(parsed * 100), values.minFontSize, values.maxFontSize) });
    }
  };

  const handleFontSizeFocus = () => {
    fontSizeIsFocused = true;
    fontSizeOpened = true;
    if (defaultAttrs) {
      fontSizeInputValue = String(defaultAttrs.fontSize / 100);
    }
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
      if (defaultAttrs) {
        fontSizeInputValue = String(defaultAttrs.fontSize / 100);
      }
      fontSizeInputElement?.blur();
      fontSizeOpened = false;
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      const currentInput = Number.parseFloat(fontSizeInputValue);
      const current = (currentInput ? Math.round(currentInput * 100) : defaultAttrs?.fontSize) || 1200;
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
        handleDefaultAttrChange({ fontSize: newValue });
        tick().then(() => {
          fontSizeInputElement?.select();
        });
      }
    }
  };

  const layoutMode = $derived(editor.layout?.layoutMode);
  const selectedPagePreset = $derived.by(() => {
    if (layoutMode?.type !== 'paginated') return 'a4';
    return (
      values.pageLayout.find((p) => p.layout.pageWidth === layoutMode.pageWidth && p.layout.pageHeight === layoutMode.pageHeight)?.value ??
      'custom'
    );
  });

  const handleLayoutModeChange = (mode: 'continuous' | 'paginated') => {
    if (mode === 'paginated') {
      const layout = createPaginatedLayout('a4');
      editor.dispatch({
        type: 'setLayoutMode',
        mode: {
          type: 'paginated',
          ...layout,
        },
      });
    } else {
      editor.dispatch({
        type: 'setLayoutMode',
        mode: { type: 'continuous', maxWidth: 600 },
      });
    }
    mixpanel.track('toggle_document_layout_mode', { mode });
  };

  const handlePagePresetChange = (value: string) => {
    if (value === 'custom') return;
    const layout = createPaginatedLayout(value as PageLayoutPreset);
    if (layoutMode?.type === 'paginated') {
      editor.dispatch({
        type: 'setLayoutMode',
        mode: {
          ...layoutMode,
          pageWidth: layout.pageWidth,
          pageHeight: layout.pageHeight,
        },
      });
      mixpanel.track('change_document_page_size', { preset: value });
    }
  };

  const handleWidthChange = (e: Event) => {
    if (!layoutMode || layoutMode.type !== 'paginated') return;
    const target = e.target as HTMLInputElement;
    const value = Math.max(100, Number(target.value));
    target.value = String(value);

    editor.dispatch({
      type: 'setLayoutMode',
      mode: { ...layoutMode, pageWidth: mmToPx(value) },
    });
  };

  const handleHeightChange = (e: Event) => {
    if (!layoutMode || layoutMode.type !== 'paginated') return;
    const target = e.target as HTMLInputElement;
    const value = Math.max(100, Number(target.value));
    target.value = String(value);

    editor.dispatch({
      type: 'setLayoutMode',
      mode: { ...layoutMode, pageHeight: mmToPx(value) },
    });
  };

  const handleMarginChange = (side: 'top' | 'bottom' | 'left' | 'right', e: Event) => {
    if (!layoutMode || layoutMode.type !== 'paginated') return;
    const target = e.target as HTMLInputElement;
    const valuePx = clamp(mmToPx(Number(target.value)), 0, getMaxMargin(side, layoutMode));
    target.value = String(pxToMm(valuePx));
    editor.dispatch({
      type: 'setLayoutMode',
      mode: {
        ...layoutMode,
        [`pageMargin${side.charAt(0).toUpperCase() + side.slice(1)}`]: valuePx,
      },
    });
  };

  const handleMaxWidthChange = (value: number) => {
    editor.dispatch({
      type: 'setLayoutMode',
      mode: { type: 'continuous', maxWidth: value },
    });
    mixpanel.track('change_document_max_width', { maxWidth: value });
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
    {#if defaultAttrs}
      <div class={css({ paddingX: '20px', fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>기본 스타일</div>

      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
          <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>폰트 패밀리</div>
        </div>
        <SearchableDropdown
          style={css.raw({
            width: '140px',
            height: '28px',
            paddingX: '8px',
            '& > input': { textAlign: 'right', fontSize: '12px', fontWeight: 'medium' },
          })}
          getLabel={(value) => editor.fontFamilies.find((f) => f.familyName === value)?.displayName ?? '(알 수 없는 폰트)'}
          items={editor.fontFamilies.filter((f) => f.state === 'ACTIVE').map((f) => ({ value: f.familyName, label: f.displayName }))}
          label=""
          onchange={(value) => {
            const closestWeight = getClosestWeight(value, defaultAttrs.fontWeight);
            handleDefaultAttrChange({ fontFamily: value, fontWeight: closestWeight });
          }}
          value={defaultAttrs.fontFamily}
        >
          {#snippet renderItem(item)}
            {@const font = representativeFontMap.get(item.value)}
            <FontSpecimen fontId={font?.id} text={item.label} weight={font?.weight} />
          {/snippet}
        </SearchableDropdown>
      </div>

      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
          <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>폰트 굵기</div>
        </div>
        <SearchableDropdown
          style={css.raw({
            width: '140px',
            height: '28px',
            paddingX: '8px',
            '& > input': { textAlign: 'right', fontSize: '12px', fontWeight: 'medium' },
          })}
          getLabel={(value) => {
            const item = fontWeightItems.find((w) => w.value === value);
            return item?.label ?? '(알 수 없는 굵기)';
          }}
          items={fontWeightItems}
          label=""
          onchange={(value) => {
            handleDefaultAttrChange({ fontWeight: value });
          }}
          value={defaultAttrs.fontWeight}
        >
          {#snippet renderItem(item)}
            <FontSpecimen fontId={weightFontIdMap.get(item.value)} text={item.label} weight={item.value} />
          {/snippet}
        </SearchableDropdown>
      </div>

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
            paddingX: '8px',
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
              width: 'full',
              paddingRight: '16px',
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
            placeholder={String(defaultAttrs.fontSize / 100)}
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
                  active={defaultAttrs.fontSize === value}
                  onclick={() => {
                    handleDefaultAttrChange({ fontSize: value });
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
          <Icon style={css.raw({ color: 'text.faint' })} icon={LetterSpacingIcon} />
          <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>자간</div>
        </div>
        <Select
          items={values.letterSpacing.map((s) => ({ value: s.value, label: s.label }))}
          onselect={(value) => {
            handleDefaultAttrChange({ letterSpacing: value });
          }}
          value={defaultAttrs.letterSpacing}
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
            handleDefaultAttrChange({ lineHeight: value });
          }}
          value={defaultAttrs.lineHeight}
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
            style:background-color={textColorItems.find(({ value }) => value === defaultAttrs.textColor)?.color}
            class={css({ borderWidth: '1px', borderRadius: 'full', size: '16px', flexShrink: '0' })}
          ></div>
          <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
            {textColorItems.find(({ value }) => value === defaultAttrs.textColor)?.label ?? defaultAttrs.textColor}
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
              currentValue={defaultAttrs.textColor}
              items={textColorItems}
              onClose={() => (textColorOpened = false)}
              onSelect={(value) => {
                handleDefaultAttrChange({ textColor: value });
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
            {#if defaultAttrs.backgroundColor === 'none'}
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
            {bgItem?.label ?? defaultAttrs.backgroundColor}
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
              currentValue={defaultAttrs.backgroundColor}
              items={bgColorItems}
              onClose={() => (bgColorOpened = false)}
              onSelect={(value) => {
                handleDefaultAttrChange({ backgroundColor: value });
                bgColorOpened = false;
              }}
              opened={bgColorOpened}
              shape="square"
            />
          </div>
        {/if}
      </div>

      <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />
    {/if}

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
                  onchange={handleWidthChange}
                  size="sm"
                  type="number"
                  value={pxToMm(layoutMode.pageWidth)}
                />
              </div>
              <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>높이</div>
                <TextInput
                  style={css.raw({ width: '80px' })}
                  min="100"
                  onchange={handleHeightChange}
                  size="sm"
                  type="number"
                  value={pxToMm(layoutMode.pageHeight)}
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
                max={String(pxToMm(getMaxMargin('top', layoutMode)))}
                min="0"
                onchange={(e) => handleMarginChange('top', e)}
                size="sm"
                type="number"
                value={pxToMm(layoutMode.pageMarginTop)}
              />
            </div>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle' })}>하단</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                max={String(pxToMm(getMaxMargin('bottom', layoutMode)))}
                min="0"
                onchange={(e) => handleMarginChange('bottom', e)}
                size="sm"
                type="number"
                value={pxToMm(layoutMode.pageMarginBottom)}
              />
            </div>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle' })}>왼쪽</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                max={String(pxToMm(getMaxMargin('left', layoutMode)))}
                min="0"
                onchange={(e) => handleMarginChange('left', e)}
                size="sm"
                type="number"
                value={pxToMm(layoutMode.pageMarginLeft)}
              />
            </div>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle' })}>오른쪽</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                max={String(pxToMm(getMaxMargin('right', layoutMode)))}
                min="0"
                onchange={(e) => handleMarginChange('right', e)}
                size="sm"
                type="number"
                value={pxToMm(layoutMode.pageMarginRight)}
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
              onselect={handleMaxWidthChange}
              size="sm"
              value={layoutMode.type === 'continuous' ? layoutMode.maxWidth : 600}
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
            editor.dispatch({ type: 'setParagraphIndent', indent: value });
          }}
          size="sm"
          value={editor.settings.paragraphIndent}
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
            editor.dispatch({ type: 'setBlockGap', gap: value });
          }}
          size="sm"
          value={editor.settings.blockGap}
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
