<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import { Mark } from '@tiptap/pm/model';
  import { Selection } from '@tiptap/pm/state';
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
  import { getNodeViewByNodeId, TiptapEditor } from '$lib/tiptap';
  import { clamp } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { token } from '$styled-system/tokens';
  import Placeholder from './Placeholder.svelte';
  import { scroll } from './scroll.svelte';
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
      const { doc, selection, storedMarks: storedMarks_ } = editor.state;
      const { $anchor: anchor } = selection;

      window.__webview__?.emitEvent('setProseMirrorState', {
        nodes: Array.from({ length: anchor.depth + 1 }, (_, i) => anchor.before(i + 1))
          .map((pos) => [pos, doc.nodeAt(pos)] as const)
          .filter(([, node]) => !!node && !node.isText)
          .map(([pos, node]) => ({ pos, type: node?.type.name, attrs: node?.attrs })),
        marks: anchor.marks().map((mark) => mark.toJSON()),
        storedMarks: editor.state.storedMarks?.map((mark) => mark.toJSON()),
        selection: editor.state.selection.toJSON(),
      });

      const marks =
        arrayOrNull(storedMarks_) || arrayOrNull(anchor.marks()) || arrayOrNull(anchor.parent.firstChild?.firstChild?.marks) || [];

      const jsonMarks = marks.map((mark) => mark.toJSON());

      if (stringify(storedMarks.current) !== stringify(jsonMarks)) {
        storedMarks.current = jsonMarks;
      }
    };

    editor?.current.on('transaction', handler);

    window.__webview__?.addEventListener('appReady', () => {
      // titleEl?.focus();
    });

    window.__webview__?.addEventListener('command', (data) => {
      const name = data.name as string;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const attrs = data.attrs as Record<string, any>;

      if (name === 'bold') {
        editor?.current.chain().focus().toggleBold().run();
      } else if (name === 'italic') {
        editor?.current.chain().focus().toggleItalic().run();
      } else if (name === 'underline') {
        editor?.current.chain().focus().toggleUnderline().run();
      } else if (name === 'strike') {
        editor?.current.chain().focus().toggleStrike().run();
      } else if (name === 'text_style') {
        if (attrs.fontFamily !== undefined) {
          editor?.current.chain().focus().setFontFamily(attrs.fontFamily).run();
        }
        if (attrs.fontSize !== undefined) {
          editor?.current.chain().focus().setFontSize(attrs.fontSize).run();
        }
        if (attrs.textColor !== undefined) {
          editor?.current.chain().focus().setTextColor(attrs.textColor).run();
        }
      } else if (name === 'paragraph') {
        if (attrs.textAlign !== undefined) {
          editor?.current.chain().focus().setParagraphTextAlign(attrs.textAlign).run();
        }
        if (attrs.lineHeight !== undefined) {
          editor?.current.chain().focus().setParagraphLineHeight(attrs.lineHeight).run();
        }
        if (attrs.letterSpacing !== undefined) {
          editor?.current.chain().focus().setParagraphLetterSpacing(attrs.letterSpacing).run();
        }
      } else if (name === 'image') {
        editor?.current.chain().focus().setImage().run();
      } else if (name === 'file') {
        editor?.current.chain().focus().setFile().run();
      } else if (name === 'embed') {
        editor?.current.chain().focus().setEmbed().run();
      } else if (name === 'horizontal_rule') {
        editor?.current.chain().focus().setHorizontalRule().run();
      } else if (name === 'blockquote') {
        editor?.current.chain().focus().toggleBlockquote().run();
      } else if (name === 'callout') {
        editor?.current.chain().focus().toggleCallout().run();
      } else if (name === 'fold') {
        editor?.current.chain().focus().toggleFold().run();
      } else if (name === 'table') {
        editor?.current.chain().focus().insertTable().run();
      } else if (name === 'bullet_list') {
        editor?.current.chain().focus().toggleBulletList().run();
      } else if (name === 'ordered_list') {
        editor?.current.chain().focus().toggleOrderedList().run();
      } else if (name === 'code_block') {
        editor?.current.chain().focus().setCodeBlock().run();
      } else if (name === 'html_block') {
        editor?.current.chain().focus().setHtmlBlock().run();
      } else if (name === 'undo') {
        editor?.current.chain().focus().undo().run();
      } else if (name === 'redo') {
        editor?.current.chain().focus().redo().run();
      } else if (name === 'delete') {
        editor?.current.chain().focus().deleteSelection().run();
      }
    });

    window.__webview__?.addEventListener('nodeview', (data) => {
      if (!editor) {
        return;
      }

      const nodeId = data.nodeId as string;
      const name = data.name as string;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const detail = data.detail as Record<string, any>;

      const nodeView = getNodeViewByNodeId(editor.current.view, nodeId);
      nodeView?.handle?.(new CustomEvent(name, { detail }));
    });

    window.__webview__?.addEventListener('caret', (data) => {
      const direction = data.direction as number;

      if (document.activeElement === titleEl) {
        const position = clamp(titleEl.selectionStart + direction, 0, titleEl.value.length);
        titleEl.setSelectionRange(position, position);
      } else if (document.activeElement === subtitleEl) {
        const position = clamp(subtitleEl.selectionStart + direction, 0, subtitleEl.value.length);
        subtitleEl.setSelectionRange(position, position);
      } else if (editor?.current.isFocused) {
        editor.current.commands.command(({ state, tr, dispatch }) => {
          const pos = state.doc.resolve(state.selection.anchor + direction);
          const selection = Selection.near(pos, direction);

          tr.setSelection(selection);
          tr.scrollIntoView();

          dispatch?.(tr);

          return true;
        });
      }
    });

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
  {@html '<st' + `yle type="text/css">${fontFaces}</st` + 'yle>'}
