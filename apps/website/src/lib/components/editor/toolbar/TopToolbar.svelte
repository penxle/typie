<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { DropdownMenu, DropdownMenuItem, VerticalDivider } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import BookmarkIcon from '~icons/lucide/bookmark';
  import ChevronsDownUpIcon from '~icons/lucide/chevrons-down-up';
  import ClockFadingIcon from '~icons/lucide/clock-fading';
  import CodeIcon from '~icons/lucide/code';
  import CodeXmlIcon from '~icons/lucide/code-xml';
  import FilePlusIcon from '~icons/lucide/file-plus';
  import FileUpIcon from '~icons/lucide/file-up';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import ImageIcon from '~icons/lucide/image';
  import InfoIcon from '~icons/lucide/info';
  import ListIcon from '~icons/lucide/list';
  import ListOrderedIcon from '~icons/lucide/list-ordered';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import QuoteIcon from '~icons/lucide/quote';
  import SettingsIcon from '~icons/lucide/settings';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import TableIcon from '~icons/lucide/table';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import { fragment, graphql } from '$graphql';
  import { getEditor } from '$lib/editor/context';
  import TableSizeSelector from './TableSizeSelector.svelte';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import ToolbarPanelTabButton from './ToolbarPanelTabButton.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { DocumentEditor_TopToolbar_user } from '$graphql';

  type Props = {
    style?: SystemStyleObject;
    $user: DocumentEditor_TopToolbar_user;
  };

  let { style, $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DocumentEditor_TopToolbar_user on User {
        id
        ...DocumentEditor_TopToolbar_PanelTabButton_user
      }
    `),
  );

  const app = getAppContext();
  const editor = getEditor();

  const toolbarSize = $derived(app.preference.current.toolbarStyle === 'compact' ? 'medium' : 'large');
  const layoutMode = $derived(editor.layout.layoutMode);

  const horizontalRuleVariants = [
    { label: '선', value: 'line' as const },
    { label: '점선', value: 'dashed_line' as const },
    { label: '동그라미가 있는 선', value: 'circle_line' as const },
    { label: '마름모가 있는 선', value: 'diamond_line' as const },
    { label: '동그라미', value: 'circle' as const },
    { label: '마름모', value: 'diamond' as const },
    { label: '세 개의 동그라미', value: 'three_circles' as const },
    { label: '세 개의 마름모', value: 'three_diamonds' as const },
    { label: '지그재그', value: 'zigzag' as const },
  ];

  const blockquoteVariants = [
    { label: '왼쪽 선', value: 'left-line' as const },
    { label: '왼쪽 따옴표', value: 'left-quote' as const },
    { label: '보낸 메시지', value: 'message-sent' as const },
    { label: '받은 메시지', value: 'message-received' as const },
  ];
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
        editor.dispatch({ type: 'insertImage', uploadId: undefined });
        editor.focus();
      }}
      size={toolbarSize}
    />

    <ToolbarButton
      icon={PaperclipIcon}
      label="파일"
      onclick={() => {
        editor.dispatch({ type: 'insertFile', uploadId: undefined });
        editor.focus();
      }}
      size={toolbarSize}
    />

    <ToolbarButton
      icon={FileUpIcon}
      label="임베드"
      onclick={() => {
        editor.dispatch({ type: 'insertEmbed' });
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
          {#each horizontalRuleVariants as { label, value } (value)}
            <DropdownMenuItem
              style={css.raw({ justifyContent: 'center', height: '48px' })}
              onclick={() => {
                editor.focus().dispatch({ type: 'insertHorizontalRule', variant: value });
                close();
              }}
            >
              {label}
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
          {#each blockquoteVariants as { label, value } (value)}
            <DropdownMenuItem
              style={css.raw({ height: '48px' })}
              onclick={() => {
                editor.focus().dispatch({ type: 'toggleBlockquote', variant: value });
                close();
              }}
            >
              {label}
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
        editor.focus().dispatch({ type: 'insertFold' });
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
            editor.focus().dispatch({ type: 'insertTable', rows, cols });
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

    <ToolbarButton disabled={true} icon={CodeIcon} label="코드" size={toolbarSize} />

    <ToolbarButton disabled={true} icon={CodeXmlIcon} label="HTML" size={toolbarSize} />

    {#if layoutMode.type === 'paginated'}
      <VerticalDivider style={css.raw({ height: '16px' })} />

      <ToolbarButton
        icon={FilePlusIcon}
        label="페이지 나누기"
        onclick={() => {
          editor.focus().dispatch({ type: 'insertPageBreak' });
        }}
        size={toolbarSize}
      />
    {/if}
  </div>

  <div class={css({ flexGrow: '1' })}></div>

  <VerticalDivider style={css.raw({ height: '[80%]', marginX: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarPanelTabButton {$user} icon={InfoIcon} label="정보" tab="info" />
    <ToolbarPanelTabButton {$user} icon={StickyNoteIcon} label="노트" tab="note" />
    <ToolbarPanelTabButton {$user} icon={BookmarkIcon} label="북마크" tab="anchors" />
    <ToolbarPanelTabButton {$user} icon={SpellCheckIcon} label="맞춤법" needSubscription tab="spellcheck" />
    <ToolbarPanelTabButton {$user} icon={ClockFadingIcon} label="타임라인" tab="timeline" />
    <ToolbarPanelTabButton {$user} icon={SettingsIcon} label="본문 설정" tab="settings" />
  </div>
</div>
