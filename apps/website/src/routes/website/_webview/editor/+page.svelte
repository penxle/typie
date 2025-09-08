<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import { getText } from '@tiptap/core';
  import { Mark } from '@tiptap/pm/model';
  import { NodeSelection, Selection, TextSelection } from '@tiptap/pm/state';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { autosize } from '@typie/ui/actions';
  import { EditorLayout, EditorZoom } from '@typie/ui/components';
  import { getIncompatibleBlocks, getNodeViewByNodeId, setupEditorContext, TiptapEditor } from '@typie/ui/tiptap';
  import { clamp, createDefaultPageLayout, PAGE_SIZE_MAP } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import stringify from 'fast-json-stable-stringify';
  import { nanoid } from 'nanoid';
  import { base64 } from 'rfc4648';
  import { onMount, untrack } from 'svelte';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import { defaultDeleteFilter, defaultProtectedNodes, ySyncPluginKey } from 'y-prosemirror';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { PostLayoutMode, PostSyncType } from '@/enums';
  import { textSerializers } from '@/pm/serializer';
  import { browser } from '$app/environment';
  import { graphql } from '$graphql';
  import { unfurlEmbed, uploadBlobAsFile, uploadBlobAsImage } from '$lib/utils';
  import Anchors from './Anchors.svelte';
  import { handleCaretMovement } from './caret';
  import FindReplace from './FindReplace.svelte';
  import Highlight from './Highlight.svelte';
  import Limit from './Limit.svelte';
  import Spellcheck from './Spellcheck.svelte';
  import { YState } from './state.svelte';
  import type { Editor } from '@tiptap/core';
  import type { PageLayout, PageLayoutPreset, Ref } from '@typie/ui/utils';

  const WEBVIEW_DISCONNECT_THRESHOLD = 10;

  const query = graphql(`
    query WebViewEditorPage_Query($slug: String!, $siteId: ID!) {
      ...WebViewEditor_Limit_query

      me @required {
        id
        name

        subscription {
          id

          plan {
            id

            rule {
              maxTotalCharacterCount
              maxTotalBlobSize
            }
          }
        }
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

              family {
                id
              }
            }
          }
        }
      }

      site(siteId: $siteId) {
        id

        usage {
          totalCharacterCount
          totalBlobSize
        }
      }
    }
  `);

  const postQuery = graphql(`
    query WebViewEditorPage_Post_Query($slug: String!) @client {
      post(slug: $slug) {
        id
        body
        maxWidth
        storedMarks
        layoutMode
        pageLayout
      }
    }
  `);

  const syncPost = graphql(`
    mutation WebViewEditor_SyncPost_Mutation($input: SyncPostInput!) {
      syncPost(input: $input)
    }
  `);

  const viewEntity = graphql(`
    mutation WebViewEditor_ViewEntity_Mutation($input: ViewEntityInput!) {
      viewEntity(input: $input) {
        id
      }
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

  const siteUsageUpdateStream = graphql(`
    subscription WebViewEditor_SiteUsageUpdateStream($siteId: ID!) {
      siteUsageUpdateStream(siteId: $siteId) {
        ... on Site {
          id

          usage {
            totalCharacterCount
            totalBlobSize
          }
        }
      }
    }
  `);

  setupEditorContext();

  const clientId = nanoid();
  let editor = $state<Ref<Editor>>();
  let connectionStatus = $state<'connecting' | 'connected' | 'disconnected'>('connecting');
  let lastHeartbeatAt = $state(dayjs());
  let lastAppActiveAt = $state(dayjs());

  let mounted = $state(false);

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();

  let scrollContainer = $state<HTMLDivElement>();

  let editorScale = $state(1);
  let editorZoomed = $state(false);

  $effect(() => {
    if (editor?.current && editor.current.storage?.page?.scale !== editorScale) {
      editor.current.chain().setPageScale(editorScale).run();
    }
  });

  let features = $state<string[]>([]);
  let settings = $state<{
    lineHighlightEnabled?: boolean;
    typewriterEnabled?: boolean;
    typewriterPosition?: number;
  }>({});

  const doc = new Y.Doc();
  const awareness = new YAwareness.Awareness(doc);
  const undoManager = new Y.UndoManager([doc.getMap('attrs'), doc.getXmlFragment('body')], {
    trackedOrigins: new Set([ySyncPluginKey, 'local']),
    captureTransaction: (tr) => tr.meta.get('addToHistory') !== false,
    deleteFilter: (item) => defaultDeleteFilter(item, defaultProtectedNodes),
  });

  const title = new YState<string>(doc, 'title', '');
  const subtitle = new YState<string>(doc, 'subtitle', '');
  const maxWidth = new YState<number>(doc, 'maxWidth', 800);
  const storedMarks = new YState<unknown[]>(doc, 'storedMarks', []);
  const note = new YState(doc, 'note', '');
  const pageLayout = new YState<PageLayout | undefined>(doc, 'pageLayout', undefined);
  const layoutMode = new YState<PostLayoutMode>(doc, 'layoutMode', PostLayoutMode.SCROLL);

  const fontFaces = $derived(
    $query.post.entity.site.fonts
      .flatMap((font) => [
        `@font-face { font-family: ${font.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
        `@font-face { font-family: ${font.family.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
      ])
      .join('\n'),
  );

  let syncUpdateTimeout: NodeJS.Timeout | null = null;
  let pendingUpdate: Uint8Array | null = null;

  doc.on('updateV2', async (update, origin) => {
    if (browser && origin !== 'remote') {
      if (pendingUpdate) {
        pendingUpdate = Y.mergeUpdatesV2([pendingUpdate, update]);
      } else {
        pendingUpdate = update;
      }

      if (syncUpdateTimeout) {
        clearTimeout(syncUpdateTimeout);
      }

      syncUpdateTimeout = setTimeout(async () => {
        if (pendingUpdate) {
          await syncPost(
            {
              clientId,
              postId: $query.post.id,
              type: PostSyncType.UPDATE,
              data: base64.stringify(pendingUpdate),
            },
            { transport: 'ws' },
          );

          pendingUpdate = null;
        }
      }, 1000);
    }
  });

  let syncAwarenessTimeout: NodeJS.Timeout | null = null;
  let pendingAwarenessStates: { added: number[]; updated: number[]; removed: number[] } | null = null;

  awareness.on('update', async (states: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
    if (browser && origin !== 'remote') {
      if (pendingAwarenessStates) {
        pendingAwarenessStates = {
          added: [...new Set([...pendingAwarenessStates.added, ...states.added])],
          updated: [...new Set([...pendingAwarenessStates.updated, ...states.updated])],
          removed: [...new Set([...pendingAwarenessStates.removed, ...states.removed])],
        };
      } else {
        pendingAwarenessStates = states;
      }

      if (syncAwarenessTimeout) {
        clearTimeout(syncAwarenessTimeout);
      }

      syncAwarenessTimeout = setTimeout(async () => {
        if (pendingAwarenessStates) {
          const update = YAwareness.encodeAwarenessUpdate(awareness, [
            ...pendingAwarenessStates.added,
            ...pendingAwarenessStates.updated,
            ...pendingAwarenessStates.removed,
          ]);

          await syncPost(
            {
              clientId,
              postId: $query.post.id,
              type: PostSyncType.AWARENESS,
              data: base64.stringify(update),
            },
            { transport: 'ws' },
          );

          pendingAwarenessStates = null;
        }
      }, 1000);
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

  const setYJSState = () => {
    window.__webview__?.emitEvent('setYJSState', {
      maxWidth: maxWidth.current,
      note: note.current,
      layoutMode: layoutMode.current,
      pageLayout: pageLayout.current,
    });
  };

  $effect(() => {
    window.__webview__?.emitEvent('connectionStatus', connectionStatus);
  });

  $effect(() => {
    const el = scrollContainer;
    if (!el) return;

    let ticking = false;
    const handleScroll = () => {
      if (!ticking) {
        ticking = true;
        requestAnimationFrame(() => {
          ticking = false;
          window.__webview__?.emitEvent('scrollTop', el.scrollTop);
        });
      }
    };

    el.addEventListener('scroll', handleScroll, { passive: true });

    return () => {
      el.removeEventListener('scroll', handleScroll);
    };
  });

  $effect(() => {
    if (layoutMode.current === PostLayoutMode.PAGE && pageLayout.current) {
      untrack(() => {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        editor?.current.commands.setPageLayout(pageLayout.current!);
      });
    } else {
      untrack(() => {
        editor?.current.commands.clearPageLayout();
      });
    }
  });

  onMount(() => {
    viewEntity({ entityId: $query.post.entity.id });

    const heartbeatCheckInterval = setInterval(() => {
      const lastActiveTime = dayjs.max(lastHeartbeatAt, lastAppActiveAt);
      if (dayjs().diff(lastActiveTime, 'seconds') > WEBVIEW_DISCONNECT_THRESHOLD) {
        connectionStatus = 'disconnected';
      }
    }, 1000);

    const handleOnline = () => {
      const isFresh = dayjs().diff(lastHeartbeatAt, 'seconds') <= WEBVIEW_DISCONNECT_THRESHOLD;
      if (isFresh) {
        connectionStatus = 'connected';
      } else {
        connectionStatus = 'connecting';
      }
    };

    const handleOffline = () => {
      connectionStatus = 'disconnected';
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    if (!navigator.onLine) {
      connectionStatus = 'disconnected';
    }

    const unsubscribe = postSyncStream.subscribe({ clientId, postId: $query.post.id }, async (payload) => {
      if (payload.type === PostSyncType.HEARTBEAT) {
        lastHeartbeatAt = dayjs(payload.data);
        connectionStatus = 'connected';
      } else if (payload.type === PostSyncType.UPDATE) {
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

    const unsubscribe2 = siteUsageUpdateStream.subscribe({ siteId: $query.site.id });

    const persistence = new IndexeddbPersistence(`typie:editor:${$query.post.id}`, doc);
    persistence.on('synced', () => forceSync());

    Y.applyUpdateV2(doc, base64.parse($query.post.update), 'remote');

    if (![PostLayoutMode.SCROLL, PostLayoutMode.PAGE].includes(layoutMode.current)) {
      layoutMode.current = PostLayoutMode.SCROLL;
    }

    awareness.setLocalStateField('user', {
      name: $query.me.name,
      color: random({ luminosity: 'bright', seed: stringHash($query.me.id) }).toHexString(),
    });

    if (editor) {
      const { tr, schema } = editor.current.state;
      tr.setSelection(TextSelection.create(tr.doc, 2));
      tr.setStoredMarks(storedMarks.current.map((mark) => Mark.fromJSON(schema, mark)));
      editor.current.view.dispatch(tr);
    }

    const forceSyncInterval = setInterval(() => forceSync(), 10_000);

    const arrayOrNull = <T,>(array: T[] | readonly T[] | null | undefined) => (array?.length ? array : null);

    const handler = ({ editor }: { editor: Editor }) => {
      const { doc, selection, storedMarks: storedMarks_ } = editor.state;
      const { $anchor: anchor, $head: head, empty, from, to } = selection;

      // NOTE: tiptap core isNodeActive에서 사용하는 것과 동일하게 nodeRanges를 구함
      const getNodeRanges = () => {
        const nodeRanges: { type: string; attrs?: Record<string, unknown>; from: number; to: number }[] = [];
        doc.nodesBetween(from, to, (node, pos) => {
          if (node.isText) {
            return;
          }

          const relativeFrom = Math.max(from, pos);
          const relativeTo = Math.min(to, pos + node.nodeSize);

          nodeRanges.push({
            type: node.type.name,
            attrs: node.attrs,
            from: relativeFrom,
            to: relativeTo,
          });
        });

        return nodeRanges;
      };

      // NOTE: tiptap core getMarkAttributes에서 사용하는 것과 동일하게 marks를 구함
      const getMarks = () => {
        const marks = [];
        if (empty) {
          if (storedMarks_) {
            marks.push(...storedMarks_);
          }
          marks.push(...head.marks());
        } else {
          doc.nodesBetween(from, to, (node) => {
            marks.push(...node.marks);
          });
        }
        return marks;
      };

      window.__webview__?.emitEvent('setProseMirrorState', {
        nodes: Array.from({ length: anchor.depth + 1 }, (_, i) => anchor.before(i + 1))
          .map((pos) => [pos, doc.nodeAt(pos)] as const)
          .filter(([, node]) => !!node && !node.isText)
          .map(([pos, node]) => ({ pos, type: node?.type.name, attrs: node?.attrs })),
        nodeRanges: getNodeRanges(),
        marks: getMarks().map((mark) => mark.toJSON()),
        storedMarks: editor.state.storedMarks?.map((mark) => mark.toJSON()),
        selection: {
          ...editor.state.selection.toJSON(),
          from: editor.state.selection.from,
          to: editor.state.selection.to,
        },
      });

      const text = getText(editor.state.doc, {
        blockSeparator: '\n',
        textSerializers,
      });

      const countWithWhitespace = [...text.replaceAll(/\s+/g, ' ').trim()].length;
      const countWithoutWhitespace = [...text.replaceAll(/\s/g, '').trim()].length;
      const countWithoutWhitespaceAndPunctuation = [...text.replaceAll(/[\s\p{P}]/gu, '').trim()].length;

      window.__webview__?.emitEvent('setCharacterCountState', {
        countWithWhitespace,
        countWithoutWhitespace,
        countWithoutWhitespaceAndPunctuation,
      });

      const marks =
        arrayOrNull(storedMarks_) || arrayOrNull(anchor.marks()) || arrayOrNull(anchor.parent.firstChild?.firstChild?.marks) || [];

      const jsonMarks = marks.map((mark) => mark.toJSON());

      if (stringify(storedMarks.current) !== stringify(jsonMarks)) {
        storedMarks.current = jsonMarks;
      }
    };

    editor?.current.on('transaction', handler);

    doc.getMap('attrs').observe(setYJSState);

    window.__webview__?.addEventListener('appResumed', () => {
      lastAppActiveAt = dayjs();
      connectionStatus = 'connecting';
    });

    window.__webview__?.addEventListener('appReady', (data) => {
      lastAppActiveAt = dayjs();
      features = data.features || [];
      settings = data.settings || {};
      const isFocusable = features.includes('focusable') ? data.focusable : true;

      if (editor) {
        editor.current.storage.webviewFeatures = features;
      }

      if (settings.typewriterEnabled && settings.typewriterPosition !== undefined) {
        if (editor) {
          editor.current.storage.typewriter = { position: settings.typewriterPosition };
        }
      } else {
        if (editor) {
          editor.current.storage.typewriter = { position: undefined };
        }
      }

      if (data.state?.selection) {
        if (data.state.selection.type === 'element') {
          if (data.state.selection.element === 'title') {
            if (isFocusable) titleEl?.focus();
          } else if (data.state.selection.element === 'subtitle') {
            // eslint-disable-next-line unicorn/no-lonely-if
            if (isFocusable) subtitleEl?.focus();
          }
        } else {
          if (editor) {
            try {
              const selection = Selection.fromJSON(editor.current.state.doc, data.state.selection);
              editor.current.commands.command(({ tr, dispatch }) => {
                tr.setSelection(selection);
                tr.setMeta('initialSelection', true);
                dispatch?.(tr);
                return true;
              });
            } catch {
              editor?.current.commands.setTextSelection(2);
            }

            if (isFocusable) {
              editor.current.commands.focus(null, { scrollIntoView: false });
            }

            let resized = false;
            let fontLoaded = false;
            let scrolled = false;

            window.addEventListener('resize', () => {
              setTimeout(() => {
                resized = true;

                if (resized && fontLoaded && !scrolled) {
                  editor?.current.commands.scrollIntoViewFixed({ position: settings.typewriterPosition ?? 0.25 });
                  scrolled = true;
                }
              }, 100);
            });

            document.fonts.ready.then(() => {
              fontLoaded = true;

              if (resized && fontLoaded && !scrolled) {
                editor?.current.commands.scrollIntoViewFixed({ position: settings.typewriterPosition ?? 0.25 });
                scrolled = true;
              }
            });
          }
        }
      } else {
        editor?.current.commands.setTextSelection(2);
        if (isFocusable) titleEl?.focus();
      }
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
      } else if (name === 'clearFormatting') {
        editor?.current.chain().focus().clearFormatting().run();
      } else if (name === 'text_style') {
        let chain = editor?.current.chain().focus();

        if (attrs.fontFamily !== undefined) {
          chain = chain?.setFontFamily(attrs.fontFamily);
        }

        if (attrs.fontWeight !== undefined) {
          chain = chain?.setFontWeight(attrs.fontWeight);
        }

        if (attrs.fontSize !== undefined) {
          chain = chain?.setFontSize(attrs.fontSize);
        }

        if (attrs.textColor !== undefined) {
          chain = chain?.setTextColor(attrs.textColor);
        }

        if (attrs.textBackgroundColor !== undefined) {
          chain = chain?.setTextBackgroundColor(attrs.textBackgroundColor);
        }

        chain?.run();
      } else if (name === 'link') {
        if (!editor) return;

        const { selection, doc } = editor.current.state;
        const { from, to } = attrs.selection || selection;

        if (attrs.selection) {
          editor.current.chain().setTextSelection({ from, to }).run();
        }

        const marks = doc.resolve(from).marks();
        const hasLinkMark = marks.some((mark) => mark.type.name === 'link');

        if (hasLinkMark) {
          editor.current.chain().focus().updateLink(attrs.url).run();
        } else {
          editor.current.chain().focus().setLink(attrs.url).run();
        }
      } else if (name === 'ruby') {
        if (!editor) return;

        const { selection, doc } = editor.current.state;
        const { from, to } = attrs.selection || selection;

        if (attrs.selection) {
          editor.current.chain().setTextSelection({ from, to }).run();
        }

        const marks = doc.resolve(from).marks();
        const hasRubyMark = marks.some((mark) => mark.type.name === 'ruby');

        if (hasRubyMark) {
          editor.current.chain().focus().updateRuby(attrs.text).run();
        } else {
          editor.current.chain().focus().setRuby(attrs.text).run();
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
        editor?.current.chain().focus().setHorizontalRule(attrs.type).run();
      } else if (name === 'blockquote') {
        editor?.current.chain().focus().toggleBlockquote(attrs.type).run();
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
      } else if (name === 'sink_list_item') {
        editor?.current.chain().focus().sinkListItem('list_item').run();
      } else if (name === 'lift_list_item') {
        editor?.current.chain().focus().liftListItem('list_item').run();
      } else if (name === 'code_block') {
        editor?.current.chain().focus().setCodeBlock().run();
      } else if (name === 'html_block') {
        editor?.current.chain().focus().setHtmlBlock().run();
      } else if (name === 'undo') {
        undoManager.undo();
      } else if (name === 'redo') {
        undoManager.redo();
      } else if (name === 'delete') {
        editor?.current.chain().focus().deleteSelection().run();
      } else if (name === 'select_upward_node') {
        if (!editor || !attrs || !attrs.nodeType) return;

        editor.current.chain().focus().selectUpwardNode(attrs.nodeType).run();
      } else if (name === 'unwrap_node') {
        if (!editor || !attrs || !attrs.nodeType) return;

        editor.current.chain().focus().unwrapNode(attrs.nodeType).run();
      } else if (name === 'set_node') {
        if (!editor || !attrs || !attrs.nodeType) return;

        editor.current.chain().focus().setNode(attrs.nodeType).run();
      } else if (name === 'update_node_attribute') {
        if (!editor || !attrs || !attrs.nodeType) return;

        const { nodeType, ...rest } = attrs;
        editor.current.chain().focus().updateAttributes(nodeType, rest).run();
      } else if (name === 'max_width') {
        maxWidth.current = attrs.maxWidth;
      } else if (name === 'body') {
        if (attrs.paragraphIndent !== undefined) {
          editor?.current.chain().focus().setBodyParagraphIndent(attrs.paragraphIndent).run();
        }
        if (attrs.blockGap !== undefined) {
          editor?.current.chain().focus().setBodyBlockGap(attrs.blockGap).run();
        }
      } else if (name === 'note') {
        note.current = attrs.note;
      }
    });

    window.__webview__?.addEventListener('setLayoutMode', (data) => {
      const mode = data.mode as PostLayoutMode;
      const convertIncompatibleBlocksFlag = data.convertIncompatibleBlocks as boolean;

      if (convertIncompatibleBlocksFlag && editor?.current) {
        editor.current.chain().focus().convertIncompatibleBlocks().run();
      }

      layoutMode.current = mode;

      if (mode === PostLayoutMode.PAGE && !pageLayout.current) {
        const preset = data.preset || 'a4';
        if (Object.keys(PAGE_SIZE_MAP).includes(preset)) {
          pageLayout.current = createDefaultPageLayout(preset as PageLayoutPreset);
        } else {
          pageLayout.current = createDefaultPageLayout('a4');
        }
      }
    });

    window.__webview__?.addEventListener('setPageLayout', (data) => {
      const { width, height, marginTop, marginBottom, marginLeft, marginRight } = data;

      pageLayout.current = {
        width: width ?? pageLayout.current?.width,
        height: height ?? pageLayout.current?.height,
        marginTop: marginTop ?? pageLayout.current?.marginTop,
        marginBottom: marginBottom ?? pageLayout.current?.marginBottom,
        marginLeft: marginLeft ?? pageLayout.current?.marginLeft,
        marginRight: marginRight ?? pageLayout.current?.marginRight,
      };
    });

    window.__webview__?.setProcedure('getIncompatibleBlocks', () => {
      if (!editor?.current) return [];
      return getIncompatibleBlocks(editor.current);
    });

    window.__webview__?.setProcedure('convertIncompatibleBlocks', () => {
      if (!editor?.current) return false;
      editor.current.chain().focus().convertIncompatibleBlocks().run();
      return true;
    });

    window.__webview__?.setProcedure('insertNodes', (params) => {
      const currentEditor = editor?.current;
      if (!currentEditor) return [];

      const nodes = params?.nodes || [];
      if (nodes.length === 0) return [];

      const insertedPositions: number[] = [];

      currentEditor
        .chain()
        .focus()
        .command(({ tr, state: commandState }) => {
          const { selection } = commandState;
          let insertPos = selection.to;

          nodes.forEach((nodeData: { type: string; attrs?: Record<string, unknown> }) => {
            const nodeType = commandState.schema.nodes[nodeData.type];
            if (!nodeType) {
              throw new Error(`Implementation Error: Unknown node type: ${nodeData.type}`);
            }

            const node = nodeType.create(nodeData.attrs || {});

            if (insertPos <= tr.doc.content.size) {
              tr.insert(insertPos, node);
              insertedPositions.push(insertPos);
              insertPos += node.nodeSize;
            }
          });

          if (insertedPositions.length > 0) {
            // eslint-disable-next-line unicorn/prefer-at
            const lastNodePos = insertedPositions[insertedPositions.length - 1];
            if (lastNodePos !== undefined) {
              const nodeSelection = NodeSelection.create(tr.doc, lastNodePos);
              tr.setSelection(nodeSelection);
            }
          }
          return true;
        })
        .run();

      // (string | null)[]
      return insertedPositions.map((pos) => currentEditor.state.doc.nodeAt(pos)).map((node) => node?.attrs.nodeId ?? null);
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
        handleCaretMovement(editor.current.view, direction);
        setTimeout(() => {
          editor?.current.commands.scrollIntoView();
        }, 0);
      }
    });

    window.__webview__?.addEventListener('loadTemplate', async (data) => {
      const resp = await postQuery.load({ slug: data.slug as string });

      if (!editor) return;

      maxWidth.current = resp.post.maxWidth;
      layoutMode.current = resp.post.layoutMode;
      pageLayout.current = resp.post.pageLayout;
      editor.current.commands.loadTemplate(resp.post);
    });

    return () => {
      clearInterval(forceSyncInterval);
      clearInterval(heartbeatCheckInterval);

      if (syncUpdateTimeout) {
        clearTimeout(syncUpdateTimeout);
      }

      if (syncAwarenessTimeout) {
        clearTimeout(syncAwarenessTimeout);
      }

      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);

      YAwareness.removeAwarenessStates(awareness, [doc.clientID], 'local');
      unsubscribe();
      unsubscribe2();

      editor?.current.off('transaction', handler);
      doc.getMap('attrs').unobserve(setYJSState);

      persistence.destroy();
      awareness.destroy();
      doc.destroy();
    };
  });
</script>

<svelte:head>
  <meta
    name="viewport"
    content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover, interactive-widget=resizes-content"
  />

  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html '<st' + `yle type="text/css">${fontFaces}</st` + 'yle>'}
</svelte:head>

<svelte:window
  onkeydowncapture={(e) => {
    if (e.key === 'Enter' && e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      editor?.current.chain().focus().setHardBreak().run();
    }
  }}
/>

<div
  bind:this={scrollContainer}
  style:--prosemirror-color-selection={token.var('colors.border.strong')}
  class={cx(
    'editor-scroll-container',
    css({
      position: 'relative',
      zIndex: 'ground',
      width: 'full',
      height: '[100dvh]',
      overflow: 'auto',
      WebkitTouchCallout: 'none',
      WebkitOverflowScrolling: 'touch',
      touchAction: 'pan-y',
      '&:has([data-layout="page"])': {
        backgroundColor: 'surface.subtle/50',
        touchAction: editorZoomed ? 'auto' : 'pan-y',
      },
    }),
  )}
>
  <EditorLayout
    style={flex.raw({
      flexDirection: 'column',
      alignItems: 'center',
      paddingTop: '40px',
      userSelect: 'text',
      minWidth: editorZoomed ? 'fit' : 'full',
      size: 'full',
    })}
    class="editor"
    bodyPadding={{
      top: 40,
      x: layoutMode.current === PostLayoutMode.PAGE && pageLayout.current ? 0 : 20,
    }}
    layoutMode={layoutMode.current}
    maxWidth={maxWidth.current}
    pageLayout={pageLayout.current}
    typewriterEnabled={settings.typewriterEnabled}
  >
    <div
      style:width={editorZoomed && layoutMode.current === PostLayoutMode.PAGE
        ? `calc(var(--prosemirror-max-width) * ${editorScale})`
        : '100%'}
      class={flex({
        flexDirection: 'column',
        flexShrink: '0',
        paddingX: '20px',
      })}
    >
      <textarea
        bind:this={titleEl}
        class={css({
          width: 'full',
          fontSize: '20px',
          fontWeight: 'bold',
          textAlign: 'center',
          overflow: 'hidden',
          resize: 'none',
        })}
        autocapitalize="off"
        autocomplete="off"
        autocorrect="off"
        maxlength="100"
        onfocus={() => {
          window.__webview__?.emitEvent('focus', { element: 'title' });
        }}
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
        })}
        autocapitalize="off"
        autocomplete="off"
        autocorrect="off"
        maxlength="100"
        onfocus={() => {
          window.__webview__?.emitEvent('focus', { element: 'subtitle' });
        }}
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
        <div class={css({ marginTop: '40px', width: '120px', height: '1px', backgroundColor: 'interactive.hover' })}></div>
      </div>
    </div>

    <EditorZoom
      style={css.raw({ position: 'relative', flexGrow: '1' })}
      layoutMode={layoutMode.current}
      pageLayout={pageLayout.current}
      {scrollContainer}
      bind:scale={editorScale}
      bind:zoomed={editorZoomed}
    >
      <TiptapEditor
        style={css.raw({ size: 'full' })}
        {awareness}
        {doc}
        onblur={() => {
          window.__webview__?.emitEvent('blur');
        }}
        oncreate={() => {
          mounted = true;
          window.__webview__?.emitEvent('webviewReady');
          setYJSState();
        }}
        onfocus={() => {
          window.__webview__?.emitEvent('focus', { element: 'editor' });
        }}
        storage={{
          uploadBlobAsImage: (file) => {
            return uploadBlobAsImage(file);
          },
          uploadBlobAsFile: (file) => {
            return uploadBlobAsFile(file);
          },
          unfurlEmbed: (url) => {
            return unfurlEmbed({ url });
          },
        }}
        {undoManager}
        bind:editor
      />
      {#if editor && mounted}
        {#if settings.lineHighlightEnabled}
          <Highlight {editor} scale={editorScale} />
        {/if}
        <Limit {$query} {editor} />
        <Spellcheck {editor} />
        <FindReplace {editor} />
        <Anchors {doc} {editor} />
      {/if}
    </EditorZoom>

    {#if editorScale !== 1}
      <div
        class={css({
          position: 'fixed',
          left: '20px',
          bottom: '20px',
          paddingX: '12px',
          paddingY: '8px',
          backgroundColor: 'surface.subtle',
          borderWidth: '1px',
          borderColor: 'border.strong',
          borderRadius: '8px',
          fontSize: '12px',
          color: 'text.default',
        })}
      >
        {Math.round(editorScale * 100)}%
      </div>
    {/if}
  </EditorLayout>
</div>
