<script lang="ts">
  import AlignVerticalSpaceAroundIcon from '~icons/lucide/align-vertical-space-around';
  import ArrowRightToLineIcon from '~icons/lucide/arrow-right-to-line';
  import BoldIcon from '~icons/lucide/bold';
  import ChevronsDownUpIcon from '~icons/lucide/chevrons-down-up';
  import CodeIcon from '~icons/lucide/code';
  import CodeXmlIcon from '~icons/lucide/code-xml';
  import FileUpIcon from '~icons/lucide/file-up';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import ImageIcon from '~icons/lucide/image';
  import ItalicIcon from '~icons/lucide/italic';
  import LinkIcon from '~icons/lucide/link';
  import ListIcon from '~icons/lucide/list';
  import ListOrderedIcon from '~icons/lucide/list-ordered';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import QuoteIcon from '~icons/lucide/quote';
  import RedoIcon from '~icons/lucide/redo';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import SettingsIcon from '~icons/lucide/settings';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import TableIcon from '~icons/lucide/table';
  import UnderlineIcon from '~icons/lucide/underline';
  import UndoIcon from '~icons/lucide/undo';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import RubyIcon from '~icons/typie/ruby';
  import { fragment, graphql } from '$graphql';
  import { HorizontalDivider, Icon, SegmentButtons, VerticalDivider } from '$lib/components';
  import { defaultValues, values } from '$lib/tiptap/values';
  import { css } from '$styled-system/css';
  import { center, flex, grid } from '$styled-system/patterns';
  import { token } from '$styled-system/tokens';
  import Spellcheck from './Spellcheck.svelte';
  import { YState } from './state.svelte';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarDropdownMenu from './ToolbarDropdownMenu.svelte';
  import ToolbarDropdownMenuItem from './ToolbarDropdownMenuItem.svelte';
  import ToolbarFloatingLink from './ToolbarFloatingLink.svelte';
  import ToolbarFloatingRuby from './ToolbarFloatingRuby.svelte';
  import ToolbarFontFamily from './ToolbarFontFamily.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import type { Editor } from '@tiptap/core';
  import type * as Y from 'yjs';
  import type { Editor_Toolbar_site, Optional } from '$graphql';
  import type { Ref } from '$lib/utils';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    $site?: Optional<Editor_Toolbar_site>;
    editor?: Ref<Editor>;
    doc: Y.Doc;
    style?: SystemStyleObject;
  };

  let { $site: _site, editor, doc, style }: Props = $props();

  const site = fragment(
    _site,
    graphql(`
      fragment Editor_Toolbar_site on Site {
        id

        user {
          id

          subscription {
            id
          }
        }

        ...Editor_Toolbar_FontFamily_site
      }
    `),
  );

  const maxWidth = new YState<number>(doc, 'maxWidth', 800);
</script>

<div
  class={css(
    {
      display: 'flex',
      flexDirection: 'column',
      gap: '8px',
      borderBottomWidth: '1px',
      borderColor: 'border.subtle',
      paddingY: '8px',
      zIndex: '10',
    },
    style,
  )}
  role="toolbar"
  tabindex="-1"
