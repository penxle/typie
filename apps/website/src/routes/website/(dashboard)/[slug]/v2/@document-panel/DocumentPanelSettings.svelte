<script lang="ts">
  import { createFragment } from '@mearie/svelte';
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
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import TypeIcon from '~icons/lucide/type';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import { FontSpecimen } from '$lib/components';
  import { familySpecimenFallbacks } from '$lib/components/font-specimen';
  import { getMaxMargin, getPageMargin, MIN_PAGE_SIZE_MM, pxToMm, resizePageUnit } from '$lib/editor/utils';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { defaultContinuousLayout, defaultPaginatedLayout, setRootLayoutMode, setRootModifier } from '$lib/editor-ffi/root-attrs';
  import { values } from '$lib/editor-ffi/values';
  import { activeFontsByWeight, fontWeightItemsForFonts, resolveFontWeightForFamily } from '$lib/font-weight';
  import { graphql } from '$mearie';
  import type { LayoutMode, Modifier, ModifierType } from '@typie/editor-ffi/browser';
  import type { PageLayout, PageMarginSide } from '$lib/editor/utils';
  import type { DocumentPanelV2_Settings_document$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelV2_Settings_document$key;
  };

  let { document$key }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DocumentPanelV2_Settings_document on Document {
        id

        fontFamilies {
          id
          familyName
          displayName
          state

          fonts {
            id
            weight
            subfamilyDisplayName
            state
          }
        }
      }
    `),
    () => document$key,
  );

  const app = getAppContext();
  const ctx = getEditorContext();

  const mod = <T extends ModifierType>(type: T) =>
    ctx.editor?.rootModifiers?.find((m): m is Extract<Modifier, { type: T }> => m.type === type);

  const layoutMode = $derived(ctx.editor?.rootAttrs?.layout_mode);

  const setMod = (modifier: Modifier) => {
    setRootModifier(ctx.editor, modifier);
  };

  const fontFamilies = $derived(document.data.fontFamilies);

  const currentFontFamily = $derived(mod('font_family')?.value ?? 'Pretendard');
  const currentFontWeight = $derived(mod('font_weight')?.value ?? 400);

  const activeFamilies = $derived(fontFamilies.filter((f) => f.state === 'ACTIVE'));

  const fontFamilyItems = $derived.by(() => {
    if (activeFamilies.every((f) => f.familyName !== currentFontFamily)) {
      const current = fontFamilies.find((f) => f.familyName === currentFontFamily);
      if (current) {
        return [...activeFamilies, current];
      }
    }
    return activeFamilies;
  });

  const representativeFontMap = $derived(
    new Map(
      fontFamilies.map((family) => {
        const active = family.fonts.filter((f) => f.state === 'ACTIVE');
        const representative =
          active.length === 0
            ? null
            : active.reduce((prev, curr) => {
                const prevDiff = Math.abs(prev.weight - 400);
                const currDiff = Math.abs(curr.weight - 400);
                if (currDiff < prevDiff) return curr;
                if (currDiff === prevDiff && curr.weight > prev.weight) return curr;
                return prev;
              });
        return [family.familyName, representative] as const;
      }),
    ),
  );

  const currentFamilyFonts = $derived.by(() => {
    const family = fontFamilies.find((f) => f.familyName === currentFontFamily);
    if (!family) return [];
    return activeFontsByWeight(family.fonts);
  });

  const fontWeightItems = $derived.by(() => {
    const items = fontWeightItemsForFonts(currentFamilyFonts, values.fontWeight);
    if (items.some((item) => item.value === currentFontWeight)) {
      return items;
    }
    const label = values.fontWeight.find((l) => l.value === currentFontWeight)?.label ?? String(currentFontWeight);
    return [...items, { value: currentFontWeight, label }].toSorted((a, b) => a.value - b.value);
  });

  const handleFontFamilyChange = (familyName: string) => {
    if (familyName === currentFontFamily) {
      return;
    }
    setMod({ type: 'font_family', value: familyName });
    const resolvedWeight = resolveFontWeightForFamily(fontFamilies, familyName, currentFontWeight);
    if (resolvedWeight !== currentFontWeight) {
      setMod({ type: 'font_weight', value: resolvedWeight });
    }
  };

  const setLayout = (layout_mode: LayoutMode) => {
    setRootLayoutMode(ctx.editor, layout_mode);
  };

  let fontSizeAnchorElement: HTMLDivElement | undefined = $state();
  let fontSizeFloatingElement: HTMLDivElement | undefined = $state();
  let fontSizeInputElement: HTMLInputElement | undefined = $state();
  let fontSizeInputValue = $state('');
  let fontSizeIsFocused = $state(false);
  let fontSizeOpened = $state(false);

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
        newIndex = (currentIndex === -1 || currentIndex <= 0 ? sortedSizes.length : currentIndex) - 1;
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

  const fromLayoutMode = (mode: Extract<LayoutMode, { type: 'paginated' }>): PageLayout => ({
    pageWidth: mode.page_width,
    pageHeight: mode.page_height,
    pageMarginTop: mode.page_margin_top,
    pageMarginBottom: mode.page_margin_bottom,
    pageMarginLeft: mode.page_margin_left,
    pageMarginRight: mode.page_margin_right,
  });

  const toLayoutMode = (
    mode: Extract<LayoutMode, { type: 'paginated' }>,
    layout: PageLayout,
  ): Extract<LayoutMode, { type: 'paginated' }> => ({
    ...mode,
    page_width: layout.pageWidth,
    page_height: layout.pageHeight,
    page_margin_top: layout.pageMarginTop,
    page_margin_bottom: layout.pageMarginBottom,
    page_margin_left: layout.pageMarginLeft,
    page_margin_right: layout.pageMarginRight,
  });

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
      setLayout(toLayoutMode(layoutMode, preset.layout));
      mixpanel.track('change_document_page_size', { preset: value });
    }
  };

  const handleWidthChange = (e: Event) => {
    if (layoutMode?.type !== 'paginated') return;
    const target = e.target as HTMLInputElement;
    const value = Math.max(MIN_PAGE_SIZE_MM, Number(target.value));
    target.value = String(value);

    const nextLayout = resizePageUnit(fromLayoutMode(layoutMode), 'width', value);
    setLayout(toLayoutMode(layoutMode, nextLayout));
  };

  const handleHeightChange = (e: Event) => {
    if (layoutMode?.type !== 'paginated') return;
    const target = e.target as HTMLInputElement;
    const value = Math.max(MIN_PAGE_SIZE_MM, Number(target.value));
    target.value = String(value);

    const nextLayout = resizePageUnit(fromLayoutMode(layoutMode), 'height', value);
    setLayout(toLayoutMode(layoutMode, nextLayout));
  };

  const handleMarginChange = (side: PageMarginSide, e: Event) => {
    if (layoutMode?.type !== 'paginated') return;
    const target = e.target as HTMLInputElement;
    const nextLayout = resizePageUnit(fromLayoutMode(layoutMode), side, Number(target.value));
    target.value = String(pxToMm(getPageMargin(side, nextLayout)));

    setLayout(toLayoutMode(layoutMode, nextLayout));
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
    <div class={css({ paddingX: '20px', fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>기본 서식</div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>글꼴</div>
      </div>
      <SearchableDropdown
        style={css.raw({ width: '140px' })}
        getLabel={(value) => fontFamilies.find((f) => f.familyName === value)?.displayName ?? '(알 수 없는 폰트)'}
        inputStyle={css.raw({ fontSize: '12px', fontWeight: 'medium', textAlign: 'right' })}
        items={fontFamilyItems.map((f) => ({ value: f.familyName, label: f.displayName }))}
        label="글꼴"
        onchange={(familyName) => handleFontFamilyChange(familyName)}
        placeholder="-"
        value={currentFontFamily}
      >
        {#snippet renderItem(item)}
          {@const font = representativeFontMap.get(item.value)}
          <FontSpecimen
            fallbacks={familySpecimenFallbacks(item.label, item.value)}
            fontId={font?.id ?? undefined}
            text={item.label}
            weight={font?.weight}
          />
        {/snippet}
      </SearchableDropdown>
    </div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>글자 굵기</div>
      </div>
      <Select
        items={fontWeightItems}
        onselect={(value) => {
          if (value !== currentFontWeight) {
            setMod({ type: 'font_weight', value });
          }
        }}
        value={currentFontWeight}
      />
    </div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px', paddingX: '20px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px', flexShrink: '0' })}>
        <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>글자 크기</div>
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
                  if (value !== currentFontSize) {
                    setMod({ type: 'font_size', value });
                  }
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
          if (value !== (mod('letter_spacing')?.value ?? 0)) {
            setMod({ type: 'letter_spacing', value });
          }
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
          if (value !== (mod('line_height')?.value ?? 160)) {
            setMod({ type: 'line_height', value });
          }
        }}
        value={mod('line_height')?.value ?? 160}
      />
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
                  onchange={handleWidthChange}
                  size="sm"
                  type="number"
                  value={pxToMm(layoutMode.page_width)}
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
                  value={pxToMm(layoutMode.page_height)}
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
                max={String(pxToMm(getMaxMargin('top', fromLayoutMode(layoutMode))))}
                min="0"
                onchange={(e) => handleMarginChange('top', e)}
                size="sm"
                type="number"
                value={pxToMm(layoutMode.page_margin_top)}
              />
            </div>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle' })}>하단</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                max={String(pxToMm(getMaxMargin('bottom', fromLayoutMode(layoutMode))))}
                min="0"
                onchange={(e) => handleMarginChange('bottom', e)}
                size="sm"
                type="number"
                value={pxToMm(layoutMode.page_margin_bottom)}
              />
            </div>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle' })}>왼쪽</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                max={String(pxToMm(getMaxMargin('left', fromLayoutMode(layoutMode))))}
                min="0"
                onchange={(e) => handleMarginChange('left', e)}
                size="sm"
                type="number"
                value={pxToMm(layoutMode.page_margin_left)}
              />
            </div>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle' })}>오른쪽</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                max={String(pxToMm(getMaxMargin('right', fromLayoutMode(layoutMode))))}
                min="0"
                onchange={(e) => handleMarginChange('right', e)}
                size="sm"
                type="number"
                value={pxToMm(layoutMode.page_margin_right)}
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
