<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { DropdownMenu, DropdownMenuItem, VerticalDivider } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
  import BoldIcon from '~icons/lucide/bold';
  import ItalicIcon from '~icons/lucide/italic';
  import LinkIcon from '~icons/lucide/link';
  import MessageSquarePlusIcon from '~icons/lucide/message-square-plus';
  import RedoIcon from '~icons/lucide/redo';
  import RemoveFormattingIcon from '~icons/lucide/remove-formatting';
  import SearchIcon from '~icons/lucide/search';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import UnderlineIcon from '~icons/lucide/underline';
  import UndoIcon from '~icons/lucide/undo';
  import LetterSpacingIcon from '~icons/typie/letter-spacing';
  import LineHeightIcon from '~icons/typie/line-height';
  import RubyIcon from '~icons/typie/ruby';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { THEME_COLORS } from '$lib/editor-ffi/theme';
  import { values } from '$lib/editor-ffi/values';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarColorGrid from './ToolbarColorGrid.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarFontFamily from './ToolbarFontFamily.svelte';
  import ToolbarFontSize from './ToolbarFontSize.svelte';
  import ToolbarFontWeight from './ToolbarFontWeight.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import ToolbarLink from './ToolbarLink.svelte';
  import ToolbarRuby from './ToolbarRuby.svelte';
  import type { Message, ModifierType, Tri } from '@typie/editor-ffi/browser';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { ThemeVariant } from '$lib/editor-ffi/theme';

  type Font = { id?: string | null; weight: number; subfamilyDisplayName?: string | null; state: string };
  type FontFamily = { id: string; familyName: string; displayName: string; state: string; fonts: readonly Font[] };

  type Props = {
    style?: SystemStyleObject;
    fontFamilies?: readonly FontFamily[];
    onSearchClick?: () => void;
    onFontUploadClick?: () => void;
  };

  let { style, fontFamilies = [], onSearchClick, onFontUploadClick }: Props = $props();

  const app = getAppContext();
  const theme = getThemeContext();
  const ctx = getEditorContext();

  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );

  const tc = $derived(THEME_COLORS[themeVariant]);

  const textColors = $derived(values.textColor.map((c) => ({ label: c.label, value: c.value, color: tc[c.themeKey] })));

  const textBackgroundColors = $derived(
    values.textBackgroundColor.map((c) => ({ label: c.label, value: c.value, color: c.themeKey ? tc[c.themeKey] : null })),
  );

  type ToggleState = { active: boolean; indeterminate: boolean };

  const toggleState = (tri: Tri<undefined> | undefined): ToggleState => {
    if (tri?.type === 'uniform') return { active: true, indeterminate: false };
    if (tri?.type === 'mixed') return { active: false, indeterminate: true };
    return { active: false, indeterminate: false };
  };

  const currentTextColor = $derived(
    ctx.editor?.modifierState?.text_color?.type === 'uniform' ? ctx.editor.modifierState.text_color.value.value : undefined,
  );

  const currentTextBackgroundColor = $derived(
    ctx.editor?.modifierState?.background_color?.type === 'uniform' ? ctx.editor.modifierState.background_color.value.value : undefined,
  );

  const currentLineHeight = $derived(
    ctx.editor?.modifierState?.line_height?.type === 'uniform' ? ctx.editor.modifierState.line_height.value.value : undefined,
  );

  const currentLetterSpacing = $derived(
    ctx.editor?.modifierState?.letter_spacing?.type === 'uniform' ? ctx.editor.modifierState.letter_spacing.value.value : undefined,
  );

  const currentTextAlign = $derived(
    ctx.editor?.modifierState?.alignment?.type === 'uniform' ? ctx.editor.modifierState.alignment.value.value : undefined,
  );

  const boldS = $derived(toggleState(ctx.editor?.modifierState?.bold));
  const italicS = $derived(toggleState(ctx.editor?.modifierState?.italic));
  const strikethroughS = $derived(toggleState(ctx.editor?.modifierState?.strikethrough));
  const underlineS = $derived(toggleState(ctx.editor?.modifierState?.underline));

  const isLinkActive = $derived(ctx.editor?.modifierState?.link?.type === 'uniform');
  const isLinkMixed = $derived(ctx.editor?.modifierState?.link?.type === 'mixed');
  const isRubyActive = $derived(ctx.editor?.modifierState?.ruby?.type === 'uniform');
  const isRubyMixed = $derived(ctx.editor?.modifierState?.ruby?.type === 'mixed');

  const isCollapsed = $derived(ctx.editor?.isSelectionCollapsed ?? true);

  const enqueue = (message: Message) => {
    ctx.editor?.enqueue(message);
    ctx.editor?.focus();
  };

  const toggleModifier = (modifier_type: ModifierType) => {
    enqueue({ type: 'modifier', op: { type: 'toggle', modifier_type } });
  };
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
      icon={UndoIcon}
      label="실행 취소"
      onclick={() => enqueue({ type: 'history', op: { type: 'undo' } })}
      size="small"
    />

    <ToolbarButton
      style={css.raw({ borderLeftRadius: '0' })}
      icon={RedoIcon}
      label="다시 실행"
      onclick={() => enqueue({ type: 'history', op: { type: 'redo' } })}
      size="small"
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarDropdownButton chevron label="글씨 색" onEscape={() => ctx.editor?.focus()} placement="bottom-start" size="small">
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
            enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'text_color', value } } });
          }}
          {opened}
        />
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton chevron label="배경색" onEscape={() => ctx.editor?.focus()} placement="bottom-start" size="small">
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
            enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'background_color', value } } });
          }}
          {opened}
          shape="square"
        />
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarFontFamily {fontFamilies} onUploadClick={onFontUploadClick} />
    <ToolbarFontWeight {fontFamilies} />
    <ToolbarFontSize />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarButton
      active={boldS.active}
      icon={BoldIcon}
      keys={['Mod', 'B']}
      label="굵게"
      onclick={() => toggleModifier('bold')}
      size="small"
    />

    <ToolbarButton
      active={italicS.active}
      icon={ItalicIcon}
      keys={['Mod', 'I']}
      label="기울임"
      onclick={() => toggleModifier('italic')}
      size="small"
    />

    <ToolbarButton
      active={strikethroughS.active}
      icon={StrikethroughIcon}
      keys={['Mod', 'Shift', 'S']}
      label="취소선"
      onclick={() => toggleModifier('strikethrough')}
      size="small"
    />

    <ToolbarButton
      active={underlineS.active}
      icon={UnderlineIcon}
      keys={['Mod', 'U']}
      label="밑줄"
      onclick={() => toggleModifier('underline')}
      size="small"
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarDropdownButton
      active={isLinkActive}
      disabled={isLinkMixed || (isCollapsed && !isLinkActive)}
      label="링크"
      onEscape={() => ctx.editor?.focus()}
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
      disabled={isRubyMixed || (isCollapsed && !isRubyActive)}
      label="루비"
      onEscape={() => ctx.editor?.focus()}
      size="small"
    >
      {#snippet anchor()}
        <ToolbarIcon icon={RubyIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <ToolbarRuby {close} />
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarButton
      disabled={isCollapsed}
      icon={MessageSquarePlusIcon}
      label="코멘트"
      onclick={() => ctx.editor?.requestCommentCompose?.()}
      onpointerdown={(e) => e.preventDefault()}
      size="small"
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarDropdownButton label="문단 정렬" onEscape={() => ctx.editor?.focus()} size="small">
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
                enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'alignment', value } } });
                close();
              }}
            >
              {label}
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton label="문단 행간" onEscape={() => ctx.editor?.focus()} size="small">
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
                enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'line_height', value } } });
                close();
              }}
            >
              {label}
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton label="문단 자간" onEscape={() => ctx.editor?.focus()} size="small">
      {#snippet anchor()}
        <ToolbarIcon icon={LetterSpacingIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu>
          {#each values.letterSpacing as { label, value } (value)}
            <DropdownMenuItem
              style={css.raw({ fontSize: '14px' })}
              active={currentLetterSpacing === value}
              onclick={() => {
                enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'letter_spacing', value } } });
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
    icon={RemoveFormattingIcon}
    keys={['Mod', '\\']}
    label="기본 서식 적용"
    onclick={() => enqueue({ type: 'modifier', op: { type: 'clear_all' } })}
    size="small"
  />

  <div class={css({ flexGrow: '1' })}></div>

  <ToolbarButton icon={SearchIcon} keys={['Mod', 'F']} label="찾기, 바꾸기" onclick={() => onSearchClick?.()} size="small" />
</div>
