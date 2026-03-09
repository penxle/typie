<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { DropdownMenu, DropdownMenuItem, VerticalDivider } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import ChevronsDownUpIcon from '~icons/lucide/chevrons-down-up';
  import ClockFadingIcon from '~icons/lucide/clock-fading';
  import FilePlusIcon from '~icons/lucide/file-plus';
  import FileUpIcon from '~icons/lucide/file-up';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import ImageIcon from '~icons/lucide/image';
  import InfoIcon from '~icons/lucide/info';
  import LightbulbIcon from '~icons/lucide/lightbulb';
  import ListIcon from '~icons/lucide/list';
  import ListOrderedIcon from '~icons/lucide/list-ordered';
  import MessageSquareTextIcon from '~icons/lucide/message-square-text';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import QuoteIcon from '~icons/lucide/quote';
  import SettingsIcon from '~icons/lucide/settings';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import TableIcon from '~icons/lucide/table';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import { blockquoteVariants, horizontalRuleVariants } from '$lib/components/editor/values';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import TableSizeSelector from './TableSizeSelector.svelte';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import ToolbarPanelTabButton from './ToolbarPanelTabButton.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    style?: SystemStyleObject;
  };

  let { style }: Props = $props();

  const app = getAppContext();
  const { editor } = getEditorContext();

  const toolbarSize = $derived(app.preference.current.toolbarStyle === 'compact' ? 'medium' : 'large');
  const layoutMode = $derived(editor.layout?.layoutMode);
</script>

<div
  class={css(
    {
      display: 'flex',
      flexShrink: '0',
      alignItems: 'center',
      gap: '4px',
      paddingLeft: '16px',
      paddingRight: '10px',
      paddingY: '6px',
      overflowX: 'auto',
      scrollbarWidth: '[thin]',
      borderBottomWidth: '1px',
      borderColor: 'border.subtle',
      zIndex: app.preference.current.zenModeEnabled ? 'underEditor' : 'overEditor',
      backgroundColor: 'surface.default',
    },
    style,
  )}
  role="toolbar"
  tabindex="-1"
>
  <div
    class={flex({
      alignItems: 'center',
      gap: '4px',
    })}
  >
    <ToolbarButton
      icon={ImageIcon}
      label="이미지"
      onclick={() => {
        editor.dispatch({ type: 'insertImage', uploadId: undefined }).scrollIntoView();
        editor.focus();
      }}
      size={toolbarSize}
    />

    <ToolbarButton
      icon={PaperclipIcon}
      label="파일"
      onclick={() => {
        editor.dispatch({ type: 'insertFile', uploadId: undefined }).scrollIntoView();
        editor.focus();
      }}
      size={toolbarSize}
    />

    <ToolbarButton
      icon={FileUpIcon}
      label="임베드"
      onclick={() => {
        editor.dispatch({ type: 'insertEmbed' }).scrollIntoView();
        editor.focus();
      }}
      size={toolbarSize}
    />

    <ToolbarDropdownButton label="구분선" size={toolbarSize}>
      {#snippet anchor()}
        <ToolbarIcon icon={HorizontalRuleIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu style={css.raw({ maxWidth: '200px' })}>
          {#each horizontalRuleVariants as { variant, component: Component } (variant)}
            <DropdownMenuItem
              style={css.raw({ justifyContent: 'center', height: '48px' })}
              onclick={() => {
                editor.focus().dispatch({ type: 'insertHorizontalRule', variant }).scrollIntoView();
                close();
              }}
            >
              <Component />
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton label="인용구" size={toolbarSize}>
      {#snippet anchor()}
        <ToolbarIcon icon={QuoteIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu style={css.raw({ maxWidth: '200px' })}>
          {#each blockquoteVariants as { variant, component: Component } (variant)}
            <DropdownMenuItem
              style={css.raw({ height: '48px' })}
              onclick={() => {
                editor.focus().dispatch({ type: 'toggleBlockquote', variant });
                close();
              }}
            >
              <Component />
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarButton
      icon={GalleryVerticalEndIcon}
      label="강조"
      onclick={() => {
        editor.dispatch({ type: 'toggleCallout' });
        editor.focus();
      }}
      size={toolbarSize}
    />

    <ToolbarButton
      icon={ChevronsDownUpIcon}
      label="접기"
      onclick={() => {
        editor.focus().dispatch({ type: 'insertFold' }).scrollIntoView();
      }}
      size={toolbarSize}
    />

    <ToolbarDropdownButton label="표" placement="bottom-start" size={toolbarSize}>
      {#snippet anchor()}
        <ToolbarIcon icon={TableIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <TableSizeSelector
          onSelect={(rows, cols) => {
            editor.focus().dispatch({ type: 'insertTable', rows, cols }).scrollIntoView();
            close();
          }}
        />
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton label="목록" size={toolbarSize}>
      {#snippet anchor()}
        <ToolbarIcon icon={ListIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu>
          <DropdownMenuItem
            onclick={() => {
              editor.focus().dispatch({ type: 'toggleBulletList' });
              close();
            }}
          >
            <div class={flex({ alignItems: 'center', gap: '4px' })}>
              <ToolbarIcon icon={ListIcon} />
              순서 없는 목록
            </div>
          </DropdownMenuItem>

          <DropdownMenuItem
            onclick={() => {
              editor.focus().dispatch({ type: 'toggleOrderedList' });
              close();
            }}
          >
            <div class={flex({ alignItems: 'center', gap: '4px' })}>
              <ToolbarIcon icon={ListOrderedIcon} />
              순서 있는 목록
            </div>
          </DropdownMenuItem>
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    {#if layoutMode?.type === 'paginated'}
      <VerticalDivider style={css.raw({ height: '16px' })} />

      <ToolbarButton
        icon={FilePlusIcon}
        label="페이지 나누기"
        onclick={() => {
          editor.focus().dispatch({ type: 'insertPageBreak' }).scrollIntoView();
        }}
        size={toolbarSize}
      />
    {/if}
  </div>

  <div class={css({ flexGrow: '1' })}></div>

  <VerticalDivider style={css.raw({ height: '[80%]', marginX: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarPanelTabButton icon={InfoIcon} label="정보" tab="info" />
    <ToolbarPanelTabButton icon={StickyNoteIcon} label="노트" tab="note" />
    <ToolbarPanelTabButton icon={MessageSquareTextIcon} label="코멘트" tab="remarks" />
    <ToolbarPanelTabButton icon={SpellCheckIcon} label="맞춤법" tab="spellcheck" />
    <ToolbarPanelTabButton icon={LightbulbIcon} label="AI 피드백" tab="ai" />
    <ToolbarPanelTabButton icon={ClockFadingIcon} label="타임라인" tab="timeline" />
    <ToolbarPanelTabButton icon={SettingsIcon} label="본문 설정" tab="settings" />
  </div>
</div>
