<script lang="ts">
  import { base64 } from 'rfc4648';
  import { onMount } from 'svelte';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { PostContentSyncKind } from '@/enums';
  import { browser } from '$app/environment';
  import { graphql } from '$graphql';
  import { TiptapEditor } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Toolbar from '../Toolbar.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  const query = graphql(`
    query EditorPostIdPage_Query($postId: ID!) {
      post(postId: $postId) {
        id

        content {
          id
          update
        }
      }
    }
  `);

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

  let editor = $state<Ref<Editor>>();

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);

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
      name: 'Anonymous',
      color: '#000000',
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

<div class={flex({ direction: 'column', alignItems: 'center', gap: '24px', paddingY: '100px', width: 'screen', height: 'screen' })}>
  {#if editor}
    <Toolbar {editor} />
  {/if}

  <div class={css({ width: 'full', flexGrow: 1 })}>
    <TiptapEditor style={{ height: 'full' }} {awareness} {doc} bind:editor />
  </div>
</div>
