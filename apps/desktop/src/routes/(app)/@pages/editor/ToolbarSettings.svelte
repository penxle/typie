<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, SegmentButtons, Select, Slider, Switch, Tooltip } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { createDefaultPageLayout, DEFAULT_PAGE_MARGINS, PAGE_LAYOUT_OPTIONS } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import AlignVerticalSpaceAroundIcon from '~icons/lucide/align-vertical-space-around';
  import ArrowRightToLineIcon from '~icons/lucide/arrow-right-to-line';
  import FileTextIcon from '~icons/lucide/file-text';
  import HighlighterIcon from '~icons/lucide/highlighter';
  import InfoIcon from '~icons/lucide/info';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import SettingsIcon from '~icons/lucide/settings';
  import TypeIcon from '~icons/lucide/type';
  import { YState } from './state.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import type { Editor } from '@tiptap/core';
  import type { PageLayoutSettings, PageLayoutSize, Ref } from '@typie/ui/utils';
  import type * as Y from 'yjs';

  type Props = {
    editor?: Ref<Editor>;
    doc: Y.Doc;
  };

  let { editor, doc }: Props = $props();

  const app = getAppContext();
  const maxWidth = new YState<number>(doc, 'maxWidth', 800);
  const pageLayout = new YState<PageLayoutSettings | undefined>(doc, 'experimental_pageLayout', undefined);
  const pageEnabled = new YState<boolean>(doc, 'experimental_pageEnabled', false);

  const isPageLayoutEnabled = $derived(app.preference.current.experimental_pageEnabled && pageEnabled.current);
</script>

