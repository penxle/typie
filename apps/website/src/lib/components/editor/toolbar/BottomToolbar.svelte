<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { DropdownMenu, DropdownMenuItem, VerticalDivider } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
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
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarColorGrid from './ToolbarColorGrid.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarFontFamily from './ToolbarFontFamily.svelte';
  import ToolbarFontSize from './ToolbarFontSize.svelte';
  import ToolbarFontWeight from './ToolbarFontWeight.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import ToolbarRuby from './ToolbarRuby.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Mark, MarkType, TextAlign } from '$lib/editor/types';

  type Props = {
    style?: SystemStyleObject;
  };

  let { style }: Props = $props();

  const app = getAppContext();
  const editor = getEditor();

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

  const textColors = [
    { label: '블랙', value: 'black', color: '#18181b' },
    { label: '다크 그레이', value: 'darkgray', color: '#52525c' },
    { label: '그레이', value: 'gray', color: '#71717a' },
    { label: '라이트 그레이', value: 'lightgray', color: '#d4d4d8' },
    { label: '화이트', value: 'white', color: '#ffffff' },
    { label: '레드', value: 'red', color: '#ef4444' },
    { label: '오렌지', value: 'orange', color: '#f97316' },
    { label: '앰버', value: 'amber', color: '#f59e0b' },
    { label: '옐로', value: 'yellow', color: '#eab308' },
    { label: '라임', value: 'lime', color: '#84cc16' },
    { label: '그린', value: 'green', color: '#22c55e' },
    { label: '에메랄드', value: 'emerald', color: '#10b981' },
    { label: '틸', value: 'teal', color: '#14b8a6' },
    { label: '시안', value: 'cyan', color: '#06b6d4' },
    { label: '스카이', value: 'sky', color: '#0ea5e9' },
    { label: '블루', value: 'blue', color: '#3b82f6' },
    { label: '인디고', value: 'indigo', color: '#6366f1' },
    { label: '바이올렛', value: 'violet', color: '#8b5cf6' },
    { label: '퍼플', value: 'purple', color: '#a855f7' },
    { label: '마젠타', value: 'fuchsia', color: '#d946ef' },
    { label: '핑크', value: 'pink', color: '#ec4899' },
    { label: '로즈', value: 'rose', color: '#f43f5e' },
  ];

  const textBackgroundColors = [
    { label: '배경 없음', value: null, color: null },
    { label: '그레이', value: 'gray', color: '#f1f1f2' },
    { label: '레드', value: 'red', color: '#fdebec' },
    { label: '오렌지', value: 'orange', color: '#ffecd5' },
    { label: '옐로', value: 'yellow', color: '#fef3c7' },
    { label: '그린', value: 'green', color: '#dff3e3' },
    { label: '블루', value: 'blue', color: '#e7f3f8' },
    { label: '퍼플', value: 'purple', color: '#f0e7fe' },
  ];

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
        editor.dispatch({ type: 'undo' });
      }}
      size="small"
    />

    <ToolbarButton
      style={css.raw({ borderLeftRadius: '0' })}
      disabled={!editor.can('redo')}
      icon={RedoIcon}
      label="다시 실행"
      onclick={() => {
        editor.dispatch({ type: 'redo' });
      }}
      size="small"
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarDropdownButton chevron disabled={!editor.can('toggleTextColor')} label="글씨 색" placement="bottom-start" size="small">
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
          onSelect={(value) => editor.dispatch({ type: 'toggleTextColor', key: value })}
          {opened}
        />
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton chevron disabled={!editor.can('toggleBackgroundColor')} label="배경색" placement="bottom-start" size="small">
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
          onSelect={(value) => editor.dispatch({ type: 'toggleBackgroundColor', key: value ?? undefined })}
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
        editor.dispatch({ type: 'toggleBold' });
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
        editor.dispatch({ type: 'toggleItalic' });
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
        editor.dispatch({ type: 'toggleStrikethrough' });
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
        editor.dispatch({ type: 'toggleUnderline' });
      }}
      size="small"
    />
  </div>

  <VerticalDivider style={css.raw({ height: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarDropdownButton active={isLinkActive} disabled={true} label="링크" size="small">
      {#snippet anchor()}
        <ToolbarIcon icon={LinkIcon} />
      {/snippet}

      {#snippet floating()}
        <div class={css({ padding: '8px' })}>링크 기능 준비 중</div>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton active={isRubyActive} disabled={!editor.can('toggleRuby')} label="루비" size="small">
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
    <ToolbarDropdownButton disabled={!editor.can('setTextAlign')} label="문단 정렬" size="small">
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
                editor.dispatch({ type: 'setTextAlign', align: value });
                close();
              }}
            >
              {label}
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton disabled={!editor.can('setLineHeight')} label="문단 행간" size="small">
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
                editor.dispatch({ type: 'setLineHeight', height: value });
                close();
              }}
            >
              {label}
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton disabled={!editor.can('setLetterSpacing')} label="문단 자간" size="small">
      {#snippet anchor()}
        <ToolbarIcon icon={LetterSpacingIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu>
          {#each letterSpacings as { label, value } (value)}
            <DropdownMenuItem
              style={css.raw({ fontSize: '14px' })}
              active={currentLetterSpacing === value}
              onclick={() => {
                editor.dispatch({ type: 'setLetterSpacing', spacing: value });
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
      editor.dispatch({ type: 'clearFormatting' });
    }}
    size="small"
  />

  <div class={css({ flexGrow: '1' })}></div>

  <ToolbarButton disabled={true} icon={SearchIcon} keys={['Mod', 'F']} label="찾기, 바꾸기" size="small" />
</div>
