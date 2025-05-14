<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import { Mark } from '@tiptap/pm/model';
  import stringify from 'fast-json-stable-stringify';
  import { nanoid } from 'nanoid';
  import { base64 } from 'rfc4648';
  import { onMount } from 'svelte';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { PostSyncType } from '@/enums';
  import { browser } from '$app/environment';
  import { graphql } from '$graphql';
  import { autosize } from '$lib/actions';
  import { HorizontalDivider } from '$lib/components';
  import { TiptapEditor } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Placeholder from './Placeholder.svelte';
  import { YState } from './state.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  const query = graphql(`
    query WebViewEditorPage_Query($slug: String!) {
      me @required {
        id
        name
      }

      post(slug: $slug) {
        id
        update

        entity {
          id

          site {
            id

            fonts {
              id
              url
              weight
            }
          }
        }
      }
    }
  `);

  const syncPost = graphql(`
    mutation WebViewEditor_SyncPost_Mutation($input: SyncPostInput!) {
      syncPost(input: $input)
    }
  `);

  const postSyncStream = graphql(`
    subscription WebViewEditor_PostSyncStream_Subscription($clientId: String!, $postId: ID!) {
      postSyncStream(clientId: $clientId, postId: $postId) {
        postId
        type
        data
      }
    }
  `);

  const clientId = nanoid();

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();

  let editor = $state<Ref<Editor>>();

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);

  const title = new YState<string>(doc, 'title', '');
  const subtitle = new YState<string>(doc, 'subtitle', '');
  const maxWidth = new YState<number>(doc, 'maxWidth', 800);
  const storedMarks = new YState<unknown[]>(doc, 'storedMarks', []);

  const fontFaces = $derived(
    $query.post.entity.site.fonts
      .map(
        (font) =>
          `@font-face { font-family: ${font.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
      )
      .join('\n'),
  );

  doc.on('updateV2', async (update, origin) => {
    if (browser && origin !== 'remote') {
      await syncPost(
        {
          clientId,
          postId: $query.post.id,
          type: PostSyncType.UPDATE,
          data: base64.stringify(update),
        },
        { transport: 'ws' },
      );
    }
  });

  awareness.on('update', async (states: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
    if (browser && origin !== 'remote') {
      const update = YAwareness.encodeAwarenessUpdate(awareness, [...states.added, ...states.updated, ...states.removed]);

      await syncPost(
        {
          clientId,
          postId: $query.post.id,
          type: PostSyncType.AWARENESS,
          data: base64.stringify(update),
        },
        { transport: 'ws' },
      );
    }
  });

  const forceSync = async () => {
    const vector = Y.encodeStateVector(doc);

    await syncPost(
      {
        clientId,
        postId: $query.post.id,
        type: PostSyncType.VECTOR,
        data: base64.stringify(vector),
      },
      { transport: 'ws' },
    );
  };

  onMount(() => {
    const unsubscribe = postSyncStream.subscribe({ clientId, postId: $query.post.id }, async (payload) => {
      if (payload.type === PostSyncType.UPDATE) {
        Y.applyUpdateV2(doc, base64.parse(payload.data), 'remote');
      } else if (payload.type === PostSyncType.VECTOR) {
        const update = Y.encodeStateAsUpdateV2(doc, base64.parse(payload.data));

        await syncPost(
          {
            clientId,
            postId: $query.post.id,
            type: PostSyncType.UPDATE,
            data: base64.stringify(update),
          },
          { transport: 'ws' },
        );
      } else if (payload.type === PostSyncType.AWARENESS) {
        YAwareness.applyAwarenessUpdate(awareness, base64.parse(payload.data), 'remote');
      } else if (payload.type === PostSyncType.PRESENCE) {
        const update = YAwareness.encodeAwarenessUpdate(awareness, [doc.clientID]);

        await syncPost(
          {
            clientId,
            postId: $query.post.id,
            type: PostSyncType.AWARENESS,
            data: base64.stringify(update),
          },
          { transport: 'ws' },
        );
      }
    });

    const persistence = new IndexeddbPersistence(`typie:editor:${$query.post.id}`, doc);
    persistence.on('synced', () => forceSync());

    Y.applyUpdateV2(doc, base64.parse($query.post.update), 'remote');
    awareness.setLocalStateField('user', {
      name: $query.me.name,
      color: random({ luminosity: 'bright', seed: stringHash($query.me.id) }).toHexString(),
    });

    if (editor) {
      const { tr, schema } = editor.current.state;
      tr.setStoredMarks(storedMarks.current.map((mark) => Mark.fromJSON(schema, mark)));
      editor.current.view.dispatch(tr);
    }

    const forceSyncInterval = setInterval(() => forceSync(), 10_000);

    const arrayOrNull = <T,>(array: T[] | readonly T[] | null | undefined) => (array?.length ? array : null);

    const handler = ({ editor }: { editor: Editor }) => {
      const marks =
        arrayOrNull(editor.state.storedMarks) ||
        arrayOrNull(editor.state.selection.$anchor.marks()) ||
        arrayOrNull(editor.state.selection.$anchor.parent.firstChild?.firstChild?.marks) ||
        [];

      const jsonMarks = marks.map((mark) => mark.toJSON());

      if (stringify(storedMarks.current) !== stringify(jsonMarks)) {
        storedMarks.current = jsonMarks;
      }
    };

    editor?.current.on('transaction', handler);

    return () => {
      clearInterval(forceSyncInterval);

      YAwareness.removeAwarenessStates(awareness, [doc.clientID], 'local');
      unsubscribe();

      editor?.current.off('transaction', handler);

      persistence.destroy();
      awareness.destroy();
      doc.destroy();
    };
  });
