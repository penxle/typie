<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import { base64 } from 'rfc4648';
  import { onMount } from 'svelte';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { PostContentSyncKind } from '@/enums';
  import { browser } from '$app/environment';
  import { fragment, graphql } from '$graphql';
  import { autosize } from '$lib/actions';
  import { Button, Helmet, HorizontalDivider } from '$lib/components';
  import { TiptapEditor } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import TopBar from '../TopBar.svelte';
  import { YState } from './state.svelte';
  import Toolbar from './Toolbar.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Editor_query } from '$graphql';
  import type { Ref } from '$lib/utils';

  type Props = {
    $query: Editor_query;
  };

  let { $query: _query }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment Editor_query on Query {
        me @required {
          id
          name
        }

        post(slug: $slug) {
          id

          entity {
            id
            slug

            ancestors {
              id

              node {
                __typename

                ... on Folder {
                  id
                  name
                }
              }
            }

            site {
              id
              url
            }
          }

          content {
            id
            update
          }
        }
      }
    `),
  );

  const syncPostContent = graphql(`
    mutation Editor_SyncPostContent_Mutation($input: SyncPostContentInput!) {
      syncPostContent(input: $input) {
        kind
        data
      }
    }
  `);

  const postContentSyncStream = graphql(`
    subscription Editor_PostContentSyncStream_Subscription($postId: ID!) {
      postContentSyncStream(postId: $postId) {
        postId
        kind
        data
      }
    }
  `);

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();

  let editor = $state<Ref<Editor>>();

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);

  const title = new YState(doc, 'title');
  const subtitle = new YState(doc, 'subtitle');

  const effectiveTitle = $derived(title.current || '(제목 없음)');

  doc.on('updateV2', async (update, origin) => {
    if (!browser || origin === 'remote') {
      return;
    }

    await syncPostContent({
      postId: $query.post.id,
      kind: PostContentSyncKind.UPDATE,
      data: base64.stringify(update),
    });
  });

  awareness.on('update', async (states: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
    if (!browser || origin === 'remote') {
      return;
    }

    const update = YAwareness.encodeAwarenessUpdate(awareness, [...states.added, ...states.updated, ...states.removed]);

    await syncPostContent({
      postId: $query.post.id,
      kind: PostContentSyncKind.AWARENESS,
      data: base64.stringify(update),
    });
  });

  postContentSyncStream.on('data', ({ postContentSyncStream: { postId, kind, data } }) => {
    if (postId !== $query.post.id) {
      return;
    }

    if (kind === PostContentSyncKind.UPDATE) {
      Y.applyUpdateV2(doc, base64.parse(data), 'remote');
    } else if (kind === PostContentSyncKind.AWARENESS) {
      YAwareness.applyAwarenessUpdate(awareness, base64.parse(data), 'remote');
      // } else if (kind === PostContentSyncKind.HEARTBEAT) {
    }
  });

  const forceSync = async () => {
    const clientStateVector = Y.encodeStateVector(doc);

    const results = await syncPostContent({
      postId: $query.post.id,
      kind: PostContentSyncKind.VECTOR,
      data: base64.stringify(clientStateVector),
    });

    for (const { kind, data } of results) {
      if (kind === PostContentSyncKind.VECTOR) {
        const serverStateVector = base64.parse(data);
        const serverMissingUpdate = Y.encodeStateAsUpdateV2(doc, serverStateVector);

        await syncPostContent({
          postId: $query.post.id,
          kind: PostContentSyncKind.UPDATE,
          data: base64.stringify(serverMissingUpdate),
        });
      } else if (kind === PostContentSyncKind.UPDATE) {
        const clientMissingUpdate = base64.parse(data);
        Y.applyUpdateV2(doc, clientMissingUpdate, 'remote');
      }
    }
  };

  onMount(() => {
    const persistence = new IndexeddbPersistence(`typie:editor:${$query.post.id}`, doc);
    persistence.on('synced', () => forceSync());

    const unsubscribe = postContentSyncStream.subscribe({ postId: $query.post.id });

    Y.applyUpdateV2(doc, base64.parse($query.post.content.update), 'remote');
    awareness.setLocalStateField('user', {
      name: $query.me.name,
      color: random({ luminosity: 'bright', seed: stringHash($query.me.id) }).toHexString(),
    });

    const forceSyncInterval = setInterval(() => forceSync(), 10_000);

    return () => {
      clearInterval(forceSyncInterval);

      unsubscribe();

      YAwareness.removeAwarenessStates(awareness, [doc.clientID], 'local');

      persistence.destroy();
      awareness.destroy();
      doc.destroy();
    };
  });
</script>

<Helmet title={`${effectiveTitle} 작성 중`} />

<TopBar>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      {#each $query.post.entity.ancestors as ancestor (ancestor.id)}
        {#if ancestor.node.__typename === 'Folder'}
          <div>{ancestor.node.name}</div>
          <div>/</div>
        {/if}
      {/each}

      <div>
        {effectiveTitle}
      </div>
    </div>

    <div>
      <Button external href={`${$query.post.entity.site.url}/${$query.post.entity.slug}`} size="sm" type="link" variant="secondary">
        공유
      </Button>
    </div>
  </div>
</TopBar>

<HorizontalDivider />

<div class={flex({ flexDirection: 'column', alignItems: 'center', flexGrow: '1', overflow: 'hidden' })}>
  <Toolbar {editor} />

  <div
    class={flex({
      flexDirection: 'column',
      alignItems: 'center',
      flexGrow: '1',
      width: 'full',
      backgroundColor: 'gray.100',
      overflow: 'scroll',
    })}
  >
    <div
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        flexGrow: '1',
        paddingY: '40px',
        width: 'full',
        maxWidth: '1200px',
        backgroundColor: 'white',
      })}
    >
      <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: '1000px' })}>
        <textarea
          bind:this={titleEl}
          class={css({ width: 'full', fontSize: '28px', fontWeight: 'bold', resize: 'none' })}
          maxlength="100"
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === 'ArrowDown') {
              e.preventDefault();
              subtitleEl?.focus();
            }
          }}
          placeholder="제목을 입력하세요"
          rows={1}
          bind:value={title.current}
          use:autosize
        ></textarea>

        <textarea
          bind:this={subtitleEl}
          class={css({ marginTop: '4px', width: 'full', fontSize: '16px', fontWeight: 'medium', resize: 'none' })}
          maxlength="100"
          onkeydown={(e) => {
            if (e.key === 'ArrowUp' || (e.key === 'Backspace' && !subtitleEl?.value)) {
              e.preventDefault();
              titleEl?.focus();
            }

            if (e.key === 'Enter' || e.key === 'ArrowDown' || (e.key === 'Tab' && !e.shiftKey)) {
              e.preventDefault();
              editor?.current.chain().focus().setTextSelection(2).run();
            }
          }}
          placeholder="부제목을 입력하세요"
          rows={1}
          bind:value={subtitle.current}
          use:autosize
        ></textarea>

        <HorizontalDivider style={css.raw({ marginTop: '10px', marginBottom: '20px' })} />
      </div>

      <TiptapEditor
        style={{ flexGrow: '1', width: 'full' }}
        {awareness}
        {doc}
        onkeydown={(view, e) => {
          const { doc, selection } = view.state;
          const { anchor } = selection;

          if (
            ((e.key === 'ArrowUp' || (e.key === 'Tab' && e.shiftKey)) && anchor === 2) ||
            (e.key === 'Backspace' && doc.child(0).childCount === 1 && doc.child(0).child(0).childCount === 0)
          ) {
            e.preventDefault();
            subtitleEl?.focus();
          }
        }}
        bind:editor
      />
    </div>
  </div>
</div>
