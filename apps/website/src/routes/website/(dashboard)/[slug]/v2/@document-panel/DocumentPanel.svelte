<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, FullAccessBadge, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import { onDestroy } from 'svelte';
  import ConstructionIcon from '~icons/lucide/construction';
  import LightbulbIcon from '~icons/lucide/lightbulb';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { graphql } from '$mearie';
  import { PlanUpgradeDialog } from '../../../plan-upgrade-dialog.svelte';
  import { getPane, getPaneGroup } from '../../@pane/context.svelte';
  import DocumentPanelAi from './DocumentPanelAi.svelte';
  import DocumentPanelComment from './DocumentPanelComment.svelte';
  import DocumentPanelInfo from './DocumentPanelInfo.svelte';
  import DocumentPanelNote from './DocumentPanelNote.svelte';
  import DocumentPanelSettings from './DocumentPanelSettings.svelte';
  import DocumentPanelSpellcheck from './DocumentPanelSpellcheck.svelte';
  import DocumentPanelTimeline from './DocumentPanelTimeline.svelte';
  import { getDocumentPanelFocusReturn } from './focus-return.svelte';
  import type { Component } from 'svelte';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';
  import type { DocumentPanelV2_document$key, DocumentPanelV2_user$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelV2_document$key;
    user$key: DocumentPanelV2_user$key;
    editor?: Editor | undefined;
  };

  const minWidth = 240;
  const maxWidth = 400;

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  let { document$key, user$key, editor: _editor }: Props = $props();

  const document = createFragment(
    graphql(`
      fragment DocumentPanelV2_document on Document {
        id

        entity {
          id
          ...DocumentPanelV2_Note_entity
        }

        ...DocumentPanelV2_Ai_document
        ...DocumentPanelV2_Info_document
        ...DocumentPanelV2_Settings_document
        ...DocumentPanelV2_Spellcheck_document
        ...DocumentPanelV2Timeline_document
      }
    `),
    () => document$key,
  );

  const user = createFragment(
    graphql(`
      fragment DocumentPanelV2_user on User {
        id

        subscription {
          id
        }

        ...DocumentPanelV2_Ai_user
        ...DocumentPanelV2_Info_user
        ...DocumentPanelV2_Spellcheck_user
      }
    `),
    () => user$key,
  );

  const app = getAppContext();
  const ctx = getEditorContext();

  const paneId = getPane().id;
  const paneGroup = getPaneGroup();
  const focusReturn = getDocumentPanelFocusReturn();

  const isExpanded = $derived(
    paneGroup.state.current.panelExpandedByPaneId[paneId] && Object.hasOwn(paneGroup.state.current.panelTabByPaneId, paneId),
  );

  type Resizer = {
    deltaX: number;
    eligible: boolean;
    event: PointerEvent;
    element: HTMLElement;
  };

  let resizer = $state<Resizer | null>(null);
  let newWidth = $derived(clamp((app.preference.current.panelWidth ?? minWidth) + (resizer?.deltaX ?? 0), minWidth, maxWidth));

  onDestroy(() => focusReturn.discard());
</script>

{#snippet planUpgradePrompt(featureIcon: Component, featureName: string, description: string)}
  <div
    class={flex({
      flexDirection: 'column',
      minWidth: 'var(--min-width)',
      width: 'var(--width)',
      maxWidth: 'var(--max-width)',
      height: 'full',
    })}
  >
    <div
      class={flex({
        flexShrink: '0',
        alignItems: 'center',
        gap: '6px',
        height: '41px',
        paddingX: '20px',
        fontSize: '13px',
        fontWeight: 'semibold',
        color: 'text.subtle',
        borderBottomWidth: '1px',
        borderColor: 'surface.muted',
      })}
    >
      {featureName}
      <FullAccessBadge />
    </div>

    <div
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '20px',
        flexGrow: '1',
        paddingX: '24px',
        textAlign: 'center',
      })}
    >
      <Icon style={css.raw({ color: 'text.faint' })} icon={featureIcon} size={32} />

      <p class={css({ fontSize: '13px', color: 'text.faint', whiteSpace: 'pre-line' })}>
        {description}
      </p>

      <Button
        onclick={() => {
          PlanUpgradeDialog.show({
            message: `${featureName} 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.`,
          });
        }}
        size="sm"
      >
        지금 업그레이드하기
      </Button>
    </div>
  </div>
{/snippet}

