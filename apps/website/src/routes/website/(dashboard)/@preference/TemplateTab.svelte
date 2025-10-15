<script lang="ts">
  import { cache } from '@typie/sark/internal';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, SegmentButtons, Select, TextInput } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { defaultValues, values } from '@typie/ui/tiptap';
  import { clamp, createDefaultPageLayout, getMaxMargin, PAGE_LAYOUT_OPTIONS, PAGE_SIZE_MAP } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import { fly } from 'svelte/transition';
  import { PostLayoutMode } from '@/enums';
  import AlignVerticalSpaceAroundIcon from '~icons/lucide/align-vertical-space-around';
  import ArrowRightToLineIcon from '~icons/lucide/arrow-right-to-line';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import FileIcon from '~icons/lucide/file';
  import FileTextIcon from '~icons/lucide/file-text';
  import PanelBottomIcon from '~icons/lucide/panel-bottom';
  import PanelLeftIcon from '~icons/lucide/panel-left';
  import PanelRightIcon from '~icons/lucide/panel-right';
  import PanelTopIcon from '~icons/lucide/panel-top';
  import RotateCcwIcon from '~icons/lucide/rotate-ccw';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import ToolbarSearchableDropdown from '../[slug]/@toolbar/ToolbarSearchableDropdown.svelte';
  import type { PageLayout, PageLayoutPreset } from '@typie/ui/utils';
  import type { DashboardLayout_PreferenceModal_TemplateTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_TemplateTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_TemplateTab_user on User {
        id
        preferences

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

  const updatePreferences = graphql(`
    mutation DashboardLayout_PreferenceModal_TemplateTab_UpdatePreferences_Mutation($input: UpdatePreferencesInput!) {
      updatePreferences(input: $input) {
        id
      }
    }
  `);

  type TemplatePreference = {
    fontFamily?: string;
    fontSize?: number;
    fontWeight?: number;
    letterSpacing?: number;
    lineHeight?: number;
    layoutMode?: PostLayoutMode;
    maxWidth?: number;
    pageLayout?: PageLayout | null;
    paragraphIndent?: number;
    blockGap?: number;
  };

  const template = $derived<TemplatePreference>(($user.preferences as Record<string, unknown>)?.template ?? {});

  const fontFamily = $derived(template.fontFamily ?? defaultValues.fontFamily);
  const fontSize = $derived(template.fontSize ?? defaultValues.fontSize);
  const fontWeight = $derived(template.fontWeight ?? defaultValues.fontWeight);

  const fontFamilyItems = $derived([
    ...values.fontFamily.map((f) => ({ value: f.value, label: f.label })),
    ...$user.fontFamilies.map((f) => ({ value: f.id, label: f.name })),
  ]);

  const currentFontFamilyWeights = $derived.by(() => {
    const systemFontFamily = values.fontFamily.find((f) => f.value === fontFamily);
    if (systemFontFamily) {
      return systemFontFamily.weights.toSorted((a, b) => a - b);
    }

    const userFontFamily = $user.fontFamilies.find((f) => f.id === fontFamily);
    if (userFontFamily) {
      return userFontFamily.fonts.map((f) => f.weight).toSorted((a, b) => a - b);
    }

    return values.fontFamily[0].weights.toSorted((a, b) => a - b);
  });

  const fontWeightItems = $derived(
    currentFontFamilyWeights.map((weight) => ({
      value: weight,
      label: values.fontWeight.find(({ value }) => value === weight)?.label || String(weight),
    })),
  );

  const getDefaultWeight = (fontFamilyOrId: string, fontWeight: number) => {
    let weights: number[];

    const systemFontFamily = values.fontFamily.find((f) => f.value === fontFamilyOrId);
    if (systemFontFamily) {
      weights = systemFontFamily.weights.toSorted((a, b) => a - b);
    } else {
      const userFontFamily = $user.fontFamilies.find((f) => f.id === fontFamilyOrId);
      if (!userFontFamily) return null;

      weights = userFontFamily.fonts.map((f) => f.weight).toSorted((a, b) => a - b);
    }

    if (weights.length === 0) return null;

    if (weights.includes(fontWeight)) {
      return fontWeight;
    }

    let closest = weights[0];
    let minDiff = Math.abs(fontWeight - weights[0]);

    for (const weight of weights) {
      const diff = Math.abs(fontWeight - weight);
      if (diff < minDiff) {
        minDiff = diff;
        closest = weight;
      }
    }

    return closest;
  };

  const letterSpacing = $derived(template.letterSpacing ?? defaultValues.letterSpacing);
  const lineHeight = $derived(template.lineHeight ?? defaultValues.lineHeight);
  const layoutMode = $derived(template.layoutMode ?? PostLayoutMode.SCROLL);
  const maxWidth = $derived(template.maxWidth ?? defaultValues.maxWidth);
  const pageLayout = $derived(template.pageLayout ?? null);
  const paragraphIndent = $derived(template.paragraphIndent ?? defaultValues.paragraphIndent);
  const blockGap = $derived(template.blockGap ?? defaultValues.blockGap);

  const isPageLayoutEnabled = $derived(layoutMode === PostLayoutMode.PAGE);

  const updateTemplate = async (updates: Partial<TemplatePreference>) => {
    const newTemplate = { ...template, ...updates };
    await updatePreferences({ value: { template: newTemplate } });
    cache.invalidate({ __typename: 'User', id: $user.id, field: 'preferences' });

    mixpanel.track('update_post_template', {
      updates: Object.keys(updates),
    });
  };

  const resetTemplate = async () => {
    await updatePreferences({ value: { template: {} } });
    cache.invalidate({ __typename: 'User', id: $user.id, field: 'preferences' });

    mixpanel.track('reset_post_template');
  };

  const MIN_FONT_SIZE = 8;
  const MAX_FONT_SIZE = 72;

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
      fontSizeInputValue = String(fontSize);
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
    fontSizeInputValue = String(fontSize);
    fontSizeInputElement?.select();
  };

  const applyFontSize = () => {
    const parsed = Number.parseFloat(fontSizeInputValue);
    if (!Number.isNaN(parsed) && parsed !== fontSize) {
      const clamped = clamp(parsed, MIN_FONT_SIZE, MAX_FONT_SIZE);
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
      fontSizeInputValue = String(fontSize);
      fontSizeInputElement?.blur();
      closeFontSizeDropdown();
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      const current = Number.parseFloat(fontSizeInputValue) || fontSize;
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
        fontSizeInputValue = String(newValue);
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
      <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>템플릿</h1>
      <p class={css({ fontSize: '13px', color: 'text.subtle', lineHeight: '[1.6]', marginTop: '8px' })}>
        새 포스트를 생성할 때 자동으로 적용될 기본 포맷을 설정해요.
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
          title: '템플릿 초기화',
          message: '모든 템플릿 설정을 기본값으로 되돌려요. 이 작업은 되돌릴 수 없어요.',
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
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '20px' })}>폰트</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          폰트 패밀리
        {/snippet}
        {#snippet value()}
          <ToolbarSearchableDropdown
            style={css.raw({ width: '160px', height: '28px', paddingX: '8px' })}
            getLabel={(value) => {
              const item = fontFamilyItems.find((f) => f.value === value);
              return item?.label ?? '(알 수 없는 폰트)';
            }}
            items={fontFamilyItems}
            label="폰트 패밀리"
            onchange={(value) => {
              const defaultWeight = getDefaultWeight(value, fontWeight) ?? defaultValues.fontWeight;
              updateTemplate({ fontFamily: value, fontWeight: defaultWeight });
            }}
            value={fontFamily}
          >
            {#snippet renderItem(item)}
              <div style:font-family={item.value}>{item.label}</div>
            {/snippet}
          </ToolbarSearchableDropdown>
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          폰트 굵기
        {/snippet}
        {#snippet value()}
          <ToolbarSearchableDropdown
            style={css.raw({ width: '120px', height: '28px', paddingX: '8px' })}
            getLabel={(value) => {
              const item = fontWeightItems.find((w) => w.value === value);
              return item?.label ?? '(알 수 없는 굵기)';
            }}
            items={fontWeightItems}
            label="폰트 굵기"
            onchange={(value) => {
              updateTemplate({ fontWeight: value });
            }}
            value={fontWeight}
          >
            {#snippet renderItem(item)}
              <div style:font-family={fontFamily} style:font-weight={item.value}>
                {item.label}
              </div>
            {/snippet}
          </ToolbarSearchableDropdown>
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
                placeholder={String(fontSize)}
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
                  zIndex: '[9999]',
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
                      fontSizeInputValue = String(value);
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
    </SettingsCard>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '20px' })}>간격</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          자간
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
          행간
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
                    pageLayout: createDefaultPageLayout('a4'),
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
                items={PAGE_LAYOUT_OPTIONS}
                onselect={(value: PageLayoutPreset | 'custom') => {
                  if (value !== 'custom') {
                    updateTemplate({ pageLayout: createDefaultPageLayout(value) });
                  }
                }}
                value={(Object.entries(PAGE_SIZE_MAP).find(
                  ([, dimension]) => dimension.width === pageLayout.width && dimension.height === pageLayout.height,
                )?.[0] as PageLayoutPreset) ?? ('custom' as const)}
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
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '20px' })}>문단</h2>

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
                { label: '0.5칸', value: 0.5 },
                { label: '1칸', value: 1 },
                { label: '2칸', value: 2 },
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
                { label: '0.5줄', value: 0.5 },
                { label: '1줄', value: 1 },
                { label: '2줄', value: 2 },
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
