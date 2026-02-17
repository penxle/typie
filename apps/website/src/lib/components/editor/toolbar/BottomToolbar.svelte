<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { DropdownMenu, DropdownMenuItem, VerticalDivider } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
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
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { THEME_COLORS } from '$lib/editor/theme';
  import { values } from '$lib/editor/values';
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
  import type { TextAlign } from '$lib/editor/types';

  type Props = {
    style?: SystemStyleObject;
    onSearchClick?: () => void;
    onFontUploadClick?: () => void;
  };

  let { style, onSearchClick, onFontUploadClick }: Props = $props();

  const app = getAppContext();
  const theme = getThemeContext();
  const { editor } = getEditorContext();

  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );

  const tc = $derived(THEME_COLORS[themeVariant]);

  const textColors = $derived(values.textColor.map((c) => ({ label: c.label, value: c.value, color: tc[c.themeKey] })));

  const textBackgroundColors = $derived(
    values.textBackgroundColor.map((c) => ({ label: c.label, value: c.value, color: c.themeKey ? tc[c.themeKey] : null })),
  );

  const textColorAttr = $derived(editor.getAttr('text_color'));
  const textColorValues = $derived(textColorAttr?.values.filter((v): v is string => v != null) ?? []);
  const currentTextColor = $derived(textColorValues.length === 1 ? textColorValues[0] : undefined);

  const bgColorAttr = $derived(editor.getAttr('background_color'));
  const bgColorValues = $derived(bgColorAttr?.values.filter((v): v is string => v != null) ?? []);
  const currentTextBackgroundColor = $derived(bgColorValues.length === 1 ? bgColorValues[0] : undefined);

  const lineHeightAttr = $derived(editor.getAttr('line_height'));
  const lineHeightValues = $derived(lineHeightAttr?.values.filter((v): v is number => v != null) ?? []);
  const currentLineHeight = $derived(lineHeightValues.length === 1 ? lineHeightValues[0] : undefined);

  const letterSpacingAttr = $derived(editor.getAttr('letter_spacing'));
  const letterSpacingValues = $derived(letterSpacingAttr?.values.filter((v): v is number => v != null) ?? []);
  const currentLetterSpacing = $derived(letterSpacingValues.length === 1 ? letterSpacingValues[0] : undefined);

  const alignAttr = $derived(editor.getAttr('text_align'));
  const alignValues = $derived(alignAttr?.values.filter((v): v is TextAlign => v != null) ?? []);
  const currentTextAlign = $derived(alignValues.length === 1 ? alignValues[0] : undefined);

  const fontWeightAttr = $derived(editor.getAttr('font_weight'));
  const fontWeightValues = $derived(fontWeightAttr?.values.filter((v): v is number => v != null) ?? []);
  const selectedFontWeight = $derived(fontWeightValues.length === 1 ? fontWeightValues[0] : undefined);
  const isBoldActive = $derived(selectedFontWeight !== undefined && selectedFontWeight >= 700);
  const isItalicActive = $derived(editor.getAttr('italic')?.values.includes(null) === false);
  const isStrikethroughActive = $derived(editor.getAttr('strikethrough')?.values.includes(null) === false);
  const isUnderlineActive = $derived(editor.getAttr('underline')?.values.includes(null) === false);
  const linkValues = $derived(editor.getAttr('link')?.values ?? []);
  const isLinkActive = $derived(linkValues.length === 1 && linkValues[0] != null);
  const isLinkMixed = $derived(linkValues.length >= 2);
  const rubyValues = $derived(editor.getAttr('ruby')?.values ?? []);
  const isRubyActive = $derived(rubyValues.length === 1 && rubyValues[0] != null);
  const isRubyMixed = $derived(rubyValues.length >= 2);
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
        editor.focus().dispatch({ type: 'undo' }).scrollIntoView();
      }}
      size="small"
    />

    <ToolbarButton
      style={css.raw({ borderLeftRadius: '0' })}
      disabled={!editor.can('redo')}
      icon={RedoIcon}
      label="다시 실행"
      onclick={() => {
        editor.focus().dispatch({ type: 'redo' }).scrollIntoView();
      }}
      size="small"
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarDropdownButton
      chevron
      disabled={!editor.can('toggleStyle')}
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
            editor.focus().dispatch({ type: 'toggleStyle', style: { type: 'text_color', color: value } });
          }}
          {opened}
        />
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton
      chevron
      disabled={!editor.can('toggleStyle')}
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
            editor.focus().dispatch({ type: 'toggleStyle', style: { type: 'background_color', color: value } });
          }}
          {opened}
          shape="square"
        />
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarFontFamily onUploadClick={onFontUploadClick} />
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
      disabled={!editor.can('toggleStyle')}
      icon={ItalicIcon}
      keys={['Mod', 'I']}
      label="기울임"
      onclick={() => {
        editor.focus().dispatch({ type: 'toggleStyle', style: { type: 'italic' } });
      }}
      size="small"
    />

    <ToolbarButton
      active={isStrikethroughActive}
      disabled={!editor.can('toggleStyle')}
      icon={StrikethroughIcon}
      keys={['Mod', 'Shift', 'S']}
      label="취소선"
      onclick={() => {
        editor.focus().dispatch({ type: 'toggleStyle', style: { type: 'strikethrough' } });
      }}
      size="small"
    />

    <ToolbarButton
      active={isUnderlineActive}
      disabled={!editor.can('toggleStyle')}
      icon={UnderlineIcon}
      keys={['Mod', 'U']}
      label="밑줄"
      onclick={() => {
        editor.focus().dispatch({ type: 'toggleStyle', style: { type: 'underline' } });
      }}
      size="small"
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarDropdownButton
      active={isLinkActive}
      disabled={isLinkMixed || (editor.selection?.collapsed !== false && !isLinkActive)}
      label="링크"
      onEscape={() => editor.focus()}
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
      disabled={isRubyMixed || (editor.selection?.collapsed !== false && !isRubyActive)}
      label="루비"
      onEscape={() => editor.focus()}
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
        <ToolbarIcon icon={values.textAlign.find((a) => a.value === currentTextAlign)?.icon ?? values.textAlign[0].icon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu>
          {#each values.textAlign as { label, value } (value)}
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
          {#each values.lineHeight as { label, value } (value)}
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

    <ToolbarDropdownButton disabled={!editor.can('toggleStyle')} label="문단 자간" onEscape={() => editor.focus()} size="small">
      {#snippet anchor()}
        <ToolbarIcon icon={LetterSpacingIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu>
          {#each values.letterSpacing as { label, value } (value)}
            <DropdownMenuItem
              style={css.raw({ fontSize: '14px' })}
              active={currentLetterSpacing !== undefined && Math.abs(currentLetterSpacing - value) < 0.001}
              onclick={() => {
                editor.focus().dispatch({ type: 'toggleStyle', style: { type: 'letter_spacing', spacing: value } });
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
    label="기본 서식 적용"
    onclick={() => {
      editor.focus().dispatch({ type: 'clearFormatting' });
    }}
    size="small"
  />

  <div class={css({ flexGrow: '1' })}></div>

  <ToolbarButton icon={SearchIcon} keys={['Mod', 'F']} label="찾기, 바꾸기" onclick={() => onSearchClick?.()} size="small" />
</div>