>
  <div class={flex({ alignItems: 'center', gap: '4px', paddingX: '16px' })}>
    <ToolbarButton
      disabled={!editor?.current.can().setImage()}
      icon={ImageIcon}
      label="이미지"
      onclick={() => {
        editor?.current.chain().focus().setImage().run();
      }}
      size="large"
    />

    <ToolbarButton
      disabled={!editor?.current.can().setFile()}
      icon={PaperclipIcon}
      label="파일"
      onclick={() => {
        editor?.current.chain().focus().setFile().run();
      }}
      size="large"
    />

    <ToolbarButton
      disabled={!editor?.current.can().setEmbed()}
      icon={FileUpIcon}
      label="임베드"
      onclick={() => {
        editor?.current.chain().focus().setEmbed().run();
      }}
      size="large"
    />

    <ToolbarDropdownButton
      active={editor?.current.isActive('horizontal_rule')}
      disabled={!editor?.current.can().setHorizontalRule()}
      label="구분선"
      size="large"
    >
      {#snippet anchor()}
        <ToolbarIcon icon={HorizontalRuleIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarDropdownMenu style={css.raw({ maxWidth: '200px' })}>
          {#each values.horizontalRule as { type, component: Component } (type)}
            <ToolbarDropdownMenuItem
              style={css.raw({ justifyContent: 'center', height: '48px' })}
              onclick={() => {
                editor?.current.chain().focus().setHorizontalRule(type).run();
                close();
              }}
            >
              <Component />
            </ToolbarDropdownMenuItem>
          {/each}
        </ToolbarDropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton
      active={editor?.current.isActive('blockquote')}
      disabled={!editor?.current.can().toggleBlockquote()}
      label="인용구"
      size="large"
    >
      {#snippet anchor()}
        <ToolbarIcon icon={QuoteIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarDropdownMenu style={css.raw({ maxWidth: '200px' })}>
          {#each values.blockquote as { type, component: Component } (type)}
            <ToolbarDropdownMenuItem
              style={css.raw({ height: '48px' })}
              onclick={() => {
                editor?.current.chain().focus().toggleBlockquote(type).run();
                close();
              }}
            >
              <Component />
            </ToolbarDropdownMenuItem>
          {/each}
        </ToolbarDropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarButton
      disabled={!editor?.current.can().toggleCallout()}
      icon={GalleryVerticalEndIcon}
      label="콜아웃"
      onclick={() => {
        editor?.current.chain().focus().toggleCallout().run();
      }}
      size="large"
    />

    <ToolbarButton
      disabled={!editor?.current.can().toggleFold()}
      icon={ChevronsDownUpIcon}
      label="폴드"
      onclick={() => {
        editor?.current.chain().focus().toggleFold().run();
      }}
      size="large"
    />

    <ToolbarButton
      disabled={!editor?.current.can().insertTable()}
      icon={TableIcon}
      label="표"
      onclick={() => {
        editor?.current.chain().focus().insertTable().run();
      }}
      size="large"
    />

    <ToolbarDropdownButton
      disabled={!editor?.current || (!editor.current.can().toggleBulletList() && !editor.current.can().toggleOrderedList())}
      label="목록"
      size="large"
    >
      {#snippet anchor()}
        <ToolbarIcon icon={ListIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarDropdownMenu>
          <ToolbarDropdownMenuItem
            onclick={() => {
              editor?.current.chain().focus().toggleBulletList().run();
              close();
            }}
          >
            <div class={flex({ alignItems: 'center', gap: '4px' })}>
              <ToolbarIcon icon={ListIcon} />
              순서 없는 목록
            </div>
          </ToolbarDropdownMenuItem>

          <ToolbarDropdownMenuItem
            onclick={() => {
              editor?.current.chain().focus().toggleOrderedList().run();
              close();
            }}
          >
            <div class={flex({ alignItems: 'center', gap: '4px' })}>
              <ToolbarIcon icon={ListOrderedIcon} />
              순서 있는 목록
            </div>
          </ToolbarDropdownMenuItem>
        </ToolbarDropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarButton
      disabled={!editor?.current.can().setCodeBlock()}
      icon={CodeIcon}
      label="코드"
      onclick={() => {
        editor?.current.chain().focus().setCodeBlock().run();
      }}
      size="large"
    />

    <ToolbarButton
      disabled={!editor?.current.can().setHtmlBlock()}
      icon={CodeXmlIcon}
      label="HTML"
      onclick={() => {
        editor?.current.chain().focus().setHtmlBlock().run();
      }}
      size="large"
    />

    <div class={css({ flexGrow: '1' })}></div>

    {#if editor && $site?.user.subscription}
      <Spellcheck {editor} />
    {/if}
  </div>

  <HorizontalDivider color="secondary" />

  <div class={flex({ alignItems: 'center', gap: '10px', paddingLeft: '20px', paddingRight: '12px' })}>
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <ToolbarButton
        style={css.raw({ borderRightRadius: '0' })}
        disabled={!editor?.current.can().undo()}
        icon={UndoIcon}
        label="실행 취소"
        onclick={() => {
          editor?.current.chain().focus().undo().run();
        }}
        size="small"
      />

      <ToolbarButton
        style={css.raw({ borderLeftRadius: '0' })}
        disabled={!editor?.current.can().redo()}
        icon={RedoIcon}
        label="다시 실행"
        onclick={() => {
          editor?.current.chain().focus().redo().run();
        }}
        size="small"
      />
    </div>

    <VerticalDivider style={css.raw({ height: '12px' })} />

    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <ToolbarDropdownButton
        chevron
        disabled={!editor?.current.can().setTextColor(defaultValues.textColor)}
        label="글씨 색"
        placement="bottom-start"
        size="small"
      >
        {#snippet anchor()}
          <div class={center({ size: '20px' })}>
            <div
              style:background-color={values.textColor.find(
                ({ value }) => value === (editor?.current.getAttributes('text_style').textColor ?? defaultValues.textColor),
              )?.color}
              class={css({ borderWidth: '1px', borderRadius: 'full', size: '16px' })}
            ></div>
          </div>
        {/snippet}

        {#snippet floating({ close })}
          <div class={grid({ columns: 10, gap: '8px', padding: '8px' })}>
            {#each values.textColor as { label, value, color } (value)}
              <button
                style:background-color={color}
                style:outline-color={value === 'white' ? token('colors.border.default') : color}
                class={center({
                  borderWidth: '1px',
                  borderRadius: 'full',
                  outlineWidth: (editor?.current.getAttributes('text_style').textColor ?? defaultValues.textColor) === value ? '2px' : '0',
                  outlineOffset: '1px',
                  size: '20px',
                })}
                aria-label={label}
                onclick={() => {
                  editor?.current.chain().focus().setTextColor(value).run();
                  close();
                }}
                type="button"
              ></button>
            {/each}
          </div>
        {/snippet}
      </ToolbarDropdownButton>

      <ToolbarDropdownButton
        chevron
        disabled={!editor?.current.can().setTextBackgroundColor('gray')}
        label="배경색"
        placement="bottom-start"
        size="small"
      >
        {#snippet anchor()}
          {@const selectedValue = editor?.current.getAttributes('text_style').textBackgroundColor ?? defaultValues.textBackgroundColor}
          {@const selectedItem = values.textBackgroundColor.find(({ value }) => value === selectedValue)}
          <div class={center({ size: '20px' })}>
            <div
              style:background-color={selectedValue === 'none' ? 'transparent' : selectedItem?.color}
              class={css({
                borderWidth: '1px',
                borderRadius: '4px',
                size: '16px',
                position: 'relative',
              })}
            >
              {#if selectedValue === 'none'}
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
          </div>
        {/snippet}

        {#snippet floating({ close })}
          <div class={grid({ columns: 8, gap: '8px', padding: '8px' })}>
            {#each values.textBackgroundColor as { label, value, color } (value)}
              <button
                style:background-color={value === 'none' ? 'transparent' : color}
                style:outline-color={value === 'none' ? token('colors.border.default') : color}
                class={center({
                  borderWidth: '1px',
                  borderRadius: '4px',
                  outlineWidth:
                    (editor?.current.getAttributes('text_style').textBackgroundColor ?? defaultValues.textBackgroundColor) === value
                      ? '2px'
                      : '0',
                  outlineOffset: '1px',
                  size: '20px',
                  position: 'relative',
                })}
                aria-label={label}
                onclick={() => {
                  editor?.current.chain().focus().setTextBackgroundColor(value).run();
                  close();
                }}
                type="button"
              >
                {#if value === 'none'}
                  <div
                    class={css({
                      position: 'absolute',
                      inset: '0',
                      margin: 'auto',
                      width: '1px',
                      height: '14px',
                      backgroundColor: 'text.disabled',
                      transform: 'rotate(45deg)',
                    })}
                  ></div>
                {/if}
              </button>
            {/each}
          </div>
        {/snippet}
      </ToolbarDropdownButton>

      <ToolbarFontFamily {$site} {editor} />

      <ToolbarDropdownButton
        style={css.raw({ width: '60px' })}
        chevron
        disabled={!editor?.current.can().setFontSize(defaultValues.fontSize)}
        label="글씨 크기"
        size="small"
      >
        {#snippet anchor()}
          <div class={css({ flexGrow: '1', fontSize: '14px', color: 'text.subtle' })}>
            {values.fontSize.find(({ value }) => value === (editor?.current.getAttributes('text_style').fontSize ?? defaultValues.fontSize))
              ?.label}
          </div>
        {/snippet}

        {#snippet floating({ close })}
          <ToolbarDropdownMenu>
            {#each values.fontSize as { label, value } (value)}
              <ToolbarDropdownMenuItem
                active={(editor?.current.getAttributes('text_style').fontSize ?? defaultValues.fontSize) === value}
                onclick={() => {
                  editor?.current.chain().focus().setFontSize(value).run();
                  close();
                }}
              >
                {label}
              </ToolbarDropdownMenuItem>
            {/each}
          </ToolbarDropdownMenu>
        {/snippet}
      </ToolbarDropdownButton>
    </div>

    <VerticalDivider style={css.raw({ height: '12px' })} />

    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <ToolbarButton
        active={editor?.current.isActive('bold')}
        disabled={!editor?.current.can().toggleBold()}
        icon={BoldIcon}
        label="굵게"
        onclick={() => {
          editor?.current.chain().focus().toggleBold().run();
        }}
        size="small"
      />

      <ToolbarButton
        active={editor?.current.isActive('italic')}
        disabled={!editor?.current.can().toggleItalic()}
        icon={ItalicIcon}
        label="기울임"
        onclick={() => {
          editor?.current.chain().focus().toggleItalic().run();
        }}
        size="small"
      />

      <ToolbarButton
        active={editor?.current.isActive('strike')}
        disabled={!editor?.current.can().toggleStrike()}
        icon={StrikethroughIcon}
        label="취소선"
        onclick={() => {
          editor?.current.chain().focus().toggleStrike().run();
        }}
        size="small"
      />

      <ToolbarButton
        active={editor?.current.isActive('underline')}
        disabled={!editor?.current.can().toggleUnderline()}
        icon={UnderlineIcon}
        label="밑줄"
        onclick={() => {
          editor?.current.chain().focus().toggleUnderline().run();
        }}
        size="small"
      />
    </div>

    <VerticalDivider style={css.raw({ height: '12px' })} />

    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <ToolbarDropdownButton
        active={editor?.current.isActive('link')}
        disabled={!editor?.current.can().setLink('')}
        label="링크"
        size="small"
      >
        {#snippet anchor()}
          <ToolbarIcon icon={LinkIcon} />
        {/snippet}

        {#snippet floating({ close })}
          {#if editor}
            <ToolbarFloatingLink {close} {editor} />
          {/if}
        {/snippet}
      </ToolbarDropdownButton>

      <ToolbarDropdownButton
        active={editor?.current.isActive('ruby')}
        disabled={!editor?.current.can().setRuby('')}
        label="루비"
        size="small"
      >
        {#snippet anchor()}
          <ToolbarIcon icon={RubyIcon} />
        {/snippet}

        {#snippet floating({ close })}
          {#if editor}
            <ToolbarFloatingRuby {close} {editor} />
          {/if}
        {/snippet}
      </ToolbarDropdownButton>
    </div>

    <VerticalDivider style={css.raw({ height: '12px' })} />

    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <ToolbarDropdownButton
        disabled={!editor?.current.can().setParagraphTextAlign(defaultValues.textAlign)}
        label="문단 정렬"
        size="small"
      >
        {#snippet anchor()}
          <ToolbarIcon
            icon={// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            values.textAlign.find(
              ({ value }) => value === (editor?.current.getAttributes('paragraph').textAlign ?? defaultValues.textAlign),
            )!.icon}
          />
        {/snippet}

        {#snippet floating({ close })}
          <ToolbarDropdownMenu>
            {#each values.textAlign as { label, value } (value)}
              <ToolbarDropdownMenuItem
                style={css.raw({ fontSize: '14px' })}
                active={(editor?.current.getAttributes('paragraph').textAlign ?? defaultValues.textAlign) === value}
                onclick={() => {
                  editor?.current.chain().focus().setParagraphTextAlign(value).run();
                  close();
                }}
              >
                {label}
              </ToolbarDropdownMenuItem>
            {/each}
          </ToolbarDropdownMenu>
        {/snippet}
      </ToolbarDropdownButton>

      <ToolbarDropdownButton
        disabled={!editor?.current.can().setParagraphLineHeight(defaultValues.lineHeight)}
        label="문단 행간"
        size="small"
      >
        {#snippet anchor()}
          <ToolbarIcon icon={LineHeightIcon} />
        {/snippet}

        {#snippet floating({ close })}
          <ToolbarDropdownMenu>
            {#each values.lineHeight as { label, value } (value)}
              <ToolbarDropdownMenuItem
                style={css.raw({ fontSize: '14px' })}
                active={(editor?.current.getAttributes('paragraph').lineHeight ?? defaultValues.lineHeight) === value}
                onclick={() => {
                  editor?.current.chain().focus().setParagraphLineHeight(value).run();
                  close();
                }}
              >
                {label}
              </ToolbarDropdownMenuItem>
            {/each}
          </ToolbarDropdownMenu>
        {/snippet}
      </ToolbarDropdownButton>

      <ToolbarDropdownButton
        disabled={!editor?.current.can().setParagraphLetterSpacing(defaultValues.letterSpacing)}
        label="문단 자간"
        size="small"
      >
        {#snippet anchor()}
          <ToolbarIcon icon={LetterSpacingIcon} />
        {/snippet}

        {#snippet floating({ close })}
          <ToolbarDropdownMenu>
            {#each values.letterSpacing as { label, value } (value)}
              <ToolbarDropdownMenuItem
                style={css.raw({ fontSize: '14px' })}
                active={(editor?.current.getAttributes('paragraph').letterSpacing ?? defaultValues.letterSpacing) === value}
                onclick={() => {
                  editor?.current.chain().focus().setParagraphLetterSpacing(value).run();
                  close();
                }}
              >
                {label}
              </ToolbarDropdownMenuItem>
            {/each}
          </ToolbarDropdownMenu>
        {/snippet}
      </ToolbarDropdownButton>
    </div>

    <div class={css({ flexGrow: '1' })}></div>

    <ToolbarDropdownButton label="본문 설정" placement="bottom-end" size="small">
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
        </div>
      {/snippet}
    </ToolbarDropdownButton>
  </div>
</div>