<aside
  style:--min-width={`${minWidth}px`}
  style:--width={`${newWidth}px`}
  style:--max-width={`${maxWidth}px`}
  class={flex({
    position: 'relative',
    zIndex: 'panel',
    backgroundColor: 'surface.default',
    flexDirection: 'column',
    flexShrink: '0',
    minWidth: isExpanded ? 'var(--min-width)' : '0',
    width: isExpanded ? 'var(--width)' : '0',
    maxWidth: isExpanded ? 'var(--max-width)' : '0',
    opacity: isExpanded ? '100' : '0',
    transitionProperty: '[min-width, max-width, opacity]',
    transitionDuration: '200ms',
    transitionTimingFunction: 'ease',
    willChange: 'min-width, max-width, opacity',
    overflow: 'hidden',
    borderLeftWidth: '1px',
    borderColor: 'border.subtle',
  })}
  onfocusin={(event) => {
    if (event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)) return;
    focusReturn.capture(event.relatedTarget);
  }}
>
  <div
    class={css({
      position: 'absolute',
      zIndex: '1',
      top: '0',
      left: '0',
      width: '8px',
      height: 'full',
      cursor: 'col-resize',
      _hoverAfter: {
        content: '""',
        display: 'block',
        borderRightRadius: '4px',
        height: 'full',
        width: '2px',
        backgroundColor: 'border.strong',
        opacity: '50',
      },
    })}
    onpointerdowncapture={(e) => {
      resizer = {
        element: e.currentTarget,
        event: e,
        deltaX: 0,
        eligible: false,
      };
    }}
    onpointermovecapture={(e) => {
      if (!resizer) return;

      if (!resizer.eligible) {
        resizer.eligible = true;
        resizer.element.setPointerCapture(e.pointerId);
      }

      resizer.deltaX = Math.round(resizer.event.clientX - e.clientX);
    }}
    onpointerupcapture={() => {
      if (!resizer) return;

      if (resizer.eligible && resizer.element.hasPointerCapture(resizer.event.pointerId)) {
        resizer.element.releasePointerCapture(resizer.event.pointerId);
      }

      app.preference.current.panelWidth = newWidth;

      resizer = null;
    }}
  ></div>

  {#if isExpanded}
    {#if paneGroup.state.current.panelTabByPaneId[paneId] === 'settings'}
      <DocumentPanelSettings document$key={document.data} />
    {:else if paneGroup.state.current.panelTabByPaneId[paneId] === 'info'}
      <DocumentPanelInfo document$key={document.data} editor={ctx.editor} user$key={user.data} />
    {:else if paneGroup.state.current.panelTabByPaneId[paneId] === 'note'}
      <DocumentPanelNote entity$key={document.data.entity} />
    {:else if paneGroup.state.current.panelTabByPaneId[paneId] === 'timeline'}
      {#if ctx.editor}
        <DocumentPanelTimeline document$key={document.data} />
      {/if}
    {:else if paneGroup.state.current.panelTabByPaneId[paneId] === 'spellcheck'}
      {#if user.data.subscription}
        <DocumentPanelSpellcheck document$key={document.data} editor={ctx.editor} user$key={user.data} />
      {:else}
        {@render planUpgradePrompt(SpellCheckIcon, '맞춤법 검사', '글의 맞춤법과 띄어쓰기를\n자동으로 검사하고 수정할 수 있어요.')}
      {/if}
    {:else if paneGroup.state.current.panelTabByPaneId[paneId] === 'ai'}
      {#if user.data.subscription}
        <DocumentPanelAi document$key={document.data} editor={ctx.editor} user$key={user.data} />
      {:else}
        {@render planUpgradePrompt(LightbulbIcon, 'AI 피드백', '글의 구조, 표현, 흐름에 대한\nAI 분석과 피드백을 받아볼 수 있어요.')}
      {/if}
    {:else if paneGroup.state.current.panelTabByPaneId[paneId] === 'comment'}
      <DocumentPanelComment editor={ctx.editor} />
    {:else}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '8px',
          height: 'full',
          color: 'text.faint',
          fontSize: '12px',
        })}
      >
        <Icon style={css.raw({ color: 'text.faint' })} icon={ConstructionIcon} size={24} />
        아직 준비중인 기능이에요
      </div>
    {/if}
  {/if}
</aside>
