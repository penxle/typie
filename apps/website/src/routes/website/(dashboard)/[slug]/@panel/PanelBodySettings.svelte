<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, SegmentButtons, Select, Slider, Switch, TextInput, Tooltip } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog } from '@typie/ui/notification';
  import { getIncompatibleBlocks } from '@typie/ui/tiptap';
  import { clamp, createDefaultPageLayout, getMaxMargin, PAGE_LAYOUT_OPTIONS, PAGE_SIZE_MAP } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { PostLayoutMode } from '@/enums';
  import AlignVerticalSpaceAroundIcon from '~icons/lucide/align-vertical-space-around';
  import ArrowRightToLineIcon from '~icons/lucide/arrow-right-to-line';
  import ChevronsDownUpIcon from '~icons/lucide/chevrons-down-up';
  import CodeIcon from '~icons/lucide/code';
  import CodeXmlIcon from '~icons/lucide/code-xml';
  import FileIcon from '~icons/lucide/file';
  import FileTextIcon from '~icons/lucide/file-text';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import HighlighterIcon from '~icons/lucide/highlighter';
  import InfoIcon from '~icons/lucide/info';
  import QuoteIcon from '~icons/lucide/quote';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import TableIcon from '~icons/lucide/table';
  import TypeIcon from '~icons/lucide/type';
  import { YState } from '../state.svelte';
  import type { Editor } from '@tiptap/core';
  import type { PageLayout, PageLayoutPreset, Ref } from '@typie/ui/utils';
  import type * as Y from 'yjs';

  type Props = {
    editor?: Ref<Editor>;
    doc: Y.Doc;
  };

  let { editor, doc }: Props = $props();

  const app = getAppContext();
  const maxWidth = new YState<number>(doc, 'maxWidth', 800);
  const pageLayout = new YState<PageLayout | undefined>(doc, 'pageLayout', undefined);
  const layoutMode = new YState<PostLayoutMode>(doc, 'layoutMode', PostLayoutMode.SCROLL);

  const isPageLayoutEnabled = $derived(layoutMode.current === PostLayoutMode.PAGE);

  const getBlockInfo = (blockType: string) => {
    const blockInfo: Record<string, { name: string; icon: typeof QuoteIcon }> = {
      blockquote: { name: '인용구', icon: QuoteIcon },
      callout: { name: '강조', icon: GalleryVerticalEndIcon },
      fold: { name: '접기', icon: ChevronsDownUpIcon },
      table: { name: '표', icon: TableIcon },
      code_block: { name: '코드', icon: CodeIcon },
      html_block: { name: 'HTML', icon: CodeXmlIcon },
    };
    return blockInfo[blockType] || { name: blockType, icon: FileTextIcon };
  };

  let capturedIncompatibleBlocks = $state<string[]>([]);

  const handlePageLayoutToggle = (value: PostLayoutMode) => {
    if (!editor?.current) {
      return;
    }

    const incompatibleBlocks = getIncompatibleBlocks(editor.current);
    capturedIncompatibleBlocks = incompatibleBlocks;

    if (value === PostLayoutMode.PAGE && incompatibleBlocks.length > 0) {
      Dialog.confirm({
        title: '페이지 모드 전환',
        message: '페이지 모드에서는 일부 블록을 지원하지 않아요.',
        children: pageLayoutToggleConfirmView,
        action: 'primary',
        actionLabel: '모두 해제하고 전환',
        cancelLabel: '취소',
        actionHandler: () => {
          if (!editor?.current) {
            return;
          }

          editor.current.chain().focus().convertIncompatibleBlocks().run();
          layoutMode.current = value;

          if (value === PostLayoutMode.PAGE && !pageLayout.current) {
            pageLayout.current = createDefaultPageLayout('a4');
          }

          mixpanel.track('toggle_post_page_layout', {
            enabled: value,
          });
        },
      });
    } else {
      layoutMode.current = value;
      if (value === PostLayoutMode.PAGE && !pageLayout.current) {
        pageLayout.current = createDefaultPageLayout('a4');
      }
      mixpanel.track('toggle_post_page_layout', {
        enabled: value,
      });
    }
  };
</script>

