<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { getAppContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import { graphql } from '$mearie';
  import { getViewContext } from '../@split-view/context.svelte';
  import PanelAi from './PanelAi.svelte';
  import PanelAnchors from './PanelAnchors.svelte';
  import PanelBodySettings from './PanelBodySettings.svelte';
  import PanelInfo from './PanelInfo.svelte';
  import PanelNote from './PanelNote.svelte';
  import PanelSpellcheck from './PanelSpellcheck.svelte';
  import PanelTimeline from './PanelTimeline.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type * as Y from 'yjs';
  import type { Editor_Panel_post$key, Editor_Panel_user$key } from '$mearie';

  type Props = {
    post$key: Editor_Panel_post$key;
    user$key: Editor_Panel_user$key;
    editor?: Ref<Editor>;
    viewEditor?: Ref<Editor>;
    doc: Y.Doc;
    viewDoc?: Y.Doc;
  };

  const minWidth = 240;
  const maxWidth = 400;

  let { post$key, user$key, editor, viewEditor, doc, viewDoc = $bindable() }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment Editor_Panel_user on User {
        id

        ...Editor_Panel_PanelAi_user
        ...Editor_Panel_PanelInfo_user
        ...Editor_Panel_PanelSpellcheck_user
      }
    `),
    () => user$key,
  );

  const post = createFragment(
    graphql(`
      fragment Editor_Panel_post on Post {
        id

        entity {
          id

          ...Editor_Panel_PanelNote_entity
        }

        ...Editor_Panel_PanelInfo_post
        ...Editor_Panel_PanelTimeline_post
      }
    `),
    () => post$key,
  );

  const app = getAppContext();

  const splitViewId = getViewContext().id;

  const isExpanded = $derived(
    Boolean(app.preference.current.panelExpandedByViewId[splitViewId] && app.preference.current.panelTabByViewId[splitViewId]),
  );

  type Resizer = {
    deltaX: number;
    eligible: boolean;
    event: PointerEvent;
    element: HTMLElement;
  };

  let resizer = $state<Resizer | null>(null);
  let newWidth = $derived(clamp((app.preference.current.panelWidth ?? minWidth) + (resizer?.deltaX ?? 0), minWidth, maxWidth));
</script>

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
    {#if app.preference.current.panelTabByViewId[splitViewId] === 'info'}
      <PanelInfo {editor} post$key={post.data} user$key={user.data} />
    {/if}

    {#if app.preference.current.panelTabByViewId[splitViewId] === 'note'}
      <PanelNote entity$key={post.data.entity} />
    {/if}

    {#if app.preference.current.panelTabByViewId[splitViewId] === 'anchors'}
      <PanelAnchors {doc} {editor} />
    {/if}

    {#if app.preference.current.panelTabByViewId[splitViewId] === 'spellcheck'}
      <PanelSpellcheck {editor} user$key={user.data} />
    {/if}

    {#if app.preference.current.panelTabByViewId[splitViewId] === 'ai'}
      <PanelAi {editor} user$key={user.data} />
    {/if}

    {#if app.preference.current.panelTabByViewId[splitViewId] === 'timeline'}
      <PanelTimeline {doc} {editor} post$key={post.data} {viewEditor} bind:viewDoc />
    {/if}

    {#if app.preference.current.panelTabByViewId[splitViewId] === 'settings'}
      <PanelBodySettings {doc} {editor} />
    {/if}
  {/if}
</aside>
