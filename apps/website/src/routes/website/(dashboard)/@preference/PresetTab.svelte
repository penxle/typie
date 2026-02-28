<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, SearchableDropdown, SegmentButtons, Select, TextInput } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { Dialog } from '@typie/ui/notification';
  import { clamp, getMaxMargin } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import { defaultValues } from '@/const';
  import { PostLayoutMode } from '@/enums';
  import AlignVerticalSpaceAroundIcon from '~icons/lucide/align-vertical-space-around';
  import ArrowRightToLineIcon from '~icons/lucide/arrow-right-to-line';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import FileIcon from '~icons/lucide/file';
  import FileTextIcon from '~icons/lucide/file-text';
  import PaletteIcon from '~icons/lucide/palette';
  import PanelBottomIcon from '~icons/lucide/panel-bottom';
  import PanelLeftIcon from '~icons/lucide/panel-left';
  import PanelRightIcon from '~icons/lucide/panel-right';
  import PanelTopIcon from '~icons/lucide/panel-top';
  import RotateCcwIcon from '~icons/lucide/rotate-ccw';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import { FontSpecimen, SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { getRepresentativeFont } from '$lib/editor/fonts';
  import { THEME_COLORS } from '$lib/editor/theme';
  import { createPaginatedLayout } from '$lib/editor/utils';
  import { values } from '$lib/editor/values';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import ToolbarColorGrid from '../[slug]/@toolbar/ToolbarColorGrid.svelte';
  import type { ThemeVariant } from '$lib/editor/theme';
  import type { PageLayout, PageLayoutPreset } from '$lib/editor/utils';
  import type { DashboardLayout_PreferenceModal_PresetTab_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_PresetTab_user$key;
  };

  let { user$key }: Props = $props();

  const theme = getThemeContext();
  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );
  const tc = $derived(THEME_COLORS[themeVariant]);

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_PresetTab_user on User {
        id
        preferences

        documentFontFamilies {
          id
          familyName
          displayName
          source
          state

          fonts {
            id
            weight
            state
            subfamilyDisplayName
            url
          }
        }
      }
    `),
    () => user$key,
  );

  const [updatePreferences] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_PresetTab_UpdatePreferences_Mutation($input: UpdatePreferencesInput!) {
        updatePreferences(input: $input) {
          id
        }
      }
    `),
  );

  type PresetPreference = {
    fontFamily?: string;
    fontSize?: number;
    fontWeight?: number;
    textColor?: string;
    backgroundColor?: string;
    letterSpacing?: number;
    lineHeight?: number;
    layoutMode?: PostLayoutMode;
    maxWidth?: number;
    pageLayout?: PageLayout | null;
    paragraphIndent?: number;
    blockGap?: number;
  };

  const userId = $derived(user.data?.id);
  const preferences = $derived((user.data?.preferences as Record<string, unknown> | undefined) ?? {});
  const documentFontFamilies = $derived(user.data?.documentFontFamilies ?? []);
  const template = $derived<PresetPreference>((preferences.template as PresetPreference | undefined) ?? {});

  const fontFamily = $derived(template.fontFamily ?? defaultValues.fontFamily);
  const fontSize = $derived(template.fontSize ?? defaultValues.fontSize);
  const fontWeight = $derived(template.fontWeight ?? defaultValues.fontWeight);

  const fontFamilyItems = $derived(
    documentFontFamilies.filter((f) => f.state === 'ACTIVE').map((f) => ({ value: f.familyName, label: f.displayName })),
  );

  const currentFontFamilyFonts = $derived.by(() => {
    const family = documentFontFamilies.find((f) => f.familyName === fontFamily);
    if (!family) return [];
    return [...new Map(family.fonts.filter((f) => f.state === 'ACTIVE').map((f) => [f.weight, f])).values()].toSorted(
      (a, b) => a.weight - b.weight,
    );
  });

  const fontWeightItems = $derived(
    currentFontFamilyFonts.map((font) => ({
      value: font.weight,
      label:
        values.fontWeight.find(({ value }) => value === font.weight)?.label ||
        (font.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${font.weight})` : String(font.weight)),
    })),
  );

  const representativeFontMap = $derived(new Map(documentFontFamilies.map((f) => [f.familyName, getRepresentativeFont(f.fonts)])));

  const weightFontIdMap = $derived(new Map(currentFontFamilyFonts.map((f) => [f.weight, f.id])));

  const getClosestWeight = (familyName: string, targetWeight: number) => {
    const family = documentFontFamilies.find((f) => f.familyName === familyName);
    if (!family) return targetWeight;

    const weights = [...new Set(family.fonts.filter((f) => f.state === 'ACTIVE').map((f) => f.weight))].toSorted((a, b) => a - b);
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

  const textColor = $derived(template.textColor ?? defaultValues.textColor);
  const backgroundColor = $derived(template.backgroundColor ?? defaultValues.backgroundColor);
  const letterSpacing = $derived(template.letterSpacing ?? defaultValues.letterSpacing);
  const lineHeight = $derived(template.lineHeight ?? defaultValues.lineHeight);
  const layoutMode = $derived(template.layoutMode ?? PostLayoutMode.SCROLL);
  const maxWidth = $derived(template.maxWidth ?? defaultValues.maxWidth);
  const pageLayout = $derived(template.pageLayout ?? null);
  const paragraphIndent = $derived(template.paragraphIndent ?? defaultValues.paragraphIndent);
  const blockGap = $derived(template.blockGap ?? defaultValues.blockGap);

  const textColorItems = $derived(values.textColor.map((c) => ({ label: c.label, value: c.value, color: tc[c.themeKey] })));
  const bgColorItems = $derived(
    values.textBackgroundColor.map((c) => ({ label: c.label, value: c.value, color: c.themeKey ? tc[c.themeKey] : null })),
  );

  const isPageLayoutEnabled = $derived(layoutMode === PostLayoutMode.PAGE);

  const updateTemplate = async (updates: Partial<PresetPreference>) => {
    const newTemplate = { ...template, ...updates };
    await updatePreferences({ input: { value: { template: newTemplate } } });
    if (userId) {
      cache.invalidate({ __typename: 'User', id: userId, $field: 'preferences' });
    }

    mixpanel.track('update_post_template', {
      updates: Object.keys(updates),
    });
  };

  const resetTemplate = async () => {
    await updatePreferences({ input: { value: { template: {} } } });
    if (userId) {
      cache.invalidate({ __typename: 'User', id: userId, $field: 'preferences' });
    }

    mixpanel.track('reset_post_template');
  };

  let textColorOpened = $state(false);
  let bgColorOpened = $state(false);

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

  const { anchor: fontSizeAnchorAction, floating: fontSizeFloatingAction } = createFloatingActions({
    placement: 'bottom-start',
    offset: 8,
    onClickOutside: (event) => {
      if (fontSizeAnchorElement?.contains(event.target as Node)) {
        return;
      }
      closeFontSizeDropdown();
    },
  });

  let fontSizeOpened = $state(false);
  let fontSizeInputElement: HTMLInputElement | undefined = $state();
  let fontSizeInputValue = $state('');
  let fontSizeIsFocused = $state(false);

  $effect(() => {
    if (!fontSizeOpened && document.activeElement !== fontSizeInputElement) {
      fontSizeInputValue = String(fontSize / 100);
    }
  });

  const openFontSizeDropdown = () => {
    fontSizeOpened = true;
  };

  const closeFontSizeDropdown = () => {
    fontSizeOpened = false;
  };

  const handleFontSizeFocus = () => {
    fontSizeIsFocused = true;
    openFontSizeDropdown();
    fontSizeInputValue = String(fontSize / 100);
    fontSizeInputElement?.select();
  };

  const applyFontSize = () => {
    const parsed = Number.parseFloat(fontSizeInputValue);
    if (!Number.isNaN(parsed) && Math.round(parsed * 100) !== fontSize) {
      const clamped = clamp(Math.round(parsed * 100), values.minFontSize, values.maxFontSize);
      updateTemplate({ fontSize: clamped });
    }
  };

  const handleFontSizeBlur = (e: FocusEvent) => {
    fontSizeIsFocused = false;

    const relatedTarget = e.relatedTarget as Node | null;
    if (relatedTarget && fontSizeFloatingElement?.contains(relatedTarget)) {
      return;
    }

    closeFontSizeDropdown();
  };

  $effect(() => {
    if (!fontSizeIsFocused && fontSizeInputValue) {
      applyFontSize();
    }
  });

  const handleFontSizeKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      applyFontSize();
      fontSizeInputElement?.blur();
      closeFontSizeDropdown();
    } else if (e.key === 'Escape') {
      fontSizeInputValue = String(fontSize / 100);
      fontSizeInputElement?.blur();
      closeFontSizeDropdown();
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      const currentInput = Number.parseFloat(fontSizeInputValue);
      const current = (currentInput ? Math.round(currentInput * 100) : fontSize) || 1200;
      const sortedSizes = values.fontSize.map(({ value }) => value).toSorted((a, b) => a - b);
      const currentIndex = sortedSizes.findIndex((size) => size >= current);

      let newIndex: number;
      if (e.key === 'ArrowDown') {
        if (currentIndex === -1) {
          newIndex = sortedSizes.length - 1;
        } else if (currentIndex >= sortedSizes.length - 1) {
          newIndex = 0;
        } else {
          newIndex = currentIndex + 1;
        }
      } else {
        if (currentIndex === -1) {
          newIndex = 0;
        } else if (currentIndex <= 0) {
          newIndex = sortedSizes.length - 1;
        } else {
          newIndex = currentIndex - 1;
        }
      }

      const newValue = sortedSizes[newIndex];
      if (newValue !== undefined) {
        fontSizeInputValue = String(newValue / 100);
        updateTemplate({ fontSize: newValue });
        tick().then(() => {
          fontSizeInputElement?.select();
        });
      }
    }
  };
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
    <div>
      <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>프리셋</h1>
      <p class={css({ fontSize: '13px', color: 'text.subtle', lineHeight: '[1.6]', marginTop: '8px' })}>
        새 문서를 생성할 때 자동으로 적용될 기본 포맷을 설정해요.
      </p>
    </div>
    <button
      class={flex({
        alignItems: 'center',
        gap: '6px',
        borderRadius: '6px',
        paddingX: '12px',
        paddingY: '6px',
        fontSize: '13px',
        fontWeight: 'medium',
        color: 'text.subtle',
        transition: 'common',
        _hover: { backgroundColor: 'surface.muted' },
      })}
      onclick={() => {
        Dialog.confirm({
          title: '프리셋 초기화',
          message: '모든 프리셋 설정을 기본값으로 되돌려요. 이 작업은 되돌릴 수 없어요.',
          actionLabel: '초기화',
          action: 'danger',
          actionHandler: resetTemplate,
          cancelLabel: '취소',
        });
      }}
      type="button"
    >
      <Icon style={css.raw({ color: 'text.faint' })} icon={RotateCcwIcon} size={14} />
      <span>초기화</span>
    </button>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '20px' })}>기본 스타일</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          폰트 패밀리
        {/snippet}
        {#snippet value()}
          <SearchableDropdown
            style={css.raw({
              width: '160px',
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
              const closestWeight = getClosestWeight(value, fontWeight);
              updateTemplate({ fontFamily: value, fontWeight: closestWeight });
            }}
            value={fontFamily}
          >
            {#snippet renderItem(item)}
              {@const font = representativeFontMap.get(item.value)}
              <FontSpecimen fontId={font?.id} text={item.label} weight={font?.weight} />
            {/snippet}
          </SearchableDropdown>
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          폰트 굵기
        {/snippet}
        {#snippet value()}
          <SearchableDropdown
            style={css.raw({
              width: '120px',
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
              updateTemplate({ fontWeight: value });
            }}
            value={fontWeight}
          >
            {#snippet renderItem(item)}
              <FontSpecimen fontId={weightFontIdMap.get(item.value)} text={item.label} weight={item.value} />
            {/snippet}
          </SearchableDropdown>
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          폰트 크기
        {/snippet}
        {#snippet value()}
          <div class={css({ position: 'relative', width: '60px' })}>
            <div
              bind:this={fontSizeAnchorElement}
              class={css({
                display: 'flex',
                alignItems: 'center',
                borderRadius: '6px',
                paddingX: '8px',
                paddingY: '4px',
                height: '28px',
                _hover: {
                  backgroundColor: 'surface.muted',
                },
                _focusWithin: {
                  backgroundColor: 'surface.muted',
                },
              })}
              use:fontSizeAnchorAction
            >
              <input
                bind:this={fontSizeInputElement}
                class={css({
                  flexGrow: '1',
                  width: 'full',
                  fontSize: '12px',
                  fontWeight: 'medium',
                  color: 'text.subtle',
                  textAlign: 'left',
                  backgroundColor: 'transparent',
                  border: 'none',
                  outline: 'none',
                })}
                onblur={handleFontSizeBlur}
                onfocus={handleFontSizeFocus}
                onkeydown={handleFontSizeKeydown}
                placeholder={String(fontSize / 100)}
                type="text"
                bind:value={fontSizeInputValue}
              />

              <Icon
                style={css.raw({
                  position: 'absolute',
                  right: '8px',
                  top: '1/2',
                  translate: 'auto',
                  translateY: '-1/2',
                  color: 'text.faint',
                  pointerEvents: 'none',
                  transform: fontSizeOpened ? 'rotate(-180deg)' : 'rotate(0deg)',
                  transitionDuration: '150ms',
                })}
                icon={ChevronDownIcon}
                size={14}
              />
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
                  maxHeight: '200px',
                  overflowY: 'auto',
                })}
                use:fontSizeFloatingAction
                in:fly={{ y: -5, duration: 150 }}
              >
                {#each values.fontSize as { label, value } (value)}
                  <button
                    class={css({
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      gap: '20px',
                      width: 'full',
                      paddingX: '12px',
                      paddingY: '8px',
                      fontSize: '12px',
                      fontWeight: 'medium',
                      color: 'text.subtle',
                      backgroundColor: fontSize === value ? 'surface.muted' : 'transparent',
                      transition: 'common',
                      _hover: { backgroundColor: 'surface.muted' },
                    })}
                    onclick={() => {
                      fontSizeInputValue = String(value / 100);
                      updateTemplate({ fontSize: value });
                      fontSizeInputElement?.blur();
                      closeFontSizeDropdown();
                    }}
                    tabindex="-1"
                    type="button"
                  >
                    <span>{label}</span>
                    {#if fontSize === value}
                      <Icon style={css.raw({ color: 'text.subtle' })} icon={CheckIcon} size={14} />
                    {:else}
                      <div style:width="14px"></div>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={LetterSpacingIcon} />
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>자간</div>
          </div>
        {/snippet}
        {#snippet value()}
          <Select
            items={values.letterSpacing.map((s) => ({ value: s.value, label: s.label }))}
            onselect={(value) => {
              updateTemplate({ letterSpacing: value });
            }}
            value={letterSpacing}
          />
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={LineHeightIcon} />
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>행간</div>
          </div>
        {/snippet}
        {#snippet value()}
          <Select
            items={values.lineHeight.map((h) => ({ value: h.value, label: h.label }))}
            onselect={(value) => {
              updateTemplate({ lineHeight: value });
            }}
            value={lineHeight}
          />
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={PaletteIcon} />
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>글자 색</div>
          </div>
        {/snippet}
        {#snippet value()}
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
              style:background-color={textColorItems.find(({ value }) => value === textColor)?.color}
              class={css({ borderWidth: '1px', borderRadius: 'full', size: '16px', flexShrink: '0' })}
            ></div>
            <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
              {textColorItems.find(({ value }) => value === textColor)?.label ?? textColor}
            </span>
          </button>
          {#if textColorOpened}
            <div
              class={css({
                borderWidth: '1px',
                borderColor: 'border.subtle',
                borderRadius: '4px',
                backgroundColor: 'surface.default',
                zIndex: 'tooltip',
                boxShadow: 'small',
                overflow: 'hidden',
              })}
              use:textColorFloatingAction
              in:fly={{ y: -5, duration: 150 }}
            >
              <ToolbarColorGrid
                columns={11}
                currentValue={textColor}
                items={textColorItems}
                onClose={() => (textColorOpened = false)}
                onSelect={(value) => {
                  updateTemplate({ textColor: value });
                  textColorOpened = false;
                }}
                opened={textColorOpened}
              />
            </div>
          {/if}
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={PaletteIcon} />
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>배경 색</div>
          </div>
        {/snippet}
        {#snippet value()}
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
              style:background-color={bgColorItems.find(({ value }) => value === backgroundColor)?.color ?? 'transparent'}
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
              {bgColorItems.find(({ value }) => value === backgroundColor)?.label ?? backgroundColor}
            </span>
          </button>
          {#if bgColorOpened}
            <div
              class={css({
                borderWidth: '1px',
                borderColor: 'border.subtle',
                borderRadius: '4px',
                backgroundColor: 'surface.default',
                zIndex: 'tooltip',
                boxShadow: 'small',
                overflow: 'hidden',
              })}
              use:bgColorFloatingAction
              in:fly={{ y: -5, duration: 150 }}
            >
              <ToolbarColorGrid
                columns={8}
                currentValue={backgroundColor}
                items={bgColorItems}
                onClose={() => (bgColorOpened = false)}
                onSelect={(value) => {
                  updateTemplate({ backgroundColor: value });
                  bgColorOpened = false;
                }}
                opened={bgColorOpened}
                shape="square"
              />
            </div>
          {/if}
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '20px' })}>레이아웃</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={FileTextIcon} />
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>레이아웃 모드</div>
          </div>
        {/snippet}
        {#snippet value()}
          <div class={css({ width: '140px' })}>
            <SegmentButtons
              items={[
                { label: '스크롤', value: PostLayoutMode.SCROLL },
                { label: '페이지', value: PostLayoutMode.PAGE },
              ]}
              onselect={(value: PostLayoutMode) => {
                if (value === PostLayoutMode.PAGE && !pageLayout) {
                  updateTemplate({
                    layoutMode: value,
                    pageLayout: createPaginatedLayout('a4'),
                  });
                } else {
                  updateTemplate({ layoutMode: value });
                }
              }}
              size="sm"
              value={layoutMode}
            />
          </div>
        {/snippet}
      </SettingsRow>

      {#if isPageLayoutEnabled && pageLayout}
        <SettingsDivider />
        <SettingsRow>
          {#snippet label()}
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} />
              <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>페이지 크기 (mm)</div>
            </div>
          {/snippet}
          {#snippet value()}
            <div class={flex({ gap: '8px', alignItems: 'center' })}>
              <Select
                items={[...values.pageLayout, { label: '직접 지정', value: 'custom' }]}
                onselect={(value: string) => {
                  if (value !== 'custom') {
                    updateTemplate({ pageLayout: createPaginatedLayout(value as PageLayoutPreset) });
                  }
                }}
                value={values.pageLayout.find((p) => p.width === pageLayout.width && p.height === pageLayout.height)?.value ?? 'custom'}
              />
              <div class={flex({ gap: '6px', alignItems: 'center' })}>
                <TextInput
                  style={css.raw({ width: '70px' })}
                  min="100"
                  onchange={(e) => {
                    if (!pageLayout) return;
                    const target = e.target as HTMLInputElement;
                    const value = Math.max(100, Number(target.value));
                    target.value = String(value);
                    updateTemplate({
                      pageLayout: {
                        ...pageLayout,
                        width: value,
                      },
                    });
                  }}
                  placeholder="너비"
                  size="sm"
                  type="number"
                  value={pageLayout.width}
                />
                <span class={css({ fontSize: '12px', color: 'text.faint' })}>×</span>
                <TextInput
                  style={css.raw({ width: '70px' })}
                  min="100"
                  onchange={(e) => {
                    if (!pageLayout) return;
                    const target = e.target as HTMLInputElement;
                    const value = Math.max(100, Number(target.value));
                    target.value = String(value);
                    updateTemplate({
                      pageLayout: {
                        ...pageLayout,
                        height: value,
                      },
                    });
                  }}
                  placeholder="높이"
                  size="sm"
                  type="number"
                  value={pageLayout.height}
                />
              </div>
            </div>
          {/snippet}
        </SettingsRow>

        <SettingsDivider />

        <SettingsRow>
          {#snippet label()}
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'text.faint' })} icon={RulerDimensionLineIcon} />
              <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>여백 (mm)</div>
            </div>
          {/snippet}
          {#snippet value()}
            <div class={flex({ gap: '8px', alignItems: 'center' })}>
              <div class={flex({ gap: '4px', alignItems: 'center' })}>
                <Icon style={css.raw({ color: 'text.faint' })} icon={PanelTopIcon} size={14} />
                <TextInput
                  style={css.raw({ width: '56px' })}
                  max={pageLayout ? String(getMaxMargin('top', pageLayout)) : undefined}
                  min="0"
                  oninput={(e) => {
                    if (!pageLayout) return;
                    const target = e.target as HTMLInputElement;
                    const value = clamp(Number(target.value), 0, getMaxMargin('top', pageLayout));
                    target.value = String(value);
                    updateTemplate({
                      pageLayout: {
                        ...pageLayout,
                        marginTop: value,
                      },
                    });
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.marginTop}
                />
              </div>
              <div class={flex({ gap: '4px', alignItems: 'center' })}>
                <Icon style={css.raw({ color: 'text.faint' })} icon={PanelBottomIcon} size={14} />
                <TextInput
                  style={css.raw({ width: '56px' })}
                  max={pageLayout ? String(getMaxMargin('bottom', pageLayout)) : undefined}
                  min="0"
                  oninput={(e) => {
                    if (!pageLayout) return;
                    const target = e.target as HTMLInputElement;
                    const value = clamp(Number(target.value), 0, getMaxMargin('bottom', pageLayout));
                    target.value = String(value);
                    updateTemplate({
                      pageLayout: {
                        ...pageLayout,
                        marginBottom: value,
                      },
                    });
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.marginBottom}
                />
              </div>
              <div class={flex({ gap: '4px', alignItems: 'center' })}>
                <Icon style={css.raw({ color: 'text.faint' })} icon={PanelLeftIcon} size={14} />
                <TextInput
                  style={css.raw({ width: '56px' })}
                  max={pageLayout ? String(getMaxMargin('left', pageLayout)) : undefined}
                  min="0"
                  onchange={(e) => {
                    if (!pageLayout) return;
                    const target = e.target as HTMLInputElement;
                    const value = clamp(Number(target.value), 0, getMaxMargin('left', pageLayout));
                    target.value = String(value);
                    updateTemplate({
                      pageLayout: {
                        ...pageLayout,
                        marginLeft: value,
                      },
                    });
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.marginLeft}
                />
              </div>
              <div class={flex({ gap: '4px', alignItems: 'center' })}>
                <Icon style={css.raw({ color: 'text.faint' })} icon={PanelRightIcon} size={14} />
                <TextInput
                  style={css.raw({ width: '56px' })}
                  max={pageLayout ? String(getMaxMargin('right', pageLayout)) : undefined}
                  min="0"
                  oninput={(e) => {
                    if (!pageLayout) return;
                    const target = e.target as HTMLInputElement;
                    const value = clamp(Number(target.value), 0, getMaxMargin('right', pageLayout));
                    target.value = String(value);
                    updateTemplate({
                      pageLayout: {
                        ...pageLayout,
                        marginRight: value,
                      },
                    });
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.marginRight}
                />
              </div>
            </div>
          {/snippet}
        </SettingsRow>
      {/if}

      {#if !isPageLayoutEnabled}
        <SettingsDivider />
        <SettingsRow>
          {#snippet label()}
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'text.faint' })} icon={RulerDimensionLineIcon} />
              <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>본문 폭</div>
            </div>
          {/snippet}
          {#snippet value()}
            <div class={css({ width: '200px' })}>
              <SegmentButtons
                items={[
                  { label: '400px', value: 400 },
                  { label: '600px', value: 600 },
                  { label: '800px', value: 800 },
                ]}
                onselect={(value) => {
                  updateTemplate({ maxWidth: value });
                }}
                size="sm"
                value={maxWidth}
              />
            </div>
          {/snippet}
        </SettingsRow>
      {/if}
    </SettingsCard>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '20px' })}>세부 레이아웃</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={ArrowRightToLineIcon} />
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>첫 줄 들여쓰기</div>
          </div>
        {/snippet}
        {#snippet value()}
          <div class={css({ width: '200px' })}>
            <SegmentButtons
              items={[
                { label: '없음', value: 0 },
                { label: '0.5칸', value: 50 },
                { label: '1칸', value: 100 },
                { label: '2칸', value: 200 },
              ]}
              onselect={(value) => {
                updateTemplate({ paragraphIndent: value });
              }}
              size="sm"
              value={paragraphIndent}
            />
          </div>
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={AlignVerticalSpaceAroundIcon} />
            <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>문단 사이 간격</div>
          </div>
        {/snippet}
        {#snippet value()}
          <div class={css({ width: '200px' })}>
            <SegmentButtons
              items={[
                { label: '없음', value: 0 },
                { label: '0.5줄', value: 50 },
                { label: '1줄', value: 100 },
                { label: '2줄', value: 200 },
              ]}
              onselect={(value) => {
                updateTemplate({ blockGap: value });
              }}
              size="sm"
              value={blockGap}
            />
          </div>
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>
</div>
