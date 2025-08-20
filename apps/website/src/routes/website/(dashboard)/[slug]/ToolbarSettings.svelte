<script lang="ts">
  import { Fragment } from '@tiptap/pm/model';
  import { css } from '@typie/styled-system/css';
  import { center, flex, grid } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, SegmentButtons, Select, Slider, Switch, TextInput, Tooltip } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog } from '@typie/ui/notification';
  import {
    clamp,
    createDefaultPageLayout,
    DEFAULT_PAGE_MARGINS,
    getMaxMargin,
    INCOMPATIBLE_NODE_TYPES,
    PAGE_LAYOUT_OPTIONS,
  } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
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
  import PanelBottomDashedIcon from '~icons/lucide/panel-bottom-dashed';
  import PanelLeftDashedIcon from '~icons/lucide/panel-left-dashed';
  import PanelRightDashedIcon from '~icons/lucide/panel-right-dashed';
  import PanelTopDashedIcon from '~icons/lucide/panel-top-dashed';
  import QuoteIcon from '~icons/lucide/quote';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import SettingsIcon from '~icons/lucide/settings';
  import TableIcon from '~icons/lucide/table';
  import TypeIcon from '~icons/lucide/type';
  import { YState } from './state.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Node } from '@tiptap/pm/model';
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

  const getIncompatibleBlocks = () => {
    if (!editor?.current) return [];
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const types = new Set<string>();

    editor.current.state.doc.descendants((node) => {
      if (INCOMPATIBLE_NODE_TYPES.has(node.type.name)) {
        types.add(node.type.name);
      }
    });

    return [...types];
  };

  const getBlockInfo = (blockType: string) => {
    const blockInfo: Record<string, { name: string; icon: typeof QuoteIcon }> = {
      blockquote: { name: '인용구', icon: QuoteIcon },
      callout: { name: '콜아웃', icon: GalleryVerticalEndIcon },
      fold: { name: '폴드', icon: ChevronsDownUpIcon },
      table: { name: '표', icon: TableIcon },
      code_block: { name: '코드', icon: CodeIcon },
      html_block: { name: 'HTML', icon: CodeXmlIcon },
    };
    return blockInfo[blockType] || { name: blockType, icon: FileTextIcon };
  };

  const convertIncompatibleBlocks = () => {
    if (!editor?.current) return;

    editor.current
      .chain()
      .focus()
      .command(({ tr, state }) => {
        const paragraph = state.schema.nodes.paragraph;

        if (!paragraph) return false;

        let docChanged = true;

        while (docChanged) {
          docChanged = false;

          type NodeWithPos = { pos: number; node: Node; depth: number };
          const blocksToConvert: NodeWithPos[] = [];

          tr.doc.descendants((node, pos) => {
            if (INCOMPATIBLE_NODE_TYPES.has(node.type.name)) {
              const depth = tr.doc.resolve(pos).depth;
              blocksToConvert.push({ pos, node, depth });
              return false; // NOTE: 자식 노드를 순회하지 않음. 중첩된 노드는 다음 루프에서 처리됨
            }
            return true;
          });

          if (blocksToConvert.length === 0) break;

          // NOTE: 깊이가 깊은 것부터 처리
          blocksToConvert.sort((a, b) => b.depth - a.depth || b.pos - a.pos);

          blocksToConvert.forEach(({ pos, node }) => {
            docChanged = true;
            if (node.type.name === 'blockquote' || node.type.name === 'callout' || node.type.name === 'fold') {
              if (node.content.size > 0) {
                const slice = node.content;
                tr.replaceWith(pos, pos + node.nodeSize, slice);
              } else {
                tr.delete(pos, pos + node.nodeSize);
              }
            } else if (node.type.name === 'table') {
              // NOTE: 모든 셀의 내용을 나열
              const blocks: Node[] = [];

              node.descendants((child) => {
                if (child.type.name === 'table_cell' || child.type.name === 'table_header') {
                  child.content.forEach((contentNode) => {
                    blocks.push(contentNode);
                  });
                }
              });

              if (blocks.length > 0) {
                const fragment = Fragment.from(blocks);
                tr.replaceWith(pos, pos + node.nodeSize, fragment);
              } else {
                tr.delete(pos, pos + node.nodeSize);
              }
            } else if (node.type.name === 'code_block' || node.type.name === 'html_block') {
              const textContent = node.textContent;
              if (textContent) {
                const hardBreak = state.schema.nodes.hard_break;
                const lines = textContent.split('\n');
                const content: Node[] = [];

                lines.forEach((line, index) => {
                  if (index > 0 && hardBreak) {
                    content.push(hardBreak.create());
                  }
                  if (line) {
                    content.push(state.schema.text(line));
                  }
                });

                if (content.length > 0) {
                  const paragraphNode = paragraph.create(null, content);
                  tr.replaceWith(pos, pos + node.nodeSize, paragraphNode);
                } else {
                  tr.delete(pos, pos + node.nodeSize);
                }
              } else {
                tr.delete(pos, pos + node.nodeSize);
              }
            }
          });
        }

        return true;
      })
      .run();
  };

  let capturedIncompatibleBlocks = $state<string[]>([]);

  const handlePageLayoutToggle = (value: boolean) => {
    const incompatibleBlocks = getIncompatibleBlocks();
    capturedIncompatibleBlocks = incompatibleBlocks;

    if (value && incompatibleBlocks.length > 0) {
      Dialog.confirm({
        title: '페이지 모드 전환',
        message: '페이지 모드에서는 일부 블록을 지원하지 않아요.',
        children: pageLayoutToggleConfirmView,
        action: 'primary',
        actionLabel: '모두 해제하고 전환',
        cancelLabel: '취소',
        actionHandler: () => {
          convertIncompatibleBlocks();
          pageEnabled.current = value;
          if (value && !pageLayout.current) {
            pageLayout.current = createDefaultPageLayout('a4');
          }
          mixpanel.track('toggle_post_page_layout', {
            enabled: value,
          });
        },
      });
    } else {
      pageEnabled.current = value;
      if (value && !pageLayout.current) {
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
              onselect={handlePageLayoutToggle}
              size="sm"
              value={pageEnabled.current}
            />
          </div>
        </div>
      {/if}

      {#if isPageLayoutEnabled && pageLayout.current}
        <div class={flex({ flexDirection: 'column', gap: '12px', marginTop: '4px', marginBottom: '8px' })}>
          <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} />
              <div class={css({ fontSize: '13px', color: 'text.subtle' })}>페이지 크기</div>
            </div>
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
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'text.faint' })} icon={RulerDimensionLineIcon} />
              <div class={css({ fontSize: '13px', color: 'text.subtle' })}>여백 (mm)</div>
            </div>
            <div class={grid({ columns: 2, columnGap: '12px', rowGap: '8px', paddingLeft: '8px' })}>
              <div class={center({ gap: '8px' })}>
                <div class={center({ gap: '4px' })}>
                  <Icon style={css.raw({ width: '14px', height: '14px', color: 'text.subtle' })} icon={PanelTopDashedIcon} />
                  <div class={css({ fontSize: '12px', color: 'text.subtle' })}>상</div>
                </div>
                <TextInput
                  style={css.raw({ width: 'full' })}
                  max={pageLayout.current ? String(getMaxMargin('top', pageLayout.current.size, pageLayout.current.margins)) : undefined}
                  min="0"
                  oninput={(e) => {
                    if (!pageLayout.current) return;
                    const target = e.target as HTMLInputElement;
                    const value = clamp(Number(target.value), 0, getMaxMargin('top', pageLayout.current.size, pageLayout.current.margins));
                    target.value = String(value);
                    pageLayout.current = {
                      ...pageLayout.current,
                      margins: { ...pageLayout.current.margins, top: value },
                    };
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.current?.margins.top ?? 25}
                />
              </div>
              <div class={center({ gap: '8px' })}>
                <div class={center({ gap: '4px' })}>
                  <Icon style={css.raw({ width: '14px', height: '14px', color: 'text.subtle' })} icon={PanelBottomDashedIcon} />
                  <div class={css({ fontSize: '12px', color: 'text.subtle' })}>하</div>
                </div>
                <TextInput
                  style={css.raw({ width: 'full' })}
                  max={pageLayout.current ? String(getMaxMargin('bottom', pageLayout.current.size, pageLayout.current.margins)) : undefined}
                  min="0"
                  oninput={(e) => {
                    if (!pageLayout.current) return;
                    const target = e.target as HTMLInputElement;
                    const value = clamp(
                      Number(target.value),
                      0,
                      getMaxMargin('bottom', pageLayout.current.size, pageLayout.current.margins),
                    );
                    target.value = String(value);
                    pageLayout.current = {
                      ...pageLayout.current,
                      margins: { ...pageLayout.current.margins, bottom: value },
                    };
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.current?.margins.bottom ?? 25}
                />
              </div>
              <div class={center({ gap: '8px' })}>
                <div class={center({ gap: '4px' })}>
                  <Icon style={css.raw({ width: '14px', height: '14px', color: 'text.subtle' })} icon={PanelLeftDashedIcon} />
                  <div class={css({ fontSize: '12px', color: 'text.subtle' })}>좌</div>
                </div>
                <TextInput
                  style={css.raw({ width: 'full' })}
                  max={pageLayout.current ? String(getMaxMargin('left', pageLayout.current.size, pageLayout.current.margins)) : undefined}
                  min="0"
                  onchange={(e) => {
                    if (!pageLayout.current) return;
                    const target = e.target as HTMLInputElement;
                    const value = clamp(Number(target.value), 0, getMaxMargin('left', pageLayout.current.size, pageLayout.current.margins));
                    target.value = String(value);
                    pageLayout.current = {
                      ...pageLayout.current,
                      margins: { ...pageLayout.current.margins, left: value },
                    };
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.current?.margins.left ?? 25}
                />
              </div>
              <div class={center({ gap: '8px' })}>
                <div class={center({ gap: '4px' })}>
                  <Icon style={css.raw({ width: '14px', height: '14px', color: 'text.subtle' })} icon={PanelRightDashedIcon} />
                  <div class={css({ fontSize: '12px', color: 'text.subtle' })}>우</div>
                </div>
                <TextInput
                  style={css.raw({ width: 'full' })}
                  max={pageLayout.current ? String(getMaxMargin('right', pageLayout.current.size, pageLayout.current.margins)) : undefined}
                  min="0"
                  oninput={(e) => {
                    if (!pageLayout.current) return;
                    const target = e.target as HTMLInputElement;
                    const value = clamp(
                      Number(target.value),
                      0,
                      getMaxMargin('right', pageLayout.current.size, pageLayout.current.margins),
                    );
                    target.value = String(value);
                    pageLayout.current = {
                      ...pageLayout.current,
                      margins: { ...pageLayout.current.margins, right: value },
                    };
                  }}
                  size="sm"
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
