<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Button, Icon, Modal, SearchableDropdown, TextInput } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import { tick, untrack } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { fly } from 'svelte/transition';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import PaletteIcon from '~icons/lucide/palette';
  import Trash2Icon from '~icons/lucide/trash-2';
  import TypeIcon from '~icons/lucide/type';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import { FontSpecimen } from '$lib/components';
  import { familySpecimenFallbacks, weightSpecimenFallbacks } from '$lib/components/font-specimen';
  import { THEME_COLORS } from '$lib/editor-ffi/theme';
  import { values } from '$lib/editor-ffi/values';
  import ToolbarColorGrid from './ToolbarColorGrid.svelte';
  import type { Modifier } from '@typie/editor-ffi/browser';
  import type { ThemeVariant } from '$lib/editor-ffi/theme';

  type Font = { id?: string | null; weight: number; subfamilyDisplayName?: string | null; state: string };
  type FontFamily = { id: string; familyName: string; displayName: string; state: string; fonts: readonly Font[] };

  type Props = {
    open: boolean;
    mode?: 'create' | 'edit';
    styleId?: string;
    initialName?: string;
    initialModifiers?: readonly Modifier[];
    fontFamilies?: readonly FontFamily[];
    onSubmit: (name: string, modifiers: Modifier[]) => void;
    onDelete?: () => void;
  };

  let {
    open = $bindable(false),
    mode = 'create',
    styleId,
    initialName = '',
    initialModifiers = [],
    fontFamilies = [],
    onSubmit,
    onDelete,
  }: Props = $props();

  const theme = getThemeContext();

  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );
  const tc = $derived(THEME_COLORS[themeVariant]);
  const textColorItems = $derived(values.textColor.map((c) => ({ label: c.label, value: c.value, color: tc[c.themeKey] })));
  const bgColorItems = $derived(
    values.textBackgroundColor.map((c) => ({ label: c.label, value: c.value, color: c.themeKey ? tc[c.themeKey] : null })),
  );

  let name = $state('');
  let fontFamily = $state<string | undefined>();
  let fontSize = $state<number | undefined>();
  let fontWeight = $state<number | undefined>();
  let letterSpacing = $state<number | undefined>();
  let textColor = $state<string | undefined>();
  let backgroundColor = $state<string | undefined>();

  const findMod = <T extends Modifier['type']>(mods: readonly Modifier[], type: T) =>
    mods.find((m): m is Extract<Modifier, { type: T }> => m.type === type);

  $effect(() => {
    if (open) {
      untrack(() => {
        name = initialName;
        fontFamily = findMod(initialModifiers, 'font_family')?.value;
        fontSize = findMod(initialModifiers, 'font_size')?.value;
        fontWeight = findMod(initialModifiers, 'font_weight')?.value;
        letterSpacing = findMod(initialModifiers, 'letter_spacing')?.value;
        textColor = findMod(initialModifiers, 'text_color')?.value;
        backgroundColor = findMod(initialModifiers, 'background_color')?.value;
      });
    }
  });

  const currentFontFamilyAndFonts = $derived.by(() => {
    if (fontFamily) {
      const family = fontFamilies.find((f) => f.familyName === fontFamily);
      if (family) {
        const fonts = [
          ...new Map(
            family.fonts
              .filter((f) => f.state === 'ACTIVE' || (fontWeight !== undefined && f.weight === fontWeight))
              .toSorted((a, b) => a.weight - b.weight)
              .map((f) => [f.weight, f]),
          ).values(),
        ];
        return { family: family.familyName, fonts };
      }
    }

    const fontsByWeight = new SvelteMap<number, Font>();
    for (const family of fontFamilies) {
      for (const font of family.fonts) {
        if (font.state === 'ACTIVE') {
          fontsByWeight.set(font.weight, font);
        }
      }
    }

    return {
      family: undefined as string | undefined,
      fonts: [...fontsByWeight.values()].toSorted((a, b) => a.weight - b.weight),
    };
  });

  const activeFontFamilies = $derived(fontFamilies.filter((f) => f.state === 'ACTIVE'));

  const fontFamilyItems = $derived.by(() => {
    const families =
      fontFamily && activeFontFamilies.every((f) => f.familyName !== fontFamily)
        ? [...activeFontFamilies, ...fontFamilies.filter((f) => f.familyName === fontFamily)]
        : activeFontFamilies;
    return families.map((f) => ({ value: f.familyName, label: f.displayName }));
  });

  const representativeFontMap = $derived.by(() => {
    const map = new SvelteMap<string, Font | null>();
    for (const family of fontFamilies) {
      const active = family.fonts.filter((f) => f.state === 'ACTIVE');
      if (active.length === 0) {
        map.set(family.familyName, null);
      } else {
        map.set(
          family.familyName,
          active.reduce((prev, curr) => {
            const prevDiff = Math.abs(prev.weight - 400);
            const currDiff = Math.abs(curr.weight - 400);
            if (currDiff < prevDiff) return curr;
            if (currDiff === prevDiff && curr.weight > prev.weight) return curr;
            return prev;
          }),
        );
      }
    }
    return map;
  });

  const weightItems = $derived.by(() => {
    const items = currentFontFamilyAndFonts.fonts.map((font) => ({
      value: font.weight,
      label:
        values.fontWeight.find((f) => f.value === font.weight)?.label ??
        (font.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${font.weight})` : String(font.weight)),
    }));

    if (fontWeight != null && items.every((w) => w.value !== fontWeight)) {
      items.push({
        value: fontWeight,
        label: values.fontWeight.find((f) => f.value === fontWeight)?.label ?? String(fontWeight),
      });
      items.sort((a, b) => a.value - b.value);
    }

    return items;
  });

  const weightFontIdMap = $derived(
    new Map(
      currentFontFamilyAndFonts.fonts.filter((f): f is typeof f & { id: string } => 'id' in f && !!f.id).map((f) => [f.weight, f.id]),
    ),
  );

  const modifiers = $derived.by(() => {
    const m: Modifier[] = [];
    if (fontFamily !== undefined) m.push({ type: 'font_family', value: fontFamily });
    if (fontSize !== undefined) m.push({ type: 'font_size', value: fontSize });
    if (fontWeight !== undefined) m.push({ type: 'font_weight', value: fontWeight });
    if (letterSpacing !== undefined) m.push({ type: 'letter_spacing', value: letterSpacing });
    if (textColor !== undefined) m.push({ type: 'text_color', value: textColor });
    if (backgroundColor !== undefined) m.push({ type: 'background_color', value: backgroundColor });
    return m;
  });

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

  $effect(() => {
    if (!fontSizeOpened && !fontSizeIsFocused) {
      fontSizeInputValue = fontSize === undefined ? '' : String(fontSize / 100);
    }
  });

  const applyFontSize = () => {
    const trimmed = fontSizeInputValue.trim();
    if (trimmed === '') {
      fontSize = undefined;
      return;
    }
    const parsed = Number.parseFloat(trimmed);
    if (!Number.isNaN(parsed)) {
      fontSize = clamp(Math.round(parsed * 100), values.minFontSize, values.maxFontSize);
    }
  };

  const handleFontSizeFocus = () => {
    fontSizeIsFocused = true;
    fontSizeOpened = true;
    fontSizeInputValue = fontSize === undefined ? '' : String(fontSize / 100);
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
      fontSizeInputValue = fontSize === undefined ? '' : String(fontSize / 100);
      fontSizeInputElement?.blur();
      fontSizeOpened = false;
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      const currentInput = Number.parseFloat(fontSizeInputValue);
      const current = (currentInput ? Math.round(currentInput * 100) : fontSize) || 1600;
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
        fontSize = newValue;
        tick().then(() => {
          fontSizeInputElement?.select();
        });
      }
    }
  };

  let textColorOpened = $state(false);
  let bgColorOpened = $state(false);
  let letterSpacingOpened = $state(false);

  const { anchor: letterSpacingAnchorAction, floating: letterSpacingFloatingAction } = createFloatingActions({
    placement: 'bottom-end',
    offset: 8,
    onClickOutside: () => {
      letterSpacingOpened = false;
    },
  });

  const { anchor: textColorAnchorAction, floating: textColorFloatingAction } = createFloatingActions({
    placement: 'bottom-end',
    offset: 8,
    onClickOutside: () => {
      textColorOpened = false;
    },
  });

  const { anchor: bgColorAnchorAction, floating: bgColorFloatingAction } = createFloatingActions({
    placement: 'bottom-end',
    offset: 8,
    onClickOutside: () => {
      bgColorOpened = false;
    },
  });

  const currentTextColorItem = $derived(textColorItems.find(({ value }) => value === textColor));
  const currentBgColorItem = $derived(bgColorItems.find(({ value }) => value === backgroundColor));

  const handleSubmit = () => {
    onSubmit(name.trim() || '새 스타일', modifiers);
    open = false;
  };

  const title = $derived(mode === 'create' ? '새 스타일 만들기' : '스타일 수정');
  const description = $derived(mode === 'create' ? '현재 서식을 기반으로 새 스타일을 만들어요.' : '스타일의 이름과 서식을 수정해요.');
  const submitLabel = $derived(mode === 'create' ? '생성' : '저장');

  const fieldLabel = css.raw({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' });
  const groupLabel = css.raw({
    paddingLeft: '2px',
    fontSize: '12px',
    fontWeight: 'semibold',
    letterSpacing: '0.02em',
    color: 'text.faint',
  });
  const trayClass = css.raw({
    display: 'flex',
    flexDirection: 'column',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '10px',
    backgroundColor: 'surface.default',
    overflow: 'hidden',
  });
  const rowClass = flex.raw({
    justifyContent: 'space-between',
    alignItems: 'center',
    gap: '12px',
    height: '42px',
    paddingX: '14px',
  });
  const rowLabelClass = flex.raw({ alignItems: 'center', gap: '8px', flexShrink: '0' });
  const popoverClass = css.raw({
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '4px',
    backgroundColor: 'surface.default',
    zIndex: 'tooltip',
    boxShadow: 'small',
    overflow: 'hidden',
  });
  const dropdownStyle = css.raw({
    width: '160px',
    height: '28px',
    '& > input': { textAlign: 'right', paddingRight: '24px', fontSize: '12px', fontWeight: 'medium' },
  });
  const listPopoverClass = css.raw({
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '4px',
    backgroundColor: 'surface.default',
    zIndex: 'tooltip',
    boxShadow: 'small',
    overflow: 'hidden',
    maxHeight: '240px',
    overflowY: 'auto',
    width: '160px',
  });
  const listItemClass = css.raw({
    display: 'flex',
    alignItems: 'center',
    width: 'full',
    paddingX: '12px',
    paddingY: '8px',
    fontSize: '12px',
    fontWeight: 'medium',
    color: 'text.subtle',
    textAlign: 'left',
    _hover: { backgroundColor: 'surface.muted' },
  });
</script>

<Modal style={css.raw({ padding: '24px', maxWidth: '440px' })} bind:open>
  <form
    class={flex({ flexDirection: 'column', gap: '20px' })}
    onsubmit={(e) => {
      e.preventDefault();
      handleSubmit();
    }}
  >
    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', letterSpacing: '-0.01em', color: 'text.default' })}>{title}</div>
      <div class={css({ fontSize: '13px', color: 'text.muted', wordBreak: 'keep-all' })}>{description}</div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <label class={css(groupLabel)} for="style-name">스타일 이름</label>
      <TextInput id="style-name" autofocus placeholder="새 스타일" size="md" bind:value={name} />
    </div>

    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css(groupLabel)}>서식</div>
      <div class={css(trayClass)}>
        <div class={css(rowClass)}>
          <div class={css(rowLabelClass)}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
            <div class={css(fieldLabel)}>글꼴</div>
          </div>
          <SearchableDropdown
            style={dropdownStyle}
            getLabel={(value) => fontFamilies.find((f) => f.familyName === value)?.displayName ?? '(알 수 없는 폰트)'}
            items={fontFamilyItems}
            label=""
            onchange={(value) => {
              fontFamily = value;
            }}
            placeholder="글꼴 선택"
            value={fontFamily}
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

        <div class={css(rowClass)}>
          <div class={css(rowLabelClass)}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
            <div class={css(fieldLabel)}>크기</div>
          </div>
          <div
            bind:this={fontSizeAnchorElement}
            class={css({
              position: 'relative',
              display: 'flex',
              alignItems: 'center',
              borderRadius: '6px',
              width: '160px',
              height: '28px',
              _hover: { backgroundColor: 'surface.muted' },
              _focusWithin: { backgroundColor: 'surface.muted' },
            })}
            use:fontSizeAnchorAction
          >
            <input
              bind:this={fontSizeInputElement}
              class={css({
                flexGrow: '1',
                size: 'full',
                paddingLeft: '10px',
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
              placeholder={fontSize === undefined ? '16' : String(fontSize / 100)}
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
            {#if fontSizeOpened}
              <div
                bind:this={fontSizeFloatingElement}
                class={css(listPopoverClass)}
                use:fontSizeFloatingAction
                in:fly={{ y: -5, duration: 150 }}
              >
                {#each values.fontSize as { label, value } (value)}
                  <button
                    class={css(listItemClass, { backgroundColor: fontSize === value ? 'surface.muted' : 'transparent' })}
                    onclick={() => {
                      fontSizeInputValue = String(value / 100);
                      fontSize = value;
                      fontSizeInputElement?.blur();
                      fontSizeOpened = false;
                    }}
                    tabindex="-1"
                    type="button"
                  >
                    {label}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <div class={css(rowClass)}>
          <div class={css(rowLabelClass)}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
            <div class={css(fieldLabel)}>굵기</div>
          </div>
          <SearchableDropdown
            style={dropdownStyle}
            getLabel={(value) => {
              const font = currentFontFamilyAndFonts.fonts.find((f) => f.weight === value);
              return (
                values.fontWeight.find((f) => f.value === value)?.label ??
                (font?.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${value})` : '(알 수 없는 굵기)')
              );
            }}
            items={weightItems}
            label=""
            onchange={(value) => {
              fontWeight = value;
            }}
            placeholder="굵기 선택"
            value={fontWeight}
          >
            {#snippet renderItem(item)}
              {@const font = currentFontFamilyAndFonts.fonts.find((candidate) => candidate.weight === item.value)}
              <FontSpecimen
                fallbacks={weightSpecimenFallbacks(item.label, font?.subfamilyDisplayName, item.value)}
                fontId={weightFontIdMap.get(item.value)}
                text={item.label}
                weight={item.value}
              />
            {/snippet}
          </SearchableDropdown>
        </div>

        <div class={css(rowClass)}>
          <div class={css(rowLabelClass)}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={LetterSpacingIcon} />
            <div class={css(fieldLabel)}>자간</div>
          </div>
          <div class={css({ position: 'relative' })}>
            <button
              class={css({
                position: 'relative',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'flex-end',
                borderRadius: '6px',
                width: '160px',
                height: '28px',
                paddingRight: '24px',
                _hover: { backgroundColor: 'surface.muted' },
              })}
              onclick={() => (letterSpacingOpened = !letterSpacingOpened)}
              type="button"
              use:letterSpacingAnchorAction
            >
              <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
                {values.letterSpacing.find((s) => s.value === (letterSpacing ?? 0))?.label ?? `${letterSpacing ?? 0}%`}
              </span>
              <Icon
                style={css.raw({
                  position: 'absolute',
                  right: '4px',
                  top: '1/2',
                  translate: 'auto',
                  translateY: '-1/2',
                  color: 'text.faint',
                  transform: letterSpacingOpened ? 'rotate(-180deg)' : 'rotate(0deg)',
                  transitionDuration: '150ms',
                })}
                icon={ChevronDownIcon}
                size={16}
              />
            </button>
            {#if letterSpacingOpened}
              <div class={css(listPopoverClass)} use:letterSpacingFloatingAction in:fly={{ y: -5, duration: 150 }}>
                {#each values.letterSpacing as { label, value } (value)}
                  <button
                    class={css(listItemClass, { backgroundColor: (letterSpacing ?? 0) === value ? 'surface.muted' : 'transparent' })}
                    onclick={() => {
                      letterSpacing = value;
                      letterSpacingOpened = false;
                    }}
                    tabindex="-1"
                    type="button"
                  >
                    {label}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <div class={css(rowClass)}>
          <div class={css(rowLabelClass)}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={PaletteIcon} />
            <div class={css(fieldLabel)}>글자 색</div>
          </div>
          <div class={css({ position: 'relative' })}>
            <button
              class={flex({
                alignItems: 'center',
                justifyContent: 'flex-end',
                gap: '8px',
                borderRadius: '6px',
                paddingX: '8px',
                height: '28px',
                width: '160px',
                _hover: { backgroundColor: 'surface.muted' },
              })}
              onclick={() => (textColorOpened = !textColorOpened)}
              type="button"
              use:textColorAnchorAction
            >
              <div
                style:background-color={currentTextColorItem?.color ?? 'transparent'}
                class={css({ borderWidth: '1px', borderRadius: 'full', size: '16px', flexShrink: '0' })}
              ></div>
              <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
                {currentTextColorItem?.label ?? '선택 안 함'}
              </span>
            </button>
            {#if textColorOpened}
              <div class={css(popoverClass)} use:textColorFloatingAction in:fly={{ y: -5, duration: 150 }}>
                <ToolbarColorGrid
                  columns={11}
                  currentValue={textColor}
                  items={textColorItems}
                  onClose={() => (textColorOpened = false)}
                  onSelect={(value) => {
                    textColor = value;
                    textColorOpened = false;
                  }}
                  opened={textColorOpened}
                />
              </div>
            {/if}
          </div>
        </div>

        <div class={css(rowClass)}>
          <div class={css(rowLabelClass)}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={PaletteIcon} />
            <div class={css(fieldLabel)}>배경 색</div>
          </div>
          <div class={css({ position: 'relative' })}>
            <button
              class={flex({
                alignItems: 'center',
                justifyContent: 'flex-end',
                gap: '8px',
                borderRadius: '6px',
                paddingX: '8px',
                height: '28px',
                width: '160px',
                _hover: { backgroundColor: 'surface.muted' },
              })}
              onclick={() => (bgColorOpened = !bgColorOpened)}
              type="button"
              use:bgColorAnchorAction
            >
              <div
                style:background-color={currentBgColorItem?.color ?? 'transparent'}
                class={css({ borderWidth: '1px', borderRadius: '4px', size: '16px', flexShrink: '0', position: 'relative' })}
              >
                {#if backgroundColor === 'none'}
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
                {currentBgColorItem?.label ?? '선택 안 함'}
              </span>
            </button>
            {#if bgColorOpened}
              <div class={css(popoverClass)} use:bgColorFloatingAction in:fly={{ y: -5, duration: 150 }}>
                <ToolbarColorGrid
                  columns={8}
                  currentValue={backgroundColor}
                  items={bgColorItems}
                  onClose={() => (bgColorOpened = false)}
                  onSelect={(value) => {
                    backgroundColor = value;
                    bgColorOpened = false;
                  }}
                  opened={bgColorOpened}
                  shape="square"
                />
              </div>
            {/if}
          </div>
        </div>
      </div>
    </div>

    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '10px' })}>
      <div>
        {#if mode === 'edit' && onDelete && styleId !== 'base'}
          <button
            class={flex({
              alignItems: 'center',
              gap: '6px',
              borderRadius: '6px',
              paddingX: '12px',
              paddingY: '6px',
              fontSize: '13px',
              fontWeight: 'medium',
              color: 'text.faint',
              transition: 'common',
              _hover: { backgroundColor: 'surface.muted', color: 'text.danger' },
            })}
            onclick={() => {
              onDelete?.();
              open = false;
            }}
            type="button"
          >
            <Icon icon={Trash2Icon} size={14} />
            <span>삭제</span>
          </button>
        {/if}
      </div>
      <div class={flex({ gap: '10px' })}>
        <Button
          style={css.raw({ paddingX: '16px' })}
          onclick={() => {
            open = false;
          }}
          type="button"
          variant="secondary"
        >
          취소
        </Button>
        <Button style={css.raw({ paddingX: '16px' })} type="submit">{submitLabel}</Button>
      </div>
    </div>
  </form>
</Modal>
