<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { DropdownMenu, DropdownMenuItem, VerticalDivider } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { getEditorContext, values } from '@typie/ui/tiptap';
  import BookmarkIcon from '~icons/lucide/bookmark';
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
  import PaperclipIcon from '~icons/lucide/paperclip';
  import QuoteIcon from '~icons/lucide/quote';
  import SettingsIcon from '~icons/lucide/settings';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import TableIcon from '~icons/lucide/table';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import { graphql } from '$mearie';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import ToolbarPanelTabButton from './ToolbarPanelTabButton.svelte';
  import type { Editor } from '@tiptap/core';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Ref } from '@typie/ui/utils';
  import type { Editor_TopToolbar_site$key } from '$mearie';

  type Props = {
    site$key: Editor_TopToolbar_site$key;
    editor?: Ref<Editor>;
    style?: SystemStyleObject;
  };

  let { site$key, editor, style }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment Editor_TopToolbar_site on Site {
        id

        user {
          id
          ...Editor_TopToolbar_PanelTabButton_user

          subscription {
            id
          }
        }
      }
    `),
    () => site$key,
  );

  const app = getAppContext();
  const editorContext = getEditorContext();

  const toolbarSize = $derived(app.preference.current.toolbarStyle === 'compact' ? 'medium' : 'large');
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
      opacity: editorContext?.timeline ? '50' : '100',
      pointerEvents: editorContext?.timeline ? 'none' : 'auto',
    })}
  >
    <ToolbarButton
      disabled={!editor?.current.can().setImage()}
      icon={ImageIcon}
      label="이미지"
      onclick={() => {
        editor?.current.chain().focus().setImage().run();
      }}
      size={toolbarSize}
    />

    <ToolbarButton
      disabled={!editor?.current.can().setFile()}
      icon={PaperclipIcon}
      label="파일"
      onclick={() => {
        editor?.current.chain().focus().setFile().run();
      }}
      size={toolbarSize}
    />

    <ToolbarButton
      disabled={!editor?.current.can().setEmbed()}
      icon={FileUpIcon}
      label="임베드"
      onclick={() => {
        editor?.current.chain().focus().setEmbed().run();
      }}
      size={toolbarSize}
    />

    <ToolbarDropdownButton
      active={editor?.current.isActive('horizontal_rule')}
      disabled={!editor?.current.can().setHorizontalRule()}
      label="구분선"
      size={toolbarSize}
    >
      {#snippet anchor()}
        <ToolbarIcon icon={HorizontalRuleIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu style={css.raw({ maxWidth: '200px' })}>
          {#each values.horizontalRule as { type, component: Component } (type)}
            <DropdownMenuItem
              style={css.raw({ justifyContent: 'center', height: '48px' })}
              onclick={() => {
                editor?.current.chain().focus().setHorizontalRule(type).run();
                close();
              }}
            >
              <Component />
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarDropdownButton
      active={editor?.current.isActive('blockquote')}
      disabled={!editor?.current.can().toggleBlockquote()}
      label="인용구"
      size={toolbarSize}
    >
      {#snippet anchor()}
        <ToolbarIcon icon={QuoteIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu style={css.raw({ maxWidth: '200px' })}>
          {#each values.blockquote as { type, component: Component } (type)}
            <DropdownMenuItem
              style={css.raw({ height: '48px' })}
              onclick={() => {
                editor?.current.chain().focus().toggleBlockquote(type).run();
                close();
              }}
            >
              <Component renderAsOption />
            </DropdownMenuItem>
          {/each}
        </DropdownMenu>
      {/snippet}
    </ToolbarDropdownButton>

    <ToolbarButton
      disabled={!editor?.current.can().toggleCallout()}
      icon={GalleryVerticalEndIcon}
      label="강조"
      onclick={() => {
        editor?.current.chain().focus().toggleCallout().run();
      }}
      size={toolbarSize}
    />

    <ToolbarButton
      disabled={!editor?.current.can().toggleFold()}
      icon={ChevronsDownUpIcon}
      label="접기"
      onclick={() => {
        editor?.current.chain().focus().toggleFold().run();
      }}
      size={toolbarSize}
    />

    <ToolbarButton
      disabled={!editor?.current.can().insertTable()}
      icon={TableIcon}
      label="표"
      onclick={() => {
        editor?.current.chain().focus().insertTable().run();
      }}
      size={toolbarSize}
    />

    <ToolbarDropdownButton
      disabled={!editor?.current || (!editor.current.can().toggleBulletList() && !editor.current.can().toggleOrderedList())}
      label="목록"
      size={toolbarSize}
    >
      {#snippet anchor()}
        <ToolbarIcon icon={ListIcon} />
      {/snippet}

      {#snippet floating({ close })}
        <DropdownMenu>
          <DropdownMenuItem
            onclick={() => {
              editor?.current.chain().focus().toggleBulletList().run();
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
              editor?.current.chain().focus().toggleOrderedList().run();
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

    {#if editor?.current.storage.page.layout}
      <VerticalDivider style={css.raw({ height: '16px' })} />

      <ToolbarButton
        icon={FilePlusIcon}
        keys={['Mod', 'Enter']}
        label="페이지 나누기"
        onclick={() => {
          editor?.current.chain().focus().setPageBreak().run();
        }}
        size={toolbarSize}
      />
    {/if}
  </div>

  <div class={css({ flexGrow: '1' })}></div>

  <VerticalDivider style={css.raw({ height: '[80%]', marginX: '12px' })} />

  <div class={flex({ alignItems: 'center', gap: '4px' })}>
    <ToolbarPanelTabButton icon={InfoIcon} label="정보" tab="info" user$key={site.data.user} />
    <ToolbarPanelTabButton icon={StickyNoteIcon} label="노트" tab="note" user$key={site.data.user} />
    <ToolbarPanelTabButton icon={BookmarkIcon} label="북마크" tab="anchors" user$key={site.data.user} />
    <ToolbarPanelTabButton icon={SpellCheckIcon} label="맞춤법" needSubscription tab="spellcheck" user$key={site.data.user} />
    <ToolbarPanelTabButton icon={LightbulbIcon} label="AI 피드백" needSubscription tab="ai" user$key={site.data.user} />
    <ToolbarPanelTabButton icon={ClockFadingIcon} label="타임라인" tab="timeline" user$key={site.data.user} />
    <ToolbarPanelTabButton icon={SettingsIcon} label="본문 설정" tab="settings" user$key={site.data.user} />
  </div>
</div>
