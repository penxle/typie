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
  import { getAppContext } from '@typie/ui/context';
  import { values } from '@typie/ui/tiptap';
  import { clamp, PAGE_LAYOUT_OPTIONS, PAGE_SIZE_MAP } from '@typie/ui/utils';
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
  import { fragment, graphql } from '$graphql';
  import ToolbarColorGrid from '../@toolbar/ToolbarColorGrid.svelte';
  import type { DocumentPanel_Settings_user } from '$graphql';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    editor: Editor;
    $user: DocumentPanel_Settings_user;
  };

  let { editor, $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DocumentPanel_Settings_user on User {
        id

        fontFamilies {
          id
          name

          fonts {
            id
            weight
          }
        }
      }
    `),
  );

  const app = getAppContext();

  const defaultStyles = $derived(editor.defaultStyles);

  const fontFamilyItems = $derived([
    ...values.fontFamily.map((f) => ({ value: f.value, label: f.label })),
    ...$user.fontFamilies.map((f) => ({ value: f.id, label: f.name })),
  ]);

  const currentFontFamilyWeights = $derived.by(() => {
    const ff = defaultStyles?.fontFamily;
    if (!ff) return values.fontFamily[0].weights.toSorted((a, b) => a - b);

    const systemFont = values.fontFamily.find((f) => f.value === ff);
    if (systemFont) return systemFont.weights.toSorted((a, b) => a - b);

    const userFont = $user.fontFamilies.find((f) => f.id === ff);
    if (userFont) return userFont.fonts.map((f) => f.weight).toSorted((a, b) => a - b);

    return values.fontFamily[0].weights.toSorted((a, b) => a - b);
  });

  const fontWeightItems = $derived(
    currentFontFamilyWeights.map((weight) => ({
      value: weight,
      label: values.fontWeight.find(({ value }) => value === weight)?.label || String(weight),
    })),
  );

  const getClosestWeight = (fontFamilyOrId: string, targetWeight: number) => {
    let weights: number[];

    const systemFont = values.fontFamily.find((f) => f.value === fontFamilyOrId);
    if (systemFont) {
      weights = systemFont.weights.toSorted((a, b) => a - b);
    } else {
      const userFont = $user.fontFamilies.find((f) => f.id === fontFamilyOrId);
      if (!userFont) return targetWeight;
      weights = userFont.fonts.map((f) => f.weight).toSorted((a, b) => a - b);
    }

    if (weights.length === 0) return targetWeight;
    if (weights.includes(targetWeight)) return targetWeight;

    let closest = weights[0];
    let minDiff = Math.abs(targetWeight - weights[0]);
    for (const w of weights) {
      const diff = Math.abs(targetWeight - w);
      if (diff < minDiff) {
        minDiff = diff;
        closest = w;
      }
    }
    return closest;
  };

  const handleDefaultStyleChange = (updates: Partial<NonNullable<typeof editor.defaultStyles>>) => {
    const current = editor.defaultStyles;
    if (!current) return;
    editor.dispatch({
      type: 'setDefaultStyles',
      styles: {
        fontFamily: updates.fontFamily ?? current.fontFamily,
        fontSize: updates.fontSize ?? current.fontSize,
        fontWeight: updates.fontWeight ?? current.fontWeight,
        textColor: updates.textColor ?? current.textColor,
        backgroundColor: updates.backgroundColor ?? current.backgroundColor,
        letterSpacing: updates.letterSpacing ?? current.letterSpacing,
        lineHeight: updates.lineHeight ?? current.lineHeight,
        italic: false,
        strikethrough: false,
        underline: false,
      },
    });
  };

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

  const MIN_FONT_SIZE = 1;
  const MAX_FONT_SIZE = 200;

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
    if (defaultStyles && !fontSizeOpened && !fontSizeIsFocused) {
      fontSizeInputValue = String(defaultStyles.fontSize);
    }
  });

  const applyFontSize = () => {
    const parsed = Number.parseFloat(fontSizeInputValue);
    if (!Number.isNaN(parsed) && defaultStyles && parsed !== defaultStyles.fontSize) {
      handleDefaultStyleChange({ fontSize: clamp(parsed, MIN_FONT_SIZE, MAX_FONT_SIZE) });
    }
  };

  const handleFontSizeFocus = () => {
    fontSizeIsFocused = true;
    fontSizeOpened = true;
    if (defaultStyles) {
      fontSizeInputValue = String(defaultStyles.fontSize);
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
      if (defaultStyles) {
        fontSizeInputValue = String(defaultStyles.fontSize);
      }
      fontSizeInputElement?.blur();
      fontSizeOpened = false;
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      const current = Number.parseFloat(fontSizeInputValue) || defaultStyles?.fontSize || 12;
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
        fontSizeInputValue = String(newValue);
        handleDefaultStyleChange({ fontSize: newValue });
        tick().then(() => {
          fontSizeInputElement?.select();
        });
      }
    }
  };

  const layoutMode = $derived(editor.layout.layoutMode);
  const isPaginated = $derived(layoutMode.type === 'paginated');

  type PageSizePreset = keyof typeof PAGE_SIZE_MAP | 'custom';

  const mmToPx = (mm: number) => Math.round((mm * 96) / 25.4);
  const pxToMm = (px: number) => Math.round((px * 25.4) / 96);

  const selectedPagePreset = $derived.by(() => {
    if (layoutMode.type !== 'paginated') return 'a4';
    const widthMm = pxToMm(layoutMode.pageWidth);
    const heightMm = pxToMm(layoutMode.pageHeight);
    const found = Object.entries(PAGE_SIZE_MAP).find(([, size]) => size.width === widthMm && size.height === heightMm);
    return (found?.[0] as PageSizePreset) ?? 'custom';
  });

  const currentWidthMm = $derived(layoutMode.type === 'paginated' ? pxToMm(layoutMode.pageWidth) : 210);
  const currentHeightMm = $derived(layoutMode.type === 'paginated' ? pxToMm(layoutMode.pageHeight) : 297);
  const currentMarginTopMm = $derived(layoutMode.type === 'paginated' ? pxToMm(layoutMode.pageMarginTop) : 25);
  const currentMarginBottomMm = $derived(layoutMode.type === 'paginated' ? pxToMm(layoutMode.pageMarginBottom) : 25);
  const currentMarginLeftMm = $derived(layoutMode.type === 'paginated' ? pxToMm(layoutMode.pageMarginLeft) : 25);
  const currentMarginRightMm = $derived(layoutMode.type === 'paginated' ? pxToMm(layoutMode.pageMarginRight) : 25);

  const getMaxMargin = (dimension: 'width' | 'height') => {
    const size = dimension === 'width' ? currentWidthMm : currentHeightMm;
    return Math.floor(size / 2) - 10;
  };

  const handleLayoutModeChange = (mode: 'continuous' | 'paginated') => {
    if (mode === 'paginated') {
      const preset = PAGE_SIZE_MAP.a4;
      editor.dispatch({
        type: 'setLayoutMode',
        mode: {
          type: 'paginated',
          pageWidth: mmToPx(preset.width),
          pageHeight: mmToPx(preset.height),
          pageMarginTop: mmToPx(25),
          pageMarginBottom: mmToPx(25),
          pageMarginLeft: mmToPx(25),
          pageMarginRight: mmToPx(25),
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
    const preset = PAGE_SIZE_MAP[value as keyof typeof PAGE_SIZE_MAP];
    if (preset && layoutMode.type === 'paginated') {
      editor.dispatch({
        type: 'setLayoutMode',
        mode: {
          type: 'paginated',
          pageWidth: mmToPx(preset.width),
          pageHeight: mmToPx(preset.height),
          pageMarginTop: layoutMode.pageMarginTop,
          pageMarginBottom: layoutMode.pageMarginBottom,
          pageMarginLeft: layoutMode.pageMarginLeft,
          pageMarginRight: layoutMode.pageMarginRight,
        },
      });
      mixpanel.track('change_document_page_size', { preset: value });
    }
  };

  const handleWidthChange = (e: Event) => {
    if (layoutMode.type !== 'paginated') return;
    const target = e.target as HTMLInputElement;
    const value = Math.max(100, Number(target.value));
    target.value = String(value);
    editor.dispatch({
      type: 'setLayoutMode',
      mode: {
        type: 'paginated',
        pageWidth: mmToPx(value),
        pageHeight: layoutMode.pageHeight,
        pageMarginTop: layoutMode.pageMarginTop,
        pageMarginBottom: layoutMode.pageMarginBottom,
        pageMarginLeft: layoutMode.pageMarginLeft,
        pageMarginRight: layoutMode.pageMarginRight,
      },
    });
  };

  const handleHeightChange = (e: Event) => {
    if (layoutMode.type !== 'paginated') return;
    const target = e.target as HTMLInputElement;
    const value = Math.max(100, Number(target.value));
    target.value = String(value);
    editor.dispatch({
      type: 'setLayoutMode',
      mode: {
        type: 'paginated',
        pageWidth: layoutMode.pageWidth,
        pageHeight: mmToPx(value),
        pageMarginTop: layoutMode.pageMarginTop,
        pageMarginBottom: layoutMode.pageMarginBottom,
        pageMarginLeft: layoutMode.pageMarginLeft,
        pageMarginRight: layoutMode.pageMarginRight,
      },
    });
  };

  const handleMarginChange = (side: 'top' | 'bottom' | 'left' | 'right', e: Event) => {
    if (layoutMode.type !== 'paginated') return;
    const target = e.target as HTMLInputElement;
    const maxMargin = side === 'top' || side === 'bottom' ? getMaxMargin('height') : getMaxMargin('width');
    const value = clamp(Number(target.value), 0, maxMargin);
    target.value = String(value);
    editor.dispatch({
      type: 'setLayoutMode',
      mode: {
        type: 'paginated',
        pageWidth: layoutMode.pageWidth,
        pageHeight: layoutMode.pageHeight,
        pageMarginTop: side === 'top' ? mmToPx(value) : layoutMode.pageMarginTop,
        pageMarginBottom: side === 'bottom' ? mmToPx(value) : layoutMode.pageMarginBottom,
        pageMarginLeft: side === 'left' ? mmToPx(value) : layoutMode.pageMarginLeft,
        pageMarginRight: side === 'right' ? mmToPx(value) : layoutMode.pageMarginRight,
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
    {#if defaultStyles}
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
          getLabel={(value) => {
            const item = fontFamilyItems.find((f) => f.value === value);
            return item?.label ?? '(알 수 없는 폰트)';
          }}
          items={fontFamilyItems}
          label=""
          onchange={(value) => {
            const closestWeight = getClosestWeight(value, defaultStyles.fontWeight);
            handleDefaultStyleChange({ fontFamily: value, fontWeight: closestWeight });
          }}
          value={defaultStyles.fontFamily}
        >
          {#snippet renderItem(item)}
            <div style:font-family={item.value}>{item.label}</div>
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
            handleDefaultStyleChange({ fontWeight: value });
          }}
          value={defaultStyles.fontWeight}
        >
          {#snippet renderItem(item)}
            <div style:font-family={defaultStyles.fontFamily} style:font-weight={item.value}>
              {item.label}
            </div>
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
            placeholder={String(defaultStyles.fontSize)}
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
                  active={defaultStyles.fontSize === value}
                  onclick={() => {
                    handleDefaultStyleChange({ fontSize: value });
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
            handleDefaultStyleChange({ letterSpacing: value });
          }}
          value={defaultStyles.letterSpacing}
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
            handleDefaultStyleChange({ lineHeight: value });
          }}
          value={defaultStyles.lineHeight}
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
            style:background-color={values.textColor.find(({ value }) => value === defaultStyles.textColor)?.color}
            class={css({ borderWidth: '1px', borderRadius: 'full', size: '16px', flexShrink: '0' })}
          ></div>
          <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
            {values.textColor.find(({ value }) => value === defaultStyles.textColor)?.label ?? defaultStyles.textColor}
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
              currentValue={defaultStyles.textColor as (typeof values.textColor)[number]['value']}
              items={values.textColor}
              onClose={() => (textColorOpened = false)}
              onSelect={(value) => {
                handleDefaultStyleChange({ textColor: value });
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
            style:background-color={defaultStyles.backgroundColor === 'none'
              ? 'transparent'
              : values.textBackgroundColor.find(({ value }) => value === defaultStyles.backgroundColor)?.color}
            class={css({ borderWidth: '1px', borderRadius: '4px', size: '16px', flexShrink: '0', position: 'relative' })}
          >
            {#if defaultStyles.backgroundColor === 'none'}
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
            {values.textBackgroundColor.find(({ value }) => value === defaultStyles.backgroundColor)?.label ??
              defaultStyles.backgroundColor}
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
              currentValue={defaultStyles.backgroundColor as (typeof values.textBackgroundColor)[number]['value']}
              items={values.textBackgroundColor}
              onClose={() => (bgColorOpened = false)}
              onSelect={(value) => {
                handleDefaultStyleChange({ backgroundColor: value });
                bgColorOpened = false;
              }}
              opened={bgColorOpened}
              shape="square"
              showNone
            />
          </div>
        {/if}
      </div>

      <HorizontalDivider style={css.raw({ marginY: '12px' })} color="secondary" />
    {/if}

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

    {#if isPaginated}
      <div class={flex({ flexDirection: 'column', gap: '6px', paddingX: '20px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} />
          <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>페이지 크기 (mm)</div>
        </div>
        <Select items={PAGE_LAYOUT_OPTIONS} onselect={handlePagePresetChange} value={selectedPagePreset} />
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
                value={currentWidthMm}
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
                value={currentHeightMm}
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
              max={String(getMaxMargin('height'))}
              min="0"
              onchange={(e) => handleMarginChange('top', e)}
              size="sm"
              type="number"
              value={currentMarginTopMm}
            />
          </div>
          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <div class={css({ fontSize: '12px', color: 'text.subtle' })}>하단</div>
            <TextInput
              style={css.raw({ width: '80px' })}
              max={String(getMaxMargin('height'))}
              min="0"
              onchange={(e) => handleMarginChange('bottom', e)}
              size="sm"
              type="number"
              value={currentMarginBottomMm}
            />
          </div>
          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <div class={css({ fontSize: '12px', color: 'text.subtle' })}>왼쪽</div>
            <TextInput
              style={css.raw({ width: '80px' })}
              max={String(getMaxMargin('width'))}
              min="0"
              onchange={(e) => handleMarginChange('left', e)}
              size="sm"
              type="number"
              value={currentMarginLeftMm}
            />
          </div>
          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <div class={css({ fontSize: '12px', color: 'text.subtle' })}>오른쪽</div>
            <TextInput
              style={css.raw({ width: '80px' })}
              max={String(getMaxMargin('width'))}
              min="0"
              onchange={(e) => handleMarginChange('right', e)}
              size="sm"
              type="number"
              value={currentMarginRightMm}
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
            { label: '0.5칸', value: 0.5 },
            { label: '1칸', value: 1 },
            { label: '2칸', value: 2 },
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
            { label: '0.5줄', value: 0.5 },
            { label: '1줄', value: 1 },
            { label: '2줄', value: 2 },
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