<ToolbarDropdownButton label="설정" placement="bottom-end" size="small">
  {#snippet anchor({ opened })}
    <ToolbarIcon style={css.raw({ transform: opened ? 'rotate(90deg)' : 'rotate(0deg)' })} icon={SettingsIcon} />
  {/snippet}

  {#snippet floating()}
    <div
      class={flex({
        flexDirection: 'column',
        gap: '8px',
        padding: '16px',
      })}
    >
      {#if app.preference.current.experimental_pageEnabled}
        <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={FileTextIcon} />
            <div class={css({ fontSize: '13px', color: 'text.subtle' })}>레이아웃 모드</div>
          </div>
          <div class={css({ width: '140px' })}>
            <SegmentButtons
              items={[
                { label: '스크롤', value: false },
                { label: '페이지', value: true },
              ]}
              onselect={(value) => {
                pageEnabled.current = value;
                if (value && !pageLayout.current) {
                  pageLayout.current = createDefaultPageLayout('a4');
                }
                mixpanel.track('toggle_post_page_layout', {
                  enabled: value,
                });
              }}
              size="sm"
              value={pageEnabled.current}
            />
          </div>
        </div>
      {/if}

      {#if isPageLayoutEnabled && pageLayout.current}
        <div class={flex({ flexDirection: 'column', gap: '12px', marginTop: '4px', marginBottom: '8px' })}>
          <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
            <div class={css({ fontSize: '12px', color: 'text.subtle', marginLeft: '28px' })}>용지 크기</div>
            <Select
              items={PAGE_LAYOUT_OPTIONS}
              onselect={(value: PageLayoutSize) => {
                if (pageLayout.current && value) {
                  pageLayout.current = {
                    size: value,
                    margins: DEFAULT_PAGE_MARGINS[value],
                  };
                }
              }}
              value={pageLayout.current?.size ?? 'a4'}
            />
          </div>

          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <div class={css({ fontSize: '12px', color: 'text.subtle', marginLeft: '28px' })}>여백 (mm)</div>
            <div class={grid({ columns: 2, gap: '8px', marginLeft: '28px' })}>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <div class={css({ fontSize: '11px', color: 'text.muted' })}>상</div>
                <input
                  class={css({
                    width: 'full',
                    paddingX: '8px',
                    paddingY: '4px',
                    fontSize: '12px',
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    borderRadius: '4px',
                    backgroundColor: 'surface.default',
                  })}
                  max="100"
                  min="0"
                  onchange={(e) => {
                    const target = e.target as HTMLInputElement;
                    if (pageLayout.current) {
                      pageLayout.current = {
                        ...pageLayout.current,
                        margins: { ...pageLayout.current.margins, top: Number(target.value) },
                      };
                    }
                  }}
                  type="number"
                  value={pageLayout.current?.margins.top ?? 25}
                />
              </div>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <div class={css({ fontSize: '11px', color: 'text.muted' })}>하</div>
                <input
                  class={css({
                    width: 'full',
                    paddingX: '8px',
                    paddingY: '4px',
                    fontSize: '12px',
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    borderRadius: '4px',
                    backgroundColor: 'surface.default',
                  })}
                  max="100"
                  min="0"
                  onchange={(e) => {
                    const target = e.target as HTMLInputElement;
                    if (pageLayout.current) {
                      pageLayout.current = {
                        ...pageLayout.current,
                        margins: { ...pageLayout.current.margins, bottom: Number(target.value) },
                      };
                    }
                  }}
                  type="number"
                  value={pageLayout.current?.margins.bottom ?? 25}
                />
              </div>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <div class={css({ fontSize: '11px', color: 'text.muted' })}>좌</div>
                <input
                  class={css({
                    width: 'full',
                    paddingX: '8px',
                    paddingY: '4px',
                    fontSize: '12px',
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    borderRadius: '4px',
                    backgroundColor: 'surface.default',
                  })}
                  max="100"
                  min="0"
                  onchange={(e) => {
                    const target = e.target as HTMLInputElement;
                    if (pageLayout.current) {
                      pageLayout.current = {
                        ...pageLayout.current,
                        margins: { ...pageLayout.current.margins, left: Number(target.value) },
                      };
                    }
                  }}
                  type="number"
                  value={pageLayout.current?.margins.left ?? 25}
                />
              </div>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <div class={css({ fontSize: '11px', color: 'text.muted' })}>우</div>
                <input
                  class={css({
                    width: 'full',
                    paddingX: '8px',
                    paddingY: '4px',
                    fontSize: '12px',
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    borderRadius: '4px',
                    backgroundColor: 'surface.default',
                  })}
                  max="100"
                  min="0"
                  onchange={(e) => {
                    const target = e.target as HTMLInputElement;
                    if (pageLayout.current) {
                      pageLayout.current = {
                        ...pageLayout.current,
                        margins: { ...pageLayout.current.margins, right: Number(target.value) },
                      };
                    }
                  }}
                  type="number"
                  value={pageLayout.current?.margins.right ?? 25}
                />
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if !isPageLayoutEnabled || !pageLayout.current}
        <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={RulerDimensionLineIcon} />
            <div class={css({ fontSize: '13px', color: 'text.subtle' })}>본문 폭</div>
          </div>
          <div class={css({ width: '200px' })}>
            <SegmentButtons
              items={[
                { label: '600px', value: 600 },
                { label: '800px', value: 800 },
                { label: '1000px', value: 1000 },
              ]}
              onselect={(value) => {
                maxWidth.current = value;
              }}
              size="sm"
              value={maxWidth.current ?? 800}
            />
          </div>
        </div>
      {/if}

      <HorizontalDivider style={css.raw({ marginY: '12px' })} />

      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={ArrowRightToLineIcon} />
          <div class={css({ fontSize: '13px', color: 'text.subtle' })}>첫 줄 들여쓰기</div>
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
              editor?.current.chain().focus().setBodyParagraphIndent(value).run();
            }}
            size="sm"
            value={editor?.current.state.doc.firstChild?.attrs.paragraphIndent}
          />
        </div>
      </div>

      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={AlignVerticalSpaceAroundIcon} />
          <div class={css({ fontSize: '13px', color: 'text.subtle' })}>문단 사이 간격</div>
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
              editor?.current.chain().focus().setBodyBlockGap(value).run();
            }}
            size="sm"
            value={editor?.current.state.doc.firstChild?.attrs.blockGap}
          />
        </div>
      </div>

      <HorizontalDivider style={css.raw({ marginY: '12px' })} />

      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={TypeIcon} />
          <div class={css({ fontSize: '13px', color: 'text.subtle' })}>타자기 모드</div>
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
        <div class={flex({ width: 'full', align: 'center', gap: '16px' })}>
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

      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px', marginTop: '8px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={HighlighterIcon} />
          <div class={css({ fontSize: '13px', color: 'text.subtle' })}>현재 줄 강조</div>
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
  {/snippet}
</ToolbarDropdownButton>