</script>

<svelte:head>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html '<style type="text/css"' + `>${fontFaces}</` + 'style>'}
</svelte:head>

<div class={css({ height: '[100dvh]', overflowY: 'auto', scrollbarGutter: 'stable' })}>
  <div
    style:--prosemirror-max-width={`${maxWidth.current}px`}
    class={flex({
      flexDirection: 'column',
      alignItems: 'center',
      paddingTop: '24px',
      paddingX: '24px',
      size: 'full',
    })}
  >
    <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: 'var(--prosemirror-max-width)' })}>
      <textarea
        bind:this={titleEl}
        class={css({ width: 'full', fontSize: '24px', fontWeight: 'bold', resize: 'none' })}
        autocapitalize="off"
        autocomplete="off"
        maxlength="100"
        onkeydown={(e) => {
          if (e.isComposing) {
            return;
          }

          if (e.key === 'Enter') {
            e.preventDefault();
            subtitleEl?.focus();
          }
        }}
        placeholder="제목을 입력하세요"
        rows={1}
        spellcheck="false"
        bind:value={title.current}
        use:autosize
      ></textarea>

      <textarea
        bind:this={subtitleEl}
        class={css({ marginTop: '4px', width: 'full', fontSize: '16px', fontWeight: 'medium', overflow: 'hidden', resize: 'none' })}
        autocapitalize="off"
        autocomplete="off"
        maxlength="100"
        onkeydown={(e) => {
          if (e.isComposing) {
            return;
          }

          if (e.key === 'Enter') {
            e.preventDefault();
            const marks = editor?.current.state.storedMarks || editor?.current.state.selection.$anchor.marks() || null;
            editor?.current
              .chain()
              .focus()
              .setTextSelection(2)
              .command(({ tr, dispatch }) => {
                tr.setStoredMarks(marks);
                dispatch?.(tr);
                return true;
              })
              .run();
          }
        }}
        placeholder="부제목을 입력하세요"
        rows={1}
        spellcheck="false"
        bind:value={subtitle.current}
        use:autosize
      ></textarea>

      <HorizontalDivider style={css.raw({ marginTop: '10px', marginBottom: '20px' })} color="secondary" />
    </div>

    <div class={css({ position: 'relative', flexGrow: '1', width: 'full' })}>
      <TiptapEditor
        style={css.raw({ size: 'full' })}
        {awareness}
        {doc}
        oncreate={() => {
          titleEl?.focus();
        }}
        bind:editor
      />

      {#if editor}
        <Placeholder {editor} />
      {/if}
    </div>
  </div>
</div>