{#snippet pageLayoutToggleConfirmView()}
  <div class={css({ fontSize: '15px' })}>
    <div class={css({ marginBottom: '12px', color: 'text.subtle' })}>다음 블록들을 해제해서 일반 문단으로 변환할까요?</div>

    <div
      class={css({
        padding: '12px',
        backgroundColor: 'surface.subtle',
        borderRadius: '6px',
      })}
    >
      <div class={flex({ flexDirection: 'column', gap: '8px' })}>
        {#each capturedIncompatibleBlocks as blockType (blockType)}
          {@const blockInfo = getBlockInfo(blockType)}
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ width: '16px', height: '16px', color: 'text.default', fontWeight: 'semibold' })} icon={blockInfo.icon} />
            <span class={css({ color: 'text.default', fontWeight: 'medium' })}>{blockInfo.name}</span>
          </div>
        {/each}
      </div>
    </div>
  </div>
{/snippet}

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
      height: '40px',
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
            { label: '스크롤', value: PostLayoutMode.SCROLL },
            { label: '페이지', value: PostLayoutMode.PAGE },
          ]}
          onselect={handlePageLayoutToggle}
          size="sm"
          value={layoutMode.current}
        />
      </div>
    </div>

    {#if isPageLayoutEnabled && pageLayout.current}
      <div class={flex({ flexDirection: 'column', gap: '6px', paddingX: '20px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} />
          <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>페이지 크기 (mm)</div>
        </div>
        <Select
          items={PAGE_LAYOUT_OPTIONS}
          onselect={(value: PageLayoutPreset | 'custom') => {
            if (pageLayout.current && value !== 'custom') {
              pageLayout.current = createDefaultPageLayout(value);
            }
          }}
          value={(Object.entries(PAGE_SIZE_MAP).find(
            ([, dimension]) => dimension.width === pageLayout.current?.width && dimension.height === pageLayout.current?.height,
          )?.[0] as PageLayoutPreset) ?? ('custom' as const)}
        />
        <div class={flex({ flexDirection: 'column', gap: '8px' })}>
          <div class={grid({ columns: 2, columnGap: '12px', rowGap: '8px', paddingLeft: '8px' })}>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>너비</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                min="100"
                onchange={(e) => {
                  if (!pageLayout.current) return;
                  const target = e.target as HTMLInputElement;
                  const value = Math.max(100, Number(target.value));
                  target.value = String(value);
                  pageLayout.current = {
                    ...pageLayout.current,
                    width: value,
                  };
                }}
                size="sm"
                type="number"
                value={pageLayout.current.width}
              />
            </div>
            <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
              <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>높이</div>
              <TextInput
                style={css.raw({ width: '80px' })}
                min="100"
                onchange={(e) => {
                  if (!pageLayout.current) return;
                  const target = e.target as HTMLInputElement;
                  const value = Math.max(100, Number(target.value));
                  target.value = String(value);
                  pageLayout.current = {
                    ...pageLayout.current,
                    height: value,
                  };
                }}
                size="sm"
                type="number"
                value={pageLayout.current.height}
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
            <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>상단</div>
            <TextInput
              style={css.raw({ width: '80px' })}
              max={pageLayout.current ? String(getMaxMargin('top', pageLayout.current)) : undefined}
              min="0"
              oninput={(e) => {
                if (!pageLayout.current) return;
                const target = e.target as HTMLInputElement;
                const value = clamp(Number(target.value), 0, getMaxMargin('top', pageLayout.current));
                target.value = String(value);
                pageLayout.current = {
                  ...pageLayout.current,
                  marginTop: value,
                };
              }}
              size="sm"
              type="number"
              value={pageLayout.current?.marginTop ?? 25}
            />
          </div>
          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>하단</div>
            <TextInput
              style={css.raw({ width: '80px' })}
              max={pageLayout.current ? String(getMaxMargin('bottom', pageLayout.current)) : undefined}
              min="0"
              oninput={(e) => {
                if (!pageLayout.current) return;
                const target = e.target as HTMLInputElement;
                const value = clamp(Number(target.value), 0, getMaxMargin('bottom', pageLayout.current));
                target.value = String(value);
                pageLayout.current = {
                  ...pageLayout.current,
                  marginBottom: value,
                };
              }}
              size="sm"
              type="number"
              value={pageLayout.current?.marginBottom ?? 25}
            />
          </div>
          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>왼쪽</div>
            <TextInput
              style={css.raw({ width: '80px' })}
              max={pageLayout.current ? String(getMaxMargin('left', pageLayout.current)) : undefined}
              min="0"
              onchange={(e) => {
                if (!pageLayout.current) return;
                const target = e.target as HTMLInputElement;
                const value = clamp(Number(target.value), 0, getMaxMargin('left', pageLayout.current));
                target.value = String(value);
                pageLayout.current = {
                  ...pageLayout.current,
                  marginLeft: value,
                };
              }}
              size="sm"
              type="number"
              value={pageLayout.current?.marginLeft ?? 25}
            />
          </div>
          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>오른쪽</div>
            <TextInput
              style={css.raw({ width: '80px' })}
              max={pageLayout.current ? String(getMaxMargin('right', pageLayout.current)) : undefined}
              min="0"
              oninput={(e) => {
                if (!pageLayout.current) return;
                const target = e.target as HTMLInputElement;
                const value = clamp(Number(target.value), 0, getMaxMargin('right', pageLayout.current));
                target.value = String(value);
                pageLayout.current = {
                  ...pageLayout.current,
                  marginRight: value,
                };
              }}
              size="sm"
              type="number"
              value={pageLayout.current?.marginRight ?? 25}
            />
          </div>
        </div>
      </div>
    {/if}

    {#if !isPageLayoutEnabled || !pageLayout.current}
      <div class={flex({ flexDirection: 'column', gap: '8px', paddingX: '20px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={RulerDimensionLineIcon} />
          <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>본문 폭</div>
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
            editor?.current.chain().focus().setBodyParagraphIndent(value).run();
          }}
          size="sm"
          value={editor?.current.state.doc.firstChild?.attrs.paragraphIndent}
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
            editor?.current.chain().focus().setBodyBlockGap(value).run();
          }}
          size="sm"
          value={editor?.current.state.doc.firstChild?.attrs.blockGap}
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
