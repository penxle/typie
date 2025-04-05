<script lang="ts">
  import BoldIcon from '~icons/lucide/bold';
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
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import TableIcon from '~icons/lucide/table';
  import UnderlineIcon from '~icons/lucide/underline';
  import UndoIcon from '~icons/lucide/undo';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import RubyIcon from '~icons/typie/ruby';
  import { HorizontalDivider, VerticalDivider } from '$lib/components';
  import { defaultValues, values } from '$lib/tiptap/values';
  import { css } from '$styled-system/css';
  import { center, flex, grid } from '$styled-system/patterns';
  import { token } from '$styled-system/tokens';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarDropdownMenu from './ToolbarDropdownMenu.svelte';
  import ToolbarDropdownMenuItem from './ToolbarDropdownMenuItem.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor: Ref<Editor>;
  };

  let { editor }: Props = $props();
</script>

<div class={flex({ direction: 'column', gap: '8px', width: 'full', maxWidth: '1000px' })}>
  <div class={flex({ alignItems: 'center', gap: '8px' })}>
    <ToolbarButton
      icon={ImageIcon}
      label="이미지"
      onclick={() => {
        editor.current.chain().focus().setImage().run();
      }}
      size="large"
    />

    <ToolbarButton
      icon={PaperclipIcon}
      label="파일"
      onclick={() => {
        editor.current.chain().focus().setFile().run();
      }}
      size="large"
    />

    <ToolbarButton
      icon={FileUpIcon}
      label="임베드"
      onclick={() => {
        editor.current.chain().focus().setEmbed().run();
      }}
      size="large"
    />

    <ToolbarDropdownButton label="구분선" size="large">
      {#snippet anchor()}
        <ToolbarIcon icon={HorizontalRuleIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarDropdownMenu style={css.raw({ maxWidth: '200px' })}>
          {#each values.horizontalRule as { type, component: Component } (type)}
            <ToolbarDropdownMenuItem
              style={css.raw({ justifyContent: 'center', height: '48px' })}
              onclick={() => {
                editor.current.chain().focus().setHorizontalRule(type).run();
                close();
              }}
            >
              <Component />
            </ToolbarDropdownMenuItem>
          {/each}
        </ToolbarDropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton label="인용구" size="large">
      {#snippet anchor()}
        <ToolbarIcon icon={QuoteIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarDropdownMenu style={css.raw({ maxWidth: '200px' })}>
          {#each values.blockquote as { type, component: Component } (type)}
            <ToolbarDropdownMenuItem
              style={css.raw({ height: '48px' })}
              onclick={() => {
                editor.current.chain().focus().setBlockquote(type).run();
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
        editor.current.chain().focus().setCallout().run();
      }}
      size="large"
    />

    <ToolbarButton
      icon={TableIcon}
      label="표"
      onclick={() => {
        editor.current.chain().focus().insertTable().run();
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
              editor.current.chain().focus().toggleBulletList().run();
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
              editor.current.chain().focus().toggleOrderedList().run();
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
      label="코드 블록"
      onclick={() => {
        editor.current.chain().focus().setCodeBlock().run();
      }}
      size="large"
    />

    <ToolbarButton
      icon={CodeXmlIcon}
      label="HTML 블록"
      onclick={() => {
        editor.current.chain().focus().setHtmlBlock().run();
      }}
      size="large"
    />
  </div>

  <HorizontalDivider />

  <div class={flex({ alignItems: 'center', gap: '8px' })}>
    <ToolbarButton
      icon={UndoIcon}
      label="실행 취소"
      onclick={() => {
        editor.current.chain().focus().undo().run();
      }}
      size="small"
    />

    <ToolbarButton
      icon={RedoIcon}
      label="다시 실행"
      onclick={() => {
        editor.current.chain().focus().redo().run();
      }}
      size="small"
    />

    <VerticalDivider />

    <ToolbarDropdownButton chevron label="글씨 색" size="small">
      {#snippet anchor()}
        <div
          style:background-color={values.fontColor.find(
            ({ value }) => value === (editor.current.getAttributes('font_color').value ?? defaultValues.fontColor),
          )?.hex}
          class={css({ borderWidth: '1px', borderRadius: 'full', size: '20px' })}
        ></div>
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
                size: '20px',
                outlineWidth: (editor.current.getAttributes('font_color').value ?? defaultValues.fontColor) === value ? '2px' : '0',
                outlineOffset: '1px',
              })}
              aria-label={label}
              onclick={() => {
                editor.current.chain().focus().setFontColor(value).run();
                close();
              }}
              type="button"
            ></button>
          {/each}
        </div>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton chevron label="글씨 서체" size="small">
      {#snippet anchor()}
        <div class={css({ fontSize: '14px' })}>
          {values.fontFamily.find(({ value }) => value === (editor.current.getAttributes('font_family').value ?? defaultValues.fontFamily))
            ?.label}
        </div>
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarDropdownMenu>
          {#each values.fontFamily as { label, value } (value)}
            <ToolbarDropdownMenuItem
              style={css.raw({ fontSize: '14px' })}
              active={(editor.current.getAttributes('font_family').value ?? defaultValues.fontFamily) === value}
              onclick={() => {
                editor.current.chain().focus().setFontFamily(value).run();
                close();
              }}
            >
              <div style:font-family={value}>{label}</div>
            </ToolbarDropdownMenuItem>
          {/each}
        </ToolbarDropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton chevron label="글씨 크기" size="small">
      {#snippet anchor()}
        <div class={css({ fontSize: '14px' })}>
          {values.fontSize.find(({ value }) => value === (editor.current.getAttributes('font_size').value ?? defaultValues.fontSize))
            ?.label}
        </div>
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarDropdownMenu>
          {#each values.fontSize as { label, value } (value)}
            <ToolbarDropdownMenuItem
              style={css.raw({ fontSize: '14px' })}
              active={(editor.current.getAttributes('font_size').value ?? defaultValues.fontSize) === value}
              onclick={() => {
                editor.current.chain().focus().setFontSize(value).run();
                close();
              }}
            >
              {label}
            </ToolbarDropdownMenuItem>
          {/each}
        </ToolbarDropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <VerticalDivider />

    <ToolbarButton
      active={editor.current.isActive('bold')}
      icon={BoldIcon}
      label="굵게"
      onclick={() => {
        editor.current.chain().focus().toggleBold().run();
      }}
      size="small"
    />

    <ToolbarButton
      active={editor.current.isActive('italic')}
      icon={ItalicIcon}
      label="기울임"
      onclick={() => {
        editor.current.chain().focus().toggleItalic().run();
      }}
      size="small"
    />

    <ToolbarButton
      active={editor.current.isActive('strike')}
      icon={StrikethroughIcon}
      label="취소선"
      onclick={() => {
        editor.current.chain().focus().toggleStrike().run();
      }}
      size="small"
    />

    <ToolbarButton
      active={editor.current.isActive('underline')}
      icon={UnderlineIcon}
      label="밑줄"
      onclick={() => {
        editor.current.chain().focus().toggleUnderline().run();
      }}
      size="small"
    />

    <VerticalDivider />

    <ToolbarButton
      active={editor.current.isActive('link')}
      icon={LinkIcon}
      label="링크"
      onclick={() => {
        const url = prompt('링크를 입력하세요');
        if (url) {
          editor.current.chain().focus().setLink(url).run();
        } else {
          editor.current.chain().focus().unsetLink().run();
        }
      }}
      size="small"
    />

    <ToolbarButton
      active={editor.current.isActive('ruby')}
      icon={RubyIcon}
      label="루비"
      onclick={() => {
        const ruby = prompt('루비를 입력하세요');
        if (ruby) {
          editor.current.chain().focus().setRuby(ruby).run();
        } else {
          editor.current.chain().focus().unsetRuby().run();
        }
      }}
      size="small"
    />

    <VerticalDivider />

    <ToolbarDropdownButton label="문단 정렬" size="small">
      {#snippet anchor()}
        <ToolbarIcon
          icon={// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          values.textAlign.find(({ value }) => value === (editor.current.getAttributes('paragraph').textAlign ?? defaultValues.textAlign))!
            .icon}
        />
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarDropdownMenu>
          {#each values.textAlign as { label, value } (value)}
            <ToolbarDropdownMenuItem
              style={css.raw({ fontSize: '14px' })}
              active={(editor.current.getAttributes('paragraph').textAlign ?? defaultValues.textAlign) === value}
              onclick={() => {
                editor.current.chain().focus().setParagraphTextAlign(value).run();
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
              active={(editor.current.getAttributes('paragraph').lineHeight ?? defaultValues.lineHeight) === value}
              onclick={() => {
                editor.current.chain().focus().setParagraphLineHeight(value).run();
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
              active={(editor.current.getAttributes('paragraph').letterSpacing ?? defaultValues.letterSpacing) === value}
              onclick={() => {
                editor.current.chain().focus().setParagraphLetterSpacing(value).run();
                close();
              }}
            >
              {label}
            </ToolbarDropdownMenuItem>
          {/each}
        </ToolbarDropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <!--
    <select
      class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
      onchange={({ currentTarget }) =>
        editor.current
          .chain()
          .focus()
          .setBodyMaxWidth(Number(currentTarget.value) as never)
          .run()}
    >
      {#each values.maxWidth as { label, value } (value)}
        <option selected={(editor.current.getAttributes('body').maxWidth ?? defaultValues.maxWidth) === value} {value}>
          {label}
        </option>
      {/each}
    </select>

    <select
      class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
      onchange={({ currentTarget }) =>
        editor.current
          .chain()
          .focus()
          .setBodyBlockGap(Number(currentTarget.value) as never)
          .run()}
    >
      {#each values.blockGap as { label, value } (value)}
        <option selected={(editor.current.getAttributes('body').blockGap ?? defaultValues.blockGap) === value} {value}>
          {label}
        </option>
      {/each}
    </select>

    <select
      class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
      onchange={({ currentTarget }) =>
        editor.current
          .chain()
          .focus()
          .setBodyParagraphIndent(Number(currentTarget.value) as never)
          .run()}
    >
      {#each values.paragraphIndent as { label, value } (value)}
        <option selected={(editor.current.getAttributes('body').paragraphIndent ?? defaultValues.paragraphIndent) === value} {value}>
          {label}
        </option>
      {/each}
    </select> -->
  </div>

  <HorizontalDivider />
</div>
