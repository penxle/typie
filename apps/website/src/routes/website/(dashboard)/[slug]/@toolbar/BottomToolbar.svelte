<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex, grid } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { VerticalDivider } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { defaultValues, values } from '@typie/ui/tiptap';
  import BoldIcon from '~icons/lucide/bold';
  import ItalicIcon from '~icons/lucide/italic';
  import LinkIcon from '~icons/lucide/link';
  import RedoIcon from '~icons/lucide/redo';
  import SearchIcon from '~icons/lucide/search';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import UnderlineIcon from '~icons/lucide/underline';
  import UndoIcon from '~icons/lucide/undo';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import RubyIcon from '~icons/typie/ruby';
  import { fragment, graphql } from '$graphql';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarDropdownMenu from './ToolbarDropdownMenu.svelte';
  import ToolbarDropdownMenuItem from './ToolbarDropdownMenuItem.svelte';
  import ToolbarFloatingLink from './ToolbarFloatingLink.svelte';
  import ToolbarFloatingRuby from './ToolbarFloatingRuby.svelte';
  import ToolbarFontFamily from './ToolbarFontFamily.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import type { Editor } from '@tiptap/core';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Ref } from '@typie/ui/utils';
  import type * as Y from 'yjs';
  import type { Editor_BottomToolbar_site, Optional } from '$graphql';

  type Props = {
    $site?: Optional<Editor_BottomToolbar_site>;
    editor?: Ref<Editor>;
    undoManager: Y.UndoManager;
    style?: SystemStyleObject;
  };

  let { $site: _site, editor, undoManager, style }: Props = $props();

  const site = fragment(
    _site,
    graphql(`
      fragment Editor_BottomToolbar_site on Site {
        id

        user {
          id

          subscription {
            id
          }
        }

        ...Editor_BottomToolbar_FontFamily_site
      }
    `),
  );

  const app = getAppContext();

  let canUndo = $state(false);
  let canRedo = $state(false);

  $effect(() => {
    const handler = () => {
      canUndo = undoManager.canUndo();
      canRedo = undoManager.canRedo();
    };

    undoManager.on('stack-item-added', handler);
    undoManager.on('stack-item-popped', handler);
    undoManager.on('stack-item-updated', handler);
    undoManager.on('stack-cleared', handler);

    return () => {
      undoManager.off('stack-item-added', handler);
      undoManager.off('stack-item-popped', handler);
      undoManager.off('stack-item-updated', handler);
      undoManager.off('stack-cleared', handler);
    };
  });
</script>

<div
  class={css(
    {
      display: 'flex',
      flexShrink: '0',
      alignItems: 'center',
      gap: '10px',
      paddingLeft: '20px',
      paddingRight: '12px',
      paddingY: '8px',
      overflowX: 'auto',
      scrollbarWidth: '[thin]',
      borderBottomWidth: '1px',
      borderColor: 'border.subtle',
      position: 'relative',
      zIndex: app.preference.current.zenModeEnabled ? 'underEditor' : 'overEditor',
      backgroundColor: 'surface.default',
    },
    style,
  )}
  role="toolbar"
  tabindex="-1"
>
  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarButton
      style={css.raw({ borderRightRadius: '0' })}
      disabled={!canUndo}
      icon={UndoIcon}
      label="실행 취소"
      onclick={() => {
        undoManager.undo();
      }}
      size="small"
    />

    <ToolbarButton
      style={css.raw({ borderLeftRadius: '0' })}
      disabled={!canRedo}
      icon={RedoIcon}
      label="다시 실행"
      onclick={() => {
        undoManager.redo();
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
      disabled={!editor?.current.can().setTextBackgroundColor(defaultValues.textBackgroundColor)}
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
      keys={['Mod', 'B']}
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
      keys={['Mod', 'I']}
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
      keys={['Mod', 'Shift', 'S']}
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
      keys={['Mod', 'U']}
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
    <ToolbarDropdownButton disabled={!editor?.current.can().setParagraphTextAlign(defaultValues.textAlign)} label="문단 정렬" size="small">
      {#snippet anchor()}
        <ToolbarIcon
          icon={// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          values.textAlign.find(({ value }) => value === (editor?.current.getAttributes('paragraph').textAlign ?? defaultValues.textAlign))!
            .icon}
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

  {#if editor}
    <ToolbarButton
      disabled={!editor.current}
      icon={SearchIcon}
      keys={['Mod', 'F']}
      label="찾기, 바꾸기"
      onclick={() => {
        app.state.findReplaceOpen = !app.state.findReplaceOpen;
      }}
      size="small"
    />
  {/if}
</div>
