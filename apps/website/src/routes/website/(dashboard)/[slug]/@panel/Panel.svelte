<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { getAppContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import { fragment, graphql } from '$graphql';
  import { getViewContext } from '../@split-view/context.svelte';
  import PanelBodySettings from './PanelBodySettings.svelte';
  import PanelInfo from './PanelInfo.svelte';
  import PanelSpellcheck from './PanelSpellcheck.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type * as Y from 'yjs';
  import type { Editor_Panel_post, Editor_Panel_user } from '$graphql';

  type Props = {
    $post: Editor_Panel_post;
    $user: Editor_Panel_user;
    editor?: Ref<Editor>;
    doc: Y.Doc;
  };

  const minWidth = 240;
  const maxWidth = 400;

  let { $post: _post, $user: _user, editor, doc }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment Editor_Panel_user on User {
        id

        ...Editor_Panel_PanelPost_user
        ...Editor_Panel_PanelSpellcheck_user
      }
    `),
  );

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_Panel_post on Post {
        id

        ...Editor_Panel_PanelInfo_post
      }
    `),
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
    flexDirection: 'column',
    flexShrink: '0',
    minWidth: isExpanded ? 'var(--min-width)' : '0',
    width: isExpanded ? 'var(--width)' : '0',
    maxWidth: isExpanded ? 'var(--max-width)' : '0',
    opacity: isExpanded ? '100' : '0',
    transitionProperty: 'min-width, max-width, opacity',
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
      <PanelInfo {$post} {$user} {doc} {editor} />
    {/if}

    {#if app.preference.current.panelTabByViewId[splitViewId] === 'spellcheck'}
      <PanelSpellcheck {$user} {editor} />
    {/if}

    {#if app.preference.current.panelTabByViewId[splitViewId] === 'settings'}
      <PanelBodySettings {doc} {editor} />
    {/if}
  {/if}
</aside>