</svelte:head>

<div
  class={css({ width: '[100dvw]', height: '[100dvh]', overflow: 'hidden', touchAction: 'none' })}
  onmomentumscrollend={() => {
    document.body.style.caretColor = token('colors.gray.950');
  }}
  onmomentumscrollstart={() => {
    document.body.style.caretColor = 'transparent';
  }}
  use:scroll
>
  <div
    style:--prosemirror-max-width={`${maxWidth.current}px`}
    style:--prosemirror-color-selection={token.var('colors.gray.950')}
    class={flex({
      flexDirection: 'column',
      alignItems: 'center',
      paddingTop: '40px',
      paddingX: '20px',
      width: 'full',
      userSelect: 'text',
      WebkitTouchCallout: 'none',
    })}
  >
    <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: 'var(--prosemirror-max-width)' })}>
      <textarea
        bind:this={titleEl}
        class={css({
          width: 'full',
          fontSize: '20px',
          fontWeight: 'bold',
          textAlign: 'center',
          overflow: 'hidden',
          resize: 'none',
          touchAction: 'none',
        })}
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
        placeholder="제목"
        rows={1}
        spellcheck="false"
        bind:value={title.current}
        use:autosize
      ></textarea>

      <textarea
        bind:this={subtitleEl}
        class={css({
          marginTop: '4px',
          width: 'full',
          fontSize: '16px',
          fontWeight: 'medium',
          textAlign: 'center',
          overflow: 'hidden',
          resize: 'none',
          touchAction: 'none',
        })}
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
        placeholder="부제목"
        rows={1}
        spellcheck="false"
        bind:value={subtitle.current}
        use:autosize
      ></textarea>

      <div class={center()}>
        <div class={css({ marginY: '40px', width: '120px', height: '1px', backgroundColor: 'gray.200' })}></div>
      </div>
    </div>

    <div class={css({ position: 'relative', flexGrow: '1', width: 'full' })}>
      <TiptapEditor
        style={css.raw({ size: 'full' })}
        {awareness}
        {doc}
        oncreate={() => {
          window.__webview__?.emitEvent('webviewReady');
        }}
        bind:editor
      />

      {#if editor}
        <Placeholder {editor} />
      {/if}
    </div>
  </div>
</div>
