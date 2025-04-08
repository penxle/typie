<script lang="ts">
  import BoldIcon from '~icons/lucide/bold';
  import ChevronsDownIcon from '~icons/lucide/chevrons-down';
  import ChevronsUpIcon from '~icons/lucide/chevrons-up';
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
  import SettingsIcon from '~icons/lucide/settings';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import TableIcon from '~icons/lucide/table';
  import UnderlineIcon from '~icons/lucide/underline';
  import UndoIcon from '~icons/lucide/undo';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import RubyIcon from '~icons/typie/ruby';
  import { SegmentButtons, VerticalDivider } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { defaultValues, values } from '$lib/tiptap/values';
  import { css } from '$styled-system/css';
  import { center, flex, grid } from '$styled-system/patterns';
  import { token } from '$styled-system/tokens';
  import { YState } from './state.svelte';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarDropdownMenu from './ToolbarDropdownMenu.svelte';
  import ToolbarDropdownMenuItem from './ToolbarDropdownMenuItem.svelte';
  import ToolbarFloatingLink from './ToolbarFloatingLink.svelte';
  import ToolbarFloatingRuby from './ToolbarFloatingRuby.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import type { Editor } from '@tiptap/core';
  import type * as Y from 'yjs';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
    doc: Y.Doc;
    sticked: boolean;
  };

  let { editor, doc, sticked }: Props = $props();

  const app = getAppContext();
  const maxWidth = new YState<number>(doc, 'maxWidth', 1000);
</script>

<div
  class={css(
    {
      display: 'flex',
      flexDirection: 'column',
      gap: '14px',
      borderRadius: '12px',
      marginX: '20px',
      paddingX: '14px',
      paddingY: '12px',
      width: 'full',
      maxWidth: '1200px',
      backgroundColor: 'gray.50',
      boxShadow: '[0px 0px 1px 0px rgba(80, 84, 90, 0.08), 0px 1px 4px 0px rgba(80, 84, 90, 0.08), 0px 2px 8px 0px rgba(80, 84, 90, 0.12)]',
      pointerEvents: 'auto',
      transitionProperty: 'transform',
      transitionDuration: '200ms',
      transitionTimingFunction: 'ease',
      _hover: { transform: 'translateY(0)' },
    },
    app.preference.current.toolbarHidden && { transform: 'translateY(-90%)', willChange: 'transform' },
    sticked ? { borderTopRadius: '0' } : { transform: 'translateY(0)' },
    app.state.toolbarActive && { transform: 'translateY(0)' },
  )}
  role="toolbar"
  tabindex="-1"
