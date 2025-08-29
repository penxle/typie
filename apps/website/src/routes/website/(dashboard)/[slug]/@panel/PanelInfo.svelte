<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, VerticalDivider } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { PostType } from '@/enums';
  import { fragment, graphql } from '$graphql';
  import PanelAnchors from './PanelAnchors.svelte';
  import PanelPost from './PanelPost.svelte';
  import PanelSubTab from './PanelSubTab.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type * as Y from 'yjs';
  import type { Editor_Panel_PanelInfo_post, Editor_Panel_PanelPost_user } from '$graphql';

  type Props = {
    $post: Editor_Panel_PanelInfo_post;
    $user: Editor_Panel_PanelPost_user;
    editor?: Ref<Editor>;
    doc: Y.Doc;
  };

  const app = getAppContext();

  let { $post: _post, $user: _user, doc, editor }: Props = $props();

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_Panel_PanelInfo_post on Post {
        id

        type

        ...Editor_Panel_PanelPost_post
      }
    `),
  );

  const anchorCount = $derived(Object.keys(editor?.current?.storage.anchors?.current || {}).length);
</script>

<div class={flex({ width: 'full', height: '40px', alignItems: 'center', gap: '4px', padding: '4px', flexShrink: '0' })}>
  <PanelSubTab label={$post.type === PostType.TEMPLATE ? '템플릿' : '포스트'} tab="post" />
  <VerticalDivider style={css.raw({ height: '20px', marginY: 'auto' })} />
  <PanelSubTab badge={anchorCount} label="북마크" tab="anchors" />
</div>

<HorizontalDivider color="secondary" />

<div class={flex({ overflowY: 'auto', flexGrow: '1' })}>
  {#if app.preference.current.panelInfoTab === 'post'}
    <PanelPost {$post} $user={_user} {doc} {editor} />
  {/if}

  {#if app.preference.current.panelInfoTab === 'anchors'}
    <PanelAnchors {doc} {editor} />
  {/if}
</div>
