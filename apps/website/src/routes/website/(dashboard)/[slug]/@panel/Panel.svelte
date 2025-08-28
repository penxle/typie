<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getAppContext } from '@typie/ui/context';
  import { fragment, graphql } from '$graphql';
  import PanelBodySettings from './PanelBodySettings.svelte';
  import PanelPost from './PanelPost.svelte';
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

  let { $post: _post, $user: _user, editor, doc }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment Editor_Panel_user on User {
        id

        ...Editor_Panel_PanelPost_user
      }
    `),
  );

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_Panel_post on Post {
        id

        ...Editor_Panel_PanelPost_post
      }
    `),
  );

  const app = getAppContext();

  const isExpanded = $derived(app.preference.current.panelExpanded && app.preference.current.panelTab);
</script>

<aside
  style:--min-width="240px"
  style:--width="15vw"
  style:--max-width="260px"
  class={css({
    flexShrink: '0',
    minWidth: isExpanded ? 'var(--min-width)' : '0',
    width: 'var(--width)',
    maxWidth: isExpanded ? 'var(--max-width)' : '0',
    opacity: isExpanded ? '100' : '0',
    transitionProperty: 'min-width, max-width, opacity',
    transitionDuration: '200ms',
    transitionTimingFunction: 'ease',
    willChange: 'min-width, max-width, opacity',
    overflowX: 'hidden',
    borderLeftWidth: '1px',
    borderColor: 'border.subtle',
  })}
>
  {#if app.preference.current.panelTab === 'info'}
    <PanelPost {$post} {$user} {doc} {editor} />
  {/if}

  {#if app.preference.current.panelTab === 'settings'}
    <PanelBodySettings {doc} {editor} />
  {/if}
</aside>