>
  <div class={flex({ alignItems: 'center', gap: '8px', width: 'full' })}>
    <ToolbarButton
      icon={ImageIcon}
      label="이미지"
      onclick={() => {
        editor?.current.chain().focus().setImage().run();
      }}
      size="large"
    />

    <ToolbarButton
      icon={PaperclipIcon}
      label="파일"
      onclick={() => {
        editor?.current.chain().focus().setFile().run();
      }}
      size="large"
    />

    <ToolbarButton
      icon={FileUpIcon}
      label="임베드"
      onclick={() => {
        editor?.current.chain().focus().setEmbed().run();
      }}
      size="large"
    />

    <ToolbarDropdownButton active={editor?.current.isActive('horizontal_rule')} label="구분선" size="large">
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

    <ToolbarDropdownButton active={editor?.current.isActive('blockquote')} label="인용구" size="large">
      {#snippet anchor()}
        <ToolbarIcon icon={QuoteIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarDropdownMenu style={css.raw({ maxWidth: '200px' })}>
          {#each values.blockquote as { type, component: Component } (type)}
            <ToolbarDropdownMenuItem
              style={css.raw({ height: '48px' })}
              onclick={() => {
                editor?.current.chain().focus().setBlockquote(type).run();
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
      icon={GalleryVerticalEndIcon}
      label="콜아웃"
      onclick={() => {
        editor?.current.chain().focus().setCallout().run();
      }}
      size="large"
    />

    <ToolbarButton
      icon={TableIcon}
      label="표"
      onclick={() => {
        editor?.current.chain().focus().insertTable().run();
      }}
      size="large"
    />

    <ToolbarDropdownButton label="목록" size="large">
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
              <div class={css({ fontSize: '14px' })}>순서 없는 목록</div>
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
              <div class={css({ fontSize: '14px' })}>순서 있는 목록</div>
            </div>
          </ToolbarDropdownMenuItem>
        </ToolbarDropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarButton
      icon={CodeIcon}
      label="코드"
      onclick={() => {
        editor?.current.chain().focus().setCodeBlock().run();
      }}
      size="large"
    />

    <ToolbarButton
      icon={CodeXmlIcon}
      label="HTML"
      onclick={() => {
        editor?.current.chain().focus().setHtmlBlock().run();
      }}
      size="large"
    />

    <div class={css({ flexGrow: '1' })}></div>

    <div class={css({ alignSelf: 'flex-start' })}>
      <ToolbarButton
        style={css.raw({ backgroundColor: 'transparent' })}
        icon={app.preference.current.toolbarHidden ? ChevronsDownIcon : ChevronsUpIcon}
        label={app.preference.current.toolbarHidden ? '툴바 고정하기' : '툴바 접기'}
        onclick={() => {
          app.preference.current.toolbarHidden = !app.preference.current.toolbarHidden;
        }}
        size="small"
      />
    </div>
  </div>

  <div class={flex({ alignItems: 'center', gap: '10px', width: 'full', maxWidth: '1200px' })}>
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <ToolbarButton
        style={css.raw({ borderRightRadius: '0' })}
        icon={UndoIcon}
        label="실행 취소"
        onclick={() => {
          editor?.current.chain().focus().undo().run();
        }}
        size="small"
      />

      <ToolbarButton
        style={css.raw({ borderLeftRadius: '0' })}
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
      <ToolbarDropdownButton chevron label="글씨 색" size="small">
        {#snippet anchor()}
          <div class={center({ size: '20px' })}>
            <div
              style:background-color={values.fontColor.find(
                ({ value }) => value === (editor?.current.getAttributes('font_color').value ?? defaultValues.fontColor),
              )?.hex}
              class={css({ borderWidth: '1px', borderRadius: 'full', size: '16px' })}
            ></div>
          </div>
        {/snippet}

        {#snippet floating({ close })}
          <div class={grid({ columns: 10, gap: '8px', borderWidth: '1px', borderRadius: '4px', padding: '8px' })}>
            {#each values.fontColor as { label, value, hex } (value)}
              <button
                style:background-color={hex}
                style:outline-color={hex === '#ffffff' ? token('colors.gray.200') : hex}
                class={center({
                  borderWidth: '1px',
                  borderRadius: 'full',
                  outlineWidth: (editor?.current.getAttributes('font_color').value ?? defaultValues.fontColor) === value ? '2px' : '0',
                  outlineOffset: '1px',
                  size: '20px',
                })}
                aria-label={label}
                onclick={() => {
                  editor?.current.chain().focus().setFontColor(value).run();
                  close();
                }}
                type="button"
              ></button>
            {/each}
          </div>
        {/snippet}
      </ToolbarDropdownButton>

      <ToolbarDropdownButton style={css.raw({ width: '120px' })} chevron label="글씨 서체" size="small">
        {#snippet anchor()}
          <div class={css({ flexGrow: '1', fontSize: '14px', fontWeight: 'medium' })}>
            {values.fontFamily.find(
              ({ value }) => value === (editor?.current.getAttributes('font_family').value ?? defaultValues.fontFamily),
            )?.label}
          </div>
        {/snippet}

        {#snippet floating({ close })}
          <ToolbarDropdownMenu>
            {#each values.fontFamily as { label, value } (value)}
              <ToolbarDropdownMenuItem
                style={css.raw({ fontSize: '14px' })}
                active={(editor?.current.getAttributes('font_family').value ?? defaultValues.fontFamily) === value}
                onclick={() => {
                  editor?.current.chain().focus().setFontFamily(value).run();
                  close();
                }}
              >
                <div style:font-family={value}>{label}</div>
              </ToolbarDropdownMenuItem>
            {/each}
          </ToolbarDropdownMenu>
        {/snippet}
      </ToolbarDropdownButton>

      <ToolbarDropdownButton style={css.raw({ width: '60px' })} chevron label="글씨 크기" size="small">
        {#snippet anchor()}
          <div class={css({ flexGrow: '1', fontSize: '14px', fontWeight: 'medium' })}>
            {values.fontSize.find(({ value }) => value === (editor?.current.getAttributes('font_size').value ?? defaultValues.fontSize))
              ?.label}
          </div>
        {/snippet}

        {#snippet floating({ close })}
          <ToolbarDropdownMenu>
            {#each values.fontSize as { label, value } (value)}
              <ToolbarDropdownMenuItem
                style={css.raw({ fontSize: '14px' })}
                active={(editor?.current.getAttributes('font_size').value ?? defaultValues.fontSize) === value}
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
        icon={BoldIcon}
        label="굵게"
        onclick={() => {
          editor?.current.chain().focus().toggleBold().run();
        }}
        size="small"
      />

      <ToolbarButton
        active={editor?.current.isActive('italic')}
        icon={ItalicIcon}
        label="기울임"
        onclick={() => {
          editor?.current.chain().focus().toggleItalic().run();
        }}
        size="small"
      />

      <ToolbarButton
        active={editor?.current.isActive('strike')}
        icon={StrikethroughIcon}
        label="취소선"
        onclick={() => {
          editor?.current.chain().focus().toggleStrike().run();
        }}
        size="small"
      />

      <ToolbarButton
        active={editor?.current.isActive('underline')}
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
      <ToolbarDropdownButton active={editor?.current.isActive('link')} label="링크" size="small">
        {#snippet anchor()}
          <ToolbarIcon icon={LinkIcon} />
        {/snippet}

        {#snippet floating({ close })}
          {#if editor}
            <ToolbarFloatingLink {close} {editor} />
          {/if}
        {/snippet}
      </ToolbarDropdownButton>

      <ToolbarDropdownButton active={editor?.current.isActive('ruby')} label="루비" size="small">
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
      <ToolbarDropdownButton label="문단 정렬" size="small">
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

      <ToolbarDropdownButton label="문단 행간" size="small">
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

      <ToolbarDropdownButton label="문단 자간" size="small">
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
      {#snippet anchor()}
        <ToolbarIcon icon={SettingsIcon} />
      {/snippet}

      {#snippet floating()}
        <div
          class={flex({
            flexDirection: 'column',
            gap: '16px',
            borderWidth: '1px',
            borderRadius: '4px',
            padding: '16px',
          })}
        >
          <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px' })}>
            <div class={css({ fontSize: '13px', fontWeight: 'medium' })}>본문 폭</div>
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
                value={maxWidth.current ?? 1000}
              />
            </div>
          </div>

          <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px' })}>
            <div class={css({ fontSize: '13px', fontWeight: 'medium' })}>문단 들여쓰기</div>
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

          <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '16px' })}>
            <div class={css({ fontSize: '13px', fontWeight: 'medium' })}>문단 사이 간격</div>
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
