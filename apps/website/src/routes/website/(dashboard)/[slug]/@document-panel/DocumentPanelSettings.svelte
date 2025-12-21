<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, SegmentButtons, Select, Slider, Switch, TextInput, Tooltip } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { clamp, PAGE_LAYOUT_OPTIONS, PAGE_SIZE_MAP } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import AlignVerticalSpaceAroundIcon from '~icons/lucide/align-vertical-space-around';
  import ArrowRightToLineIcon from '~icons/lucide/arrow-right-to-line';
  import FileIcon from '~icons/lucide/file';
  import FileTextIcon from '~icons/lucide/file-text';
  import HighlighterIcon from '~icons/lucide/highlighter';
  import InfoIcon from '~icons/lucide/info';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import TypeIcon from '~icons/lucide/type';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  const app = getAppContext();

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
