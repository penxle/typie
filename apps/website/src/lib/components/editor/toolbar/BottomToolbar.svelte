<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { DropdownMenu, DropdownMenuItem, VerticalDivider } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
  import AlignCenterIcon from '~icons/lucide/align-center';
  import AlignJustifyIcon from '~icons/lucide/align-justify';
  import AlignLeftIcon from '~icons/lucide/align-left';
  import AlignRightIcon from '~icons/lucide/align-right';
  import BoldIcon from '~icons/lucide/bold';
  import ItalicIcon from '~icons/lucide/italic';
  import LinkIcon from '~icons/lucide/link';
  import RedoIcon from '~icons/lucide/redo';
  import RemoveFormattingIcon from '~icons/lucide/remove-formatting';
  import SearchIcon from '~icons/lucide/search';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import UnderlineIcon from '~icons/lucide/underline';
  import UndoIcon from '~icons/lucide/undo';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import RubyIcon from '~icons/typie/ruby';
  import { getEditor } from '$lib/editor/context';
  import { THEME_COLORS } from '$lib/editor/theme';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarColorGrid from './ToolbarColorGrid.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarFontFamily from './ToolbarFontFamily.svelte';
  import ToolbarFontSize from './ToolbarFontSize.svelte';
  import ToolbarFontWeight from './ToolbarFontWeight.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import ToolbarLink from './ToolbarLink.svelte';
  import ToolbarRuby from './ToolbarRuby.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { ThemeVariant } from '$lib/editor/theme';
  import type { Mark, MarkType, TextAlign } from '$lib/editor/types';

  type Props = {
    style?: SystemStyleObject;
    onSearchClick?: () => void;
  };

  let { style, onSearchClick }: Props = $props();

  const app = getAppContext();
  const theme = getThemeContext();
  const editor = getEditor();

  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );

  const activeMarks = $derived(editor.activeMarks);
  const selection = $derived(editor.selection);

  const findMark = (type: string): Mark | undefined => activeMarks.uniformMarks.find((m) => m.type === type);
  const isMixed = (type: MarkType): boolean => activeMarks.mixedMarks.includes(type);
  const hasMark = (type: string): boolean => activeMarks.uniformMarks.some((m) => m.type === type);

  const defaultTextColor = 'black';
  const defaultTextBackgroundColor = 'none';
  const defaultLineHeight = 1.6;
  const defaultLetterSpacing = 0;
  const defaultTextAlign: TextAlign = 'left';

  const tc = $derived(THEME_COLORS[themeVariant]);

  const textColors = $derived([
    { label: '블랙', value: 'black', color: tc['text.black'] },
    { label: '다크 그레이', value: 'darkgray', color: tc['text.darkgray'] },
    { label: '그레이', value: 'gray', color: tc['text.gray'] },
    { label: '라이트 그레이', value: 'lightgray', color: tc['text.lightgray'] },
    { label: '화이트', value: 'white', color: tc['text.white'] },
    { label: '레드', value: 'red', color: tc['text.red'] },
    { label: '오렌지', value: 'orange', color: tc['text.orange'] },
    { label: '앰버', value: 'amber', color: tc['text.amber'] },
    { label: '옐로', value: 'yellow', color: tc['text.yellow'] },
    { label: '라임', value: 'lime', color: tc['text.lime'] },
    { label: '그린', value: 'green', color: tc['text.green'] },
    { label: '에메랄드', value: 'emerald', color: tc['text.emerald'] },
    { label: '틸', value: 'teal', color: tc['text.teal'] },
    { label: '시안', value: 'cyan', color: tc['text.cyan'] },
    { label: '스카이', value: 'sky', color: tc['text.sky'] },
    { label: '블루', value: 'blue', color: tc['text.blue'] },
    { label: '인디고', value: 'indigo', color: tc['text.indigo'] },
    { label: '바이올렛', value: 'violet', color: tc['text.violet'] },
    { label: '퍼플', value: 'purple', color: tc['text.purple'] },
    { label: '마젠타', value: 'fuchsia', color: tc['text.fuchsia'] },
    { label: '핑크', value: 'pink', color: tc['text.pink'] },
    { label: '로즈', value: 'rose', color: tc['text.rose'] },
  ]);

  const textBackgroundColors = $derived([
    { label: '배경 없음', value: null, color: null },
    { label: '그레이', value: 'gray', color: tc['bg.gray'] },
    { label: '레드', value: 'red', color: tc['bg.red'] },
    { label: '오렌지', value: 'orange', color: tc['bg.orange'] },
    { label: '옐로', value: 'yellow', color: tc['bg.yellow'] },
    { label: '그린', value: 'green', color: tc['bg.green'] },
    { label: '블루', value: 'blue', color: tc['bg.blue'] },
    { label: '퍼플', value: 'purple', color: tc['bg.purple'] },
  ]);

  const lineHeights = [
    { label: '80%', value: 0.8 },
    { label: '100%', value: 1 },
    { label: '120%', value: 1.2 },
    { label: '140%', value: 1.4 },
    { label: '160%', value: 1.6 },
    { label: '180%', value: 1.8 },
    { label: '200%', value: 2 },
    { label: '220%', value: 2.2 },
  ];

  const letterSpacings = [
    { label: '-10%', value: -0.1 },
    { label: '-5%', value: -0.05 },
    { label: '0%', value: 0 },
    { label: '5%', value: 0.05 },
    { label: '10%', value: 0.1 },
    { label: '20%', value: 0.2 },
    { label: '40%', value: 0.4 },
  ];

  const textAligns: { label: string; value: TextAlign; icon: typeof AlignLeftIcon }[] = [
    { label: '왼쪽 정렬', value: 'left', icon: AlignLeftIcon },
    { label: '가운데 정렬', value: 'center', icon: AlignCenterIcon },
    { label: '오른쪽 정렬', value: 'right', icon: AlignRightIcon },
    { label: '양쪽 정렬', value: 'justify', icon: AlignJustifyIcon },
  ];

  const currentTextColor = $derived(
    isMixed('text_color') ? defaultTextColor : ((findMark('text_color') as { key?: string })?.key ?? defaultTextColor),
  );
  const currentTextBackgroundColor = $derived(
    isMixed('background_color')
      ? defaultTextBackgroundColor
      : ((findMark('background_color') as { key?: string })?.key ?? defaultTextBackgroundColor),
  );
  const currentLineHeight = $derived(selection.stats.uniformLineHeight ?? defaultLineHeight);
  const currentLetterSpacing = $derived(
    isMixed('letter_spacing')
      ? defaultLetterSpacing
      : ((findMark('letter_spacing') as { spacing?: number })?.spacing ?? defaultLetterSpacing),
  );
  const currentTextAlign = $derived(selection.stats.uniformAlign ?? defaultTextAlign);

  const selectedFontWeight = $derived(
    isMixed('font_weight') ? undefined : ((findMark('font_weight') as { weight?: number })?.weight ?? 400),
  );
  const isBoldActive = $derived(selectedFontWeight !== undefined && selectedFontWeight >= 700);
  const isItalicActive = $derived(hasMark('italic'));
  const isStrikethroughActive = $derived(hasMark('strikethrough'));
  const isUnderlineActive = $derived(hasMark('underline'));
  const isLinkActive = $derived(hasMark('link'));
  const isRubyActive = $derived(hasMark('ruby'));

  const currentTextAlignIcon = $derived(textAligns.find((a) => a.value === currentTextAlign)?.icon ?? AlignLeftIcon);
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
      disabled={!editor.can('undo')}
      icon={UndoIcon}
      label="실행 취소"
      onclick={() => {
        editor.focus().dispatch({ type: 'undo' });
      }}
      size="small"
    />

    <ToolbarButton
      style={css.raw({ borderLeftRadius: '0' })}
      disabled={!editor.can('redo')}
      icon={RedoIcon}
      label="다시 실행"
      onclick={() => {
        editor.focus().dispatch({ type: 'redo' });
      }}
      size="small"
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarDropdownButton
      chevron
      disabled={!editor.can('toggleTextColor')}
      label="글씨 색"
      onEscape={() => editor.focus()}
      placement="bottom-start"
      size="small"
    >
      {#snippet anchor()}
        <div class={center({ size: '20px' })}>
          <div
            style:background-color={textColors.find(({ value }) => value === currentTextColor)?.color}
            class={css({ borderWidth: '1px', borderRadius: 'full', size: '16px' })}
          ></div>
        </div>
      {/snippet}

      {#snippet floating({ close, opened })}
        <ToolbarColorGrid
          columns={11}
          currentValue={currentTextColor}
          items={textColors}
          onClose={close}
          onSelect={(value) => {
            editor.focus().dispatch({ type: 'toggleTextColor', key: value });
          }}
          {opened}
        />
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton
      chevron
      disabled={!editor.can('toggleBackgroundColor')}
      label="배경색"
      onEscape={() => editor.focus()}
      placement="bottom-start"
      size="small"
    >
      {#snippet anchor()}
        {@const selectedValue = currentTextBackgroundColor}
        {@const selectedItem = textBackgroundColors.find(({ value }) => value === selectedValue)}
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

      {#snippet floating({ close, opened })}
        <ToolbarColorGrid
          columns={8}
          currentValue={currentTextBackgroundColor}
          items={textBackgroundColors}
          onClose={close}
          onSelect={(value) => {
            editor.focus().dispatch({ type: 'toggleBackgroundColor', key: value ?? undefined });
          }}
          {opened}
          shape="square"
          showNone
        />
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarFontFamily />
    <ToolbarFontWeight />
    <ToolbarFontSize />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarButton
      active={isBoldActive}
      disabled={!editor.can('toggleBold')}
      icon={BoldIcon}
      keys={['Mod', 'B']}
      label="굵게"
      onclick={() => {
        editor.focus().dispatch({ type: 'toggleBold' });
      }}
      size="small"
    />

    <ToolbarButton
      active={isItalicActive}
      disabled={!editor.can('toggleItalic')}
      icon={ItalicIcon}
      keys={['Mod', 'I']}
      label="기울임"
      onclick={() => {
        editor.focus().dispatch({ type: 'toggleItalic' });
      }}
      size="small"
    />

    <ToolbarButton
      active={isStrikethroughActive}
      disabled={!editor.can('toggleStrikethrough')}
      icon={StrikethroughIcon}
      keys={['Mod', 'Shift', 'S']}
      label="취소선"
      onclick={() => {
        editor.focus().dispatch({ type: 'toggleStrikethrough' });
      }}
      size="small"
    />

    <ToolbarButton
      active={isUnderlineActive}
      disabled={!editor.can('toggleUnderline')}
      icon={UnderlineIcon}
      keys={['Mod', 'U']}
      label="밑줄"
      onclick={() => {
        editor.focus().dispatch({ type: 'toggleUnderline' });
      }}
      size="small"
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarDropdownButton
      active={isLinkActive}
      disabled={!(editor.can('toggleLink') || isLinkActive)}
      label="링크"
      onEscape={() => editor.focus()}
      onOpenChange={(opened) => {
        if (opened && isLinkActive) {
          editor.dispatch({ type: 'extendMarkRange', markType: 'link' });
        }
      }}
      size="small"
    >
      {#snippet anchor()}
        <ToolbarIcon icon={LinkIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarLink {close} />
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton
      active={isRubyActive}
      disabled={!(editor.can('toggleRuby') || isRubyActive)}
      label="루비"
      onEscape={() => editor.focus()}
      onOpenChange={(opened) => {
        if (opened && isRubyActive) {
          editor.dispatch({ type: 'extendMarkRange', markType: 'ruby' });
        }
      }}
      size="small"
    >
      {#snippet anchor()}
        <ToolbarIcon icon={RubyIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarRuby {close} />
      {/snippet}
    </ToolbarDropdownButton>
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarDropdownButton disabled={!editor.can('setTextAlign')} label="문단 정렬" onEscape={() => editor.focus()} size="small">
      {#snippet anchor()}
        <ToolbarIcon icon={currentTextAlignIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu>
          {#each textAligns as { label, value } (value)}
            <DropdownMenuItem
              style={css.raw({ fontSize: '14px' })}
              active={currentTextAlign === value}
              onclick={() => {
                editor.focus().dispatch({ type: 'setTextAlign', align: value });
                close();
              }}
            >
              {label}
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton disabled={!editor.can('setLineHeight')} label="문단 행간" onEscape={() => editor.focus()} size="small">
      {#snippet anchor()}
        <ToolbarIcon icon={LineHeightIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu>
          {#each lineHeights as { label, value } (value)}
            <DropdownMenuItem
              style={css.raw({ fontSize: '14px' })}
              active={currentLineHeight === value}
              onclick={() => {
                editor.focus().dispatch({ type: 'setLineHeight', height: value });
                close();
              }}
            >
              {label}
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton disabled={!editor.can('setLetterSpacing')} label="문단 자간" onEscape={() => editor.focus()} size="small">
      {#snippet anchor()}
        <ToolbarIcon icon={LetterSpacingIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu>
          {#each letterSpacings as { label, value } (value)}
            <DropdownMenuItem
              style={css.raw({ fontSize: '14px' })}
              active={Math.abs(currentLetterSpacing - value) < 0.001}
              onclick={() => {
                editor.focus().dispatch({ type: 'setLetterSpacing', spacing: value });
                close();
              }}
            >
              {label}
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <ToolbarButton
    disabled={!editor.can('clearFormatting')}
    icon={RemoveFormattingIcon}
    keys={['Mod', '\\']}
    label="서식 해제"
    onclick={() => {
      editor.focus().dispatch({ type: 'clearFormatting' });
    }}
    size="small"
  />

  <div class={css({ flexGrow: '1' })}></div>

  <ToolbarButton icon={SearchIcon} keys={['Mod', 'F']} label="찾기, 바꾸기" onclick={() => onSearchClick?.()} size="small" />
</div>
