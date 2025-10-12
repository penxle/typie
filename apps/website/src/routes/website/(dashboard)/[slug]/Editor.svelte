<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import { isiOS, isMacOS } from '@tiptap/core';
  import { Selection, Transaction } from '@tiptap/pm/state';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { EditorLayout, EditorZoom, Helmet, HorizontalDivider, Icon, Menu } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Tip } from '@typie/ui/notification';
  import { LocalStore } from '@typie/ui/state';
  import { getNodeView, setupEditorContext, TiptapEditor } from '@typie/ui/tiptap';
  import dayjs from 'dayjs';
  import stringify from 'fast-json-stable-stringify';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { untrack } from 'svelte';
  import { on } from 'svelte/events';
  import { match } from 'ts-pattern';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import { defaultDeleteFilter, defaultProtectedNodes, ySyncPluginKey } from 'y-prosemirror';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { PostLayoutMode, PostSyncType } from '@/enums';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CrownIcon from '~icons/lucide/crown';
  import ElipsisIcon from '~icons/lucide/ellipsis';
  import FolderIcon from '~icons/lucide/folder';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import XIcon from '~icons/lucide/x';
  import { browser } from '$app/environment';
  import { fragment, graphql } from '$graphql';
  import { unfurlEmbed, uploadBlobAsFile, uploadBlobAsImage } from '$lib/utils';
  import PostMenu from '../@context-menu/PostMenu.svelte';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import Anchors from './@anchor/Anchors.svelte';
  import Panel from './@panel/Panel.svelte';
  import PanelNote from './@panel/PanelNote.svelte';
  import CloseSplitView from './@split-view/CloseSplitView.svelte';
  import { getSplitViewContext, getViewContext } from './@split-view/context.svelte';
  import { getDragDropContext } from './@split-view/drag-context.svelte';
  import { dragView } from './@split-view/drag-view-action';
  import { VIEW_BUFFER_SIZE, VIEW_MIN_SIZE } from './@split-view/utils';
  import BottomToolbar from './@toolbar/BottomToolbar.svelte';
  import TopToolbar from './@toolbar/TopToolbar.svelte';
  import FloatingFindReplace from './FloatingFindReplace.svelte';
  import Highlight from './Highlight.svelte';
  import Limit from './Limit.svelte';
  import PasteModal from './PasteModal.svelte';
  import { YState } from './state.svelte';
  import type { Editor } from '@tiptap/core';
  import type { PageLayout, Ref } from '@typie/ui/utils';
  import type { Editor_query } from '$graphql';

  const DISCONNECT_THRESHOLD = 3;

  type Props = {
    $query: Editor_query;
    slug: string;
    focused: boolean;
  };

  let { $query: _query, slug, focused }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment Editor_query on Query {
        ...Editor_Limit_query

        me @required {
          id
          name
          role

          ...Editor_Panel_user
        }

        entities(slugs: $slugs) {
          id
          slug
          url
          visibility
          availability

          ...PanelNote_Notes_entity

          parent {
            id

            children {
              id
              slug

              node {
                __typename

                ... on Post {
                  id
                  title
                }
              }
            }
          }

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

            entities {
              id
              slug

              node {
                __typename

                ... on Post {
                  id
                  title
                }
              }
            }

            ...Editor_Limit_site
            ...Editor_TopToolbar_site
          }

          user {
            id

            subscription {
              id
            }

            ...Editor_BottomToolbar_user
          }

          node {
            __typename

            ... on Post {
              id
              title
              type
              update

              ...Editor_Panel_post
            }
          }
        }
      }
    `),
  );

  const syncPost = graphql(`
    mutation Editor_SyncPost_Mutation($input: SyncPostInput!) {
      syncPost(input: $input)
    }
  `);

  const postSyncStream = graphql(`
    subscription Editor_PostSyncStream_Subscription($clientId: String!, $postId: ID!) {
      postSyncStream(clientId: $clientId, postId: $postId) {
        postId
        type
        data
      }
    }
  `);

  const editorContext = setupEditorContext();

  const app = getAppContext();
  const splitView = getSplitViewContext();
  const splitViewId = getViewContext().id;
  const dragDropContext = getDragDropContext();
  const dragViewProps = $derived({ dragDropContext, viewId: splitViewId });
  const clientId = nanoid();
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  let entity = $state<(typeof $query.entities)[number]>($query.entities.find((entity) => entity.slug === slug)!);

  const selectionsStore = new LocalStore<
    Record<
      string,
      {
        selection?: unknown;
        type?: string;
        element?: string;
        timestamp: number;
      }
    >
  >('typie:selections', {});

  $effect(() => {
    void slug;

    untrack(() => {
      const next = $query.entities.find((entity) => entity.slug === slug);
      if (next) {
        entity = next;
      }
    });
  });

  const postId = $derived(entity.node.__typename === 'Post' ? entity.node.id : null);

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();

  let editor = $state<Ref<Editor>>();
  let viewEditor = $state<Ref<Editor>>();

  let connectionStatus = $state<'connecting' | 'connected' | 'disconnected'>('connecting');
  let lastHeartbeatAt = $state(dayjs());

  let mounted = $state(false);

  let showAnchorOutline = $state(false);

  let clipboardData = $state<{ html: string; text?: string }>();
  let openPasteModal = $state(false);
  let planUpgradeModalOpen = $state(false);

  const doc = new Y.Doc();
  let viewDoc = $state<Y.Doc>();

  const awareness = new YAwareness.Awareness(doc);
  const undoManager = new Y.UndoManager([doc.getMap('attrs'), doc.getXmlFragment('body')], {
    trackedOrigins: new Set([ySyncPluginKey, 'local']),
    captureTransaction: (tr) => tr.meta.get('addToHistory') !== false,
    deleteFilter: (item) => defaultDeleteFilter(item, defaultProtectedNodes),
  });

  const title = new YState<string>(doc, 'title', '');
  const subtitle = new YState<string>(doc, 'subtitle', '');
  const maxWidth = new YState<number>(doc, 'maxWidth', 800);
  const pageLayout = new YState<PageLayout | undefined>(doc, 'pageLayout', undefined);
  const layoutMode = new YState<PostLayoutMode>(doc, 'layoutMode', PostLayoutMode.SCROLL);
  const anchors = new YState<Record<string, string | null>>(doc, 'anchors', {});
  const initialMarks = new YState<unknown[]>(doc, 'initialMarks', []);

  const viewTitle = $derived(viewDoc ? new YState<string>(viewDoc, 'title', '') : undefined);
  const viewSubtitle = $derived(viewDoc ? new YState<string>(viewDoc, 'subtitle', '') : undefined);
  const viewMaxWidth = $derived(viewDoc ? new YState<number>(viewDoc, 'maxWidth', 800) : undefined);
  const viewPageLayout = $derived(viewDoc ? new YState<PageLayout | undefined>(viewDoc, 'pageLayout', undefined) : undefined);
  const viewLayoutMode = $derived(viewDoc ? new YState<PostLayoutMode>(viewDoc, 'layoutMode', PostLayoutMode.SCROLL) : undefined);

  const effectiveTitle = $derived(viewTitle ?? title);
  const effectiveSubtitle = $derived(viewSubtitle ?? subtitle);
  const effectiveMaxWidth = $derived(viewMaxWidth ?? maxWidth);
  const effectivePageLayout = $derived(viewPageLayout ?? pageLayout);
  const effectiveLayoutMode = $derived(viewLayoutMode ?? layoutMode);

  let scrollContainer = $state<HTMLDivElement>();

  let editorScale = $state(1);
  let editorZoomed = $state(false);

  $effect(() => {
    if (editor?.current && editor.current.storage?.page?.scale !== editorScale) {
      editor.current.chain().setPageScale(editorScale).run();
    }
  });

  const persistSelection = ({ transaction }: { transaction: Transaction }) => {
    if (!editor?.current || !postId) return;

    if (transaction.getMeta('initialSelection')) {
      return;
    }

    const { selection } = transaction;

    selectionsStore.current = {
      ...selectionsStore.current,
      [postId]: { ...selection.toJSON(), timestamp: dayjs().valueOf() },
    };
  };

  let syncUpdateTimeout: NodeJS.Timeout | null = null;
  let pendingUpdate: Uint8Array | null = null;
  let lastSyncTime = Date.now();

  let syncAwarenessTimeout: NodeJS.Timeout | null = null;
  let pendingAwarenessStates: { added: number[]; updated: number[]; removed: number[] } | null = null;
  let lastAwarenessSyncTime = Date.now();

  const forceSync = async () => {
    if (!postId) return;

    const vector = Y.encodeStateVector(doc);

    await syncPost(
      {
        clientId,
        postId,
        type: PostSyncType.VECTOR,
        data: vector.toBase64(),
      },
      { transport: 'ws' },
    );
  };

  const fullSync = async () => {
    if (!postId) return;

    const update = Y.encodeStateAsUpdateV2(doc);

    await syncPost({
      clientId,
      postId,
      type: PostSyncType.UPDATE,
      data: update.toBase64(),
    });
  };

  $effect(() => {
    if (app.preference.current.typewriterEnabled && app.preference.current.typewriterPosition !== undefined) {
      untrack(() => {
        if (editor) {
          editor.current.storage.typewriter = { position: app.preference.current.typewriterPosition };
        }
      });
    } else {
      untrack(() => {
        if (editor) {
          editor.current.storage.typewriter = { position: undefined };
        }
      });
    }
  });

  $effect(() => {
    void app.preference.current.autoSurroundEnabled;

    untrack(() => {
      if (editor) {
        editor.current.storage.autoSurround = { enabled: app.preference.current.autoSurroundEnabled };
      }
    });
  });

  const currentViewZenModeEnabled = $derived(
    app.preference.current.zenModeEnabled && splitViewId === splitView.state.current.focusedViewId,
  );

  $effect(() => {
    if (currentViewZenModeEnabled) {
      Tip.show('editor.zen-mode.enabled', '집중 모드가 활성화되었어요. Esc 키를 눌러 빠져나올 수 있어요.');
    }
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

  const onPasteConfirm = (mode: 'html' | 'text') => {
    if (!editor?.current || !clipboardData) {
      return;
    }

    if (mode === 'html') {
      editor.current.view.pasteHTML(clipboardData.html);
    } else if (mode === 'text') {
      if (clipboardData.text) {
        editor.current.view.pasteText(clipboardData.text);
      } else {
        const dom = new DOMParser().parseFromString(clipboardData.html, 'text/html');
        editor.current.view.pasteText(dom.body.textContent);
      }
    }

    clipboardData = undefined;
  };

  $effect(() => {
    if (!postId) return;

    return untrack(() => {
      const currentPostId = postId;

      const handleOnline = () => {
        const isFresh = dayjs().diff(lastHeartbeatAt, 'seconds') <= DISCONNECT_THRESHOLD;
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

      doc.on('updateV2', async (update, origin) => {
        if (browser && origin !== 'remote' && postId === currentPostId) {
          if (pendingUpdate) {
            pendingUpdate = Y.mergeUpdatesV2([pendingUpdate, update]);
          } else {
            pendingUpdate = update;
          }

          if (syncUpdateTimeout) {
            clearTimeout(syncUpdateTimeout);
          }

          const timeSinceLastSync = Date.now() - lastSyncTime;
          const shouldForceSync = timeSinceLastSync >= 100;

          if (shouldForceSync && pendingUpdate) {
            if (postId === currentPostId) {
              await syncPost(
                {
                  clientId,
                  postId,
                  type: PostSyncType.UPDATE,
                  data: pendingUpdate.toBase64(),
                },
                { transport: 'ws' },
              );

              pendingUpdate = null;
              lastSyncTime = Date.now();
            }
          } else {
            const remainingTime = Math.max(0, 100 - timeSinceLastSync);

            syncUpdateTimeout = setTimeout(async () => {
              if (pendingUpdate && postId === currentPostId) {
                await syncPost(
                  {
                    clientId,
                    postId,
                    type: PostSyncType.UPDATE,
                    data: pendingUpdate.toBase64(),
                  },
                  { transport: 'ws' },
                );

                pendingUpdate = null;
                lastSyncTime = Date.now();
              }
            }, remainingTime);
          }
        }
      });

      awareness.on('update', async (states: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
        if (browser && origin !== 'remote' && postId === currentPostId) {
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

          const timeSinceLastSync = Date.now() - lastAwarenessSyncTime;
          const shouldForceSync = timeSinceLastSync >= 100;

          if (shouldForceSync && pendingAwarenessStates) {
            if (postId === currentPostId) {
              const update = YAwareness.encodeAwarenessUpdate(awareness, [
                ...pendingAwarenessStates.added,
                ...pendingAwarenessStates.updated,
                ...pendingAwarenessStates.removed,
              ]);

              await syncPost(
                {
                  clientId,
                  postId,
                  type: PostSyncType.AWARENESS,
                  data: update.toBase64(),
                },
                { transport: 'ws' },
              );

              pendingAwarenessStates = null;
              lastAwarenessSyncTime = Date.now();
            }
          } else {
            const remainingTime = Math.max(0, 100 - timeSinceLastSync);

            syncAwarenessTimeout = setTimeout(async () => {
              if (pendingAwarenessStates && postId === currentPostId) {
                const update = YAwareness.encodeAwarenessUpdate(awareness, [
                  ...pendingAwarenessStates.added,
                  ...pendingAwarenessStates.updated,
                  ...pendingAwarenessStates.removed,
                ]);

                await syncPost(
                  {
                    clientId,
                    postId,
                    type: PostSyncType.AWARENESS,
                    data: update.toBase64(),
                  },
                  { transport: 'ws' },
                );

                pendingAwarenessStates = null;
                lastAwarenessSyncTime = Date.now();
              }
            }, remainingTime);
          }
        }
      });

      const unsubscribe = postSyncStream.subscribe({ clientId, postId: currentPostId }, async (payload) => {
        if (postId !== currentPostId) {
          return;
        }

        if (payload.type === PostSyncType.HEARTBEAT) {
          lastHeartbeatAt = dayjs(payload.data);
          connectionStatus = 'connected';
        } else if (payload.type === PostSyncType.UPDATE) {
          Y.applyUpdateV2(doc, Uint8Array.fromBase64(payload.data), 'remote');
        } else if (payload.type === PostSyncType.VECTOR) {
          const update = Y.encodeStateAsUpdateV2(doc, Uint8Array.fromBase64(payload.data));

          await syncPost(
            {
              clientId,
              postId: currentPostId,
              type: PostSyncType.UPDATE,
              data: update.toBase64(),
            },
            { transport: 'ws' },
          );
        } else if (payload.type === PostSyncType.AWARENESS) {
          YAwareness.applyAwarenessUpdate(awareness, Uint8Array.fromBase64(payload.data), 'remote');
        } else if (payload.type === PostSyncType.PRESENCE) {
          const update = YAwareness.encodeAwarenessUpdate(awareness, [doc.clientID]);

          await syncPost(
            {
              clientId,
              postId: currentPostId,
              type: PostSyncType.AWARENESS,
              data: update.toBase64(),
            },
            { transport: 'ws' },
          );
        }
      });

      const persistence = new IndexeddbPersistence(`typie:editor:${currentPostId}`, doc);

      if (entity.node.__typename === 'Post') {
        Y.applyUpdateV2(doc, Uint8Array.fromBase64(entity.node.update), 'remote');

        if (![PostLayoutMode.SCROLL, PostLayoutMode.PAGE].includes(layoutMode.current)) {
          layoutMode.current = PostLayoutMode.SCROLL;
        }
      }

      awareness.setLocalStateField('user', {
        name: $query.me.name,
        color: random({ luminosity: 'bright', seed: stringHash($query.me.id) }).toHexString(),
      });

      editor?.current.once('create', ({ editor }) => {
        const isDocumentEmpty = editor.state.doc.child(0).childCount === 1 && editor.state.doc.child(0).child(0).childCount === 0;

        if (isDocumentEmpty && initialMarks.current.length > 0) {
          const marks = initialMarks.current.map((markJSON) => editor.schema.markFromJSON(markJSON));
          editor.commands.command(({ tr, dispatch }) => {
            tr.setStoredMarks(marks);
            dispatch?.(tr);
            return true;
          });
        }

        const selections = selectionsStore.current;
        if (currentPostId && selections[currentPostId]) {
          if (selections[currentPostId].type === 'element') {
            if (selections[currentPostId].element === 'title') {
              titleEl?.focus();
            } else if (selections[currentPostId].element === 'subtitle') {
              subtitleEl?.focus();
            }
          } else {
            try {
              const selection = Selection.fromJSON(editor.state.doc, selections[currentPostId]);
              editor.commands.command(({ tr, dispatch }) => {
                tr.setSelection(selection);
                tr.setMeta('initialSelection', true);
                dispatch?.(tr);
                return true;
              });
            } catch {
              // pass
            }

            document.fonts.ready.then(() => {
              editor.commands.focus();
            });
          }
        } else {
          editor.commands.setTextSelection(2);
          titleEl?.focus();
        }
      });

      const fullSyncInterval = setInterval(() => fullSync(), 60_000);
      const forceSyncInterval = setInterval(() => forceSync(), 10_000);
      const heartbeatInterval = setInterval(() => {
        if (dayjs().diff(lastHeartbeatAt, 'seconds') > DISCONNECT_THRESHOLD) {
          connectionStatus = 'disconnected';
        }
      }, 1000);

      const off = on(globalThis.window, 'keydown', async (e) => {
        if (!focused) return;

        if ((e.metaKey || e.ctrlKey) && e.key === 's') {
          e.preventDefault();
          e.stopPropagation();

          forceSync();
          Tip.show('editor.shortcut.save', '따로 저장 키를 누르지 않아도 모든 변경 사항은 실시간으로 저장돼요.');
        }
      });

      const applyInitialMarks = ({ transaction }: { transaction: Transaction }) => {
        if (!editor?.current) return;
        if (transaction.getMeta('applyInitialMarks')) return;

        const currentEditor = editor.current;

        const isDocumentEmpty =
          currentEditor.state.doc.child(0).childCount === 1 && currentEditor.state.doc.child(0).child(0).childCount === 0;

        if (!isDocumentEmpty) return;

        const currentStoredMarks = transaction.storedMarks;

        if (!currentStoredMarks && initialMarks.current.length > 0) {
          const marks = initialMarks.current.map((markJSON) => currentEditor.schema.markFromJSON(markJSON));
          currentEditor.commands.command(({ tr, dispatch }) => {
            tr.setStoredMarks(marks);
            tr.setMeta('applyInitialMarks', true);
            dispatch?.(tr);
            return true;
          });
        } else if (currentStoredMarks) {
          const newInitialMarks = currentStoredMarks.map((mark) => mark.toJSON());
          if (stringify(newInitialMarks) !== stringify(initialMarks.current)) {
            initialMarks.current = newInitialMarks;
          }
        }
      };

      editor?.current.on('selectionUpdate', persistSelection);
      editor?.current.on('transaction', applyInitialMarks);

      fullSync();

      return () => {
        off();

        clearInterval(fullSyncInterval);
        clearInterval(forceSyncInterval);
        clearInterval(heartbeatInterval);

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

        editor?.current.off('selectionUpdate', persistSelection);
        editor?.current.off('transaction', applyInitialMarks);

        persistence.destroy();
        awareness.destroy();
        doc.destroy();
      };
    });
  });

  $effect(() => {
    if (focused) {
      app.state.ancestors = entity.ancestors.map((ancestor) => ancestor.id);
      app.state.current = entity.id;
    }
  });
</script>

<svelte:window
  onkeydown={(e) => {
    if (!focused) return;
    if (editorContext?.timeline || app.state.notesOpen) return;

    const modKey = isMacOS() || isiOS() ? e.metaKey : e.ctrlKey;

    if (modKey && e.key === 'z' && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      undoManager.undo();
    } else if ((modKey && e.key === 'y') || (modKey && e.key === 'z' && e.shiftKey)) {
      e.preventDefault();
      e.stopPropagation();
      undoManager.redo();
    }
  }}
/>

{#if focused}
  <Helmet title={`${effectiveTitle.current || '(제목 없음)'} 작성 중`} />
{/if}

{#if entity.node.__typename === 'Post'}
  <div class={flex({ height: 'full', flex: '1', overflowX: 'auto' })}>
    <div class={flex({ flexDirection: 'column', flexGrow: '1', overflowX: 'auto' })}>
      <div
        class={flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          gap: '6px',
          flexShrink: '0',
          paddingLeft: '24px',
          paddingRight: '8px',
          height: '36px',
          backgroundColor: 'surface.default',
          borderRadius: '4px',
          userSelect: 'none',
        })}
        role="region"
        use:dragView={dragViewProps}
      >
        <div class={flex({ alignItems: 'center', gap: '4px', overflowX: 'hidden' })}>
          <Icon style={css.raw({ color: 'text.disabled' })} icon={FolderIcon} size={12} />

          <div class={css({ flex: 'none', fontSize: '12px', color: 'text.disabled' })}>내 포스트</div>
          <Icon style={css.raw({ color: 'text.disabled' })} icon={ChevronRightIcon} size={12} />

          {#each entity.ancestors as ancestor (ancestor.id)}
            {#if ancestor.node.__typename === 'Folder'}
              <div class={css({ flex: 'none', fontSize: '12px', color: 'text.disabled' })}>
                {ancestor.node.name}
              </div>
              <Icon style={css.raw({ color: 'text.disabled' })} icon={ChevronRightIcon} size={12} />
            {/if}
          {/each}

          <button
            class={css({
              fontSize: '12px',
              fontWeight: 'medium',
              color: 'text.subtle',
              lineClamp: 1,
              _hover: { color: 'text.default' },
              transition: 'common',
            })}
            onclick={() => {
              titleEl?.focus();
            }}
            type="button"
          >
            {effectiveTitle.current || '(제목 없음)'}
          </button>
        </div>

        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          {#if !entity.user.subscription}
            <button
              class={flex({
                alignItems: 'center',
                gap: '4px',
                paddingX: '8px',
                paddingY: '4px',
                borderRadius: '4px',
                borderWidth: '1px',
                borderColor: 'border.brand',
                fontSize: '11px',
                fontWeight: 'semibold',
                color: 'text.brand',
                backgroundColor: 'transparent',
                cursor: 'pointer',
                transition: 'common',
                _hover: { backgroundColor: 'accent.brand.subtle' },
              })}
              onclick={() => {
                planUpgradeModalOpen = true;
                mixpanel.track('open_plan_upgrade_modal', { via: 'editor_header' });
              }}
              type="button"
            >
              <Icon icon={CrownIcon} size={12} />
              <span>업그레이드</span>
            </button>
          {/if}

          <div class={center({ size: '24px' })}>
            <div
              style:background-color={match(connectionStatus)
                .with('connecting', () => '#eab308')
                .with('connected', () => '#22c55e')
                .with('disconnected', () => '#ef4444')
                .exhaustive()}
              class={css({ size: '8px', borderRadius: 'full' })}
              use:tooltip={{
                message: match(connectionStatus)
                  .with('connecting', () => '서버 연결 중...')
                  .with('connected', () => '실시간 저장 중')
                  .with('disconnected', () => '서버 연결 끊김')
                  .exhaustive(),
                placement: 'left',
                offset: 12,
                delay: 0,
              }}
            ></div>
          </div>

          {#if $query.me.id === entity.user.id}
            <Menu>
              {#snippet button({ open })}
                <button
                  class={center({
                    borderRadius: '4px',
                    size: '24px',
                    color: 'text.faint',
                    transition: 'common',
                    _hover: {
                      color: 'text.subtle',
                      backgroundColor: 'surface.muted',
                    },
                    _pressed: {
                      color: 'text.subtle',
                      backgroundColor: 'surface.muted',
                    },
                  })}
                  aria-pressed={open}
                  type="button"
                >
                  <Icon icon={ElipsisIcon} size={16} />
                </button>
              {/snippet}

              {#if entity.node.__typename === 'Post'}
                <PostMenu {entity} layoutMode={layoutMode.current} pageLayout={pageLayout.current} post={entity.node} via="editor"
                ></PostMenu>
              {/if}
            </Menu>
          {/if}

          <button
            class={center({
              borderRadius: '4px',
              size: '24px',
              color: 'text.faint',
              transition: 'common',
              _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
            })}
            onclick={() => {
              app.preference.current.zenModeEnabled = !app.preference.current.zenModeEnabled;
              if (app.preference.current.zenModeEnabled) {
                mixpanel.track('zen_mode_enabled', { via: 'editor' });
              } else {
                mixpanel.track('zen_mode_disabled', { via: 'editor' });
              }
            }}
            type="button"
            use:tooltip={{
              message: app.preference.current.zenModeEnabled ? '집중 모드 끄기' : '집중 모드 켜기',
              keys: ['Mod', 'Shift', 'M'],
            }}
          >
            <Icon icon={Maximize2Icon} size={16} />
          </button>
          {#if splitView.state.current.enabled}
            <CloseSplitView>
              <Icon icon={XIcon} size={16} />
            </CloseSplitView>
          {/if}
        </div>
      </div>

      <HorizontalDivider color="secondary" />

      <TopToolbar $site={entity.site} {editor} />

      <div class={flex({ position: 'relative', flexGrow: '1', overflowY: 'hidden' })}>
        <div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', overflowX: 'auto' })}>
          <BottomToolbar $user={entity.user} {editor} {undoManager} />
          <div
            style:position={currentViewZenModeEnabled ? 'fixed' : 'relative'}
            style:top={currentViewZenModeEnabled ? '0' : 'auto'}
            style:left={currentViewZenModeEnabled ? '0' : 'auto'}
            style:right={currentViewZenModeEnabled ? '0' : 'auto'}
            style:bottom={currentViewZenModeEnabled ? '0' : 'auto'}
            class={flex({
              position: 'relative',
              flexDirection: 'column',
              flexGrow: '1',
              overflowX: 'auto',
              zIndex: app.preference.current.zenModeEnabled && !currentViewZenModeEnabled ? 'underEditor' : 'editor',
              backgroundColor: 'surface.default',
            })}
          >
            <div
              bind:this={scrollContainer}
              id="editor-container"
              style:min-width={`${VIEW_MIN_SIZE - VIEW_BUFFER_SIZE}px`}
              class={cx(
                'editor-scroll-container',
                flex({
                  position: 'relative',
                  zIndex: 'ground',
                  flexGrow: '1',
                  backgroundColor: 'surface.default',
                  width: 'full',
                  overflow: 'auto',
                  scrollbarGutter: 'stable',
                  '&:has([data-layout="page"])': {
                    backgroundColor: 'surface.subtle/50',
                  },
                }),
              )}
              onmouseleave={() => {
                showAnchorOutline = false;
              }}
              onmousemove={(e) => {
                const rect = e.currentTarget.getBoundingClientRect();
                const mouseX = e.clientX - rect.left;
                const width = rect.width;

                showAnchorOutline = mouseX > width - 50;
              }}
              role="none"
            >
              <EditorLayout
                style={flex.raw({
                  position: 'relative',
                  flexDirection: 'column',
                  alignItems: 'center',
                  flexGrow: '1',
                })}
                class="editor"
                bodyPadding={{
                  top: 20,
                  x: effectiveLayoutMode.current === PostLayoutMode.PAGE && effectivePageLayout.current ? 0 : 40,
                }}
                layoutMode={effectiveLayoutMode}
                maxWidth={effectiveMaxWidth}
                pageLayout={effectivePageLayout}
                typewriterEnabled={app.preference.current.typewriterEnabled}
              >
                <div
                  class={flex({
                    flexDirection: 'column',
                    alignItems: 'center',
                    paddingTop: '60px',
                    size: 'full',
                  })}
                >
                  <div
                    style:width={effectiveLayoutMode.current === PostLayoutMode.PAGE
                      ? `calc(var(--prosemirror-max-width) * ${editorScale})`
                      : '100%'}
                    class={flex({
                      maxWidth: 'var(--prosemirror-max-width)',
                      flexDirection: 'column',
                      flexShrink: '0',
                      width: 'full',
                      paddingX: '40px',
                      '[data-layout="page"] &': {
                        paddingX: '0',
                        marginX: '40px',
                      },
                    })}
                  >
                    <textarea
                      bind:this={titleEl}
                      class={css({ width: 'full', fontSize: '28px', fontWeight: 'bold', resize: 'none' })}
                      autocapitalize="off"
                      autocomplete="off"
                      disabled={editorContext?.timeline}
                      maxlength="100"
                      onfocus={() => {
                        if (postId) {
                          selectionsStore.current = {
                            ...selectionsStore.current,
                            [postId]: { type: 'element', element: 'title', timestamp: dayjs().valueOf() },
                          };
                        }
                      }}
                      onkeydown={(e) => {
                        if (e.isComposing) {
                          return;
                        }

                        if (e.key === 'Enter' || (!e.altKey && e.key === 'ArrowDown')) {
                          e.preventDefault();
                          subtitleEl?.focus();
                        }
                      }}
                      placeholder="제목을 입력하세요"
                      rows={1}
                      spellcheck="false"
                      bind:value={effectiveTitle.current}
                      use:autosize
                    ></textarea>

                    <textarea
                      bind:this={subtitleEl}
                      class={css({
                        marginTop: '4px',
                        width: 'full',
                        fontSize: '16px',
                        fontWeight: 'medium',
                        overflow: 'hidden',
                        resize: 'none',
                      })}
                      autocapitalize="off"
                      autocomplete="off"
                      disabled={editorContext?.timeline}
                      maxlength="100"
                      onfocus={() => {
                        if (postId) {
                          selectionsStore.current = {
                            ...selectionsStore.current,
                            [postId]: { type: 'element', element: 'subtitle', timestamp: dayjs().valueOf() },
                          };
                        }
                      }}
                      onkeydown={(e) => {
                        if (e.isComposing) {
                          return;
                        }

                        if ((!e.altKey && e.key === 'ArrowUp') || (e.key === 'Backspace' && !subtitleEl?.value)) {
                          e.preventDefault();
                          titleEl?.focus();
                        }

                        if (e.key === 'Enter' || (!e.altKey && e.key === 'ArrowDown') || (e.key === 'Tab' && !e.shiftKey)) {
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
                      bind:value={effectiveSubtitle.current}
                      use:autosize
                    ></textarea>

                    <HorizontalDivider style={css.raw({ marginTop: '10px' })} />
                  </div>

                  <div
                    class={flex({
                      position: 'relative',
                      flexGrow: '1',
                      size: 'full',
                      flexDirection: 'column',
                      alignItems: 'center',
                    })}
                  >
                    <EditorZoom
                      style={flex.raw({
                        position: 'relative',
                        flexGrow: '1',
                        '[data-layout="page"] &': { marginX: '40px' },
                      })}
                      layoutMode={effectiveLayoutMode.current}
                      marginX={40}
                      pageLayout={effectivePageLayout.current}
                      {scrollContainer}
                      bind:scale={editorScale}
                      bind:zoomed={editorZoomed}
                    >
                      <div class={css({ display: viewDoc ? 'none' : 'flex' })}>
                        <TiptapEditor
                          style={css.raw({
                            size: 'full',
                          })}
                          awareness={viewDoc ? undefined : awareness}
                          {doc}
                          editable={!viewDoc}
                          oncreate={() => {
                            mounted = true;
                          }}
                          onfile={async ({ pos, file }) => {
                            if (!editor) {
                              return;
                            }

                            if (file.type.startsWith('image/')) {
                              editor.current.chain().focus(pos).setImage().run();
                              const nodeView = getNodeView(editor.current.view, editor.current.state.selection.anchor);

                              const url = URL.createObjectURL(file);
                              nodeView?.handle?.(new CustomEvent('inflight', { detail: { url } }));

                              try {
                                const attrs = await uploadBlobAsImage(file);
                                nodeView?.handle?.(new CustomEvent('success', { detail: { attrs } }));
                              } catch {
                                nodeView?.handle?.(new CustomEvent('error'));
                              } finally {
                                URL.revokeObjectURL(url);
                              }
                            } else {
                              editor?.current.chain().focus(pos).setFile().run();
                              const nodeView = getNodeView(editor.current.view, editor.current.state.selection.anchor);

                              nodeView?.handle?.(new CustomEvent('inflight', { detail: { file } }));

                              try {
                                const attrs = await uploadBlobAsFile(file);
                                nodeView?.handle?.(new CustomEvent('success', { detail: { attrs } }));
                              } catch {
                                nodeView?.handle?.(new CustomEvent('error'));
                              }
                            }
                          }}
                          onkeydown={(view, e) => {
                            const { doc, selection } = view.state;
                            const { anchor } = selection;

                            if (
                              (((!e.altKey && e.key === 'ArrowUp') || (e.key === 'Tab' && e.shiftKey)) && anchor === 2) ||
                              (e.key === 'Backspace' && doc.child(0).childCount === 1 && doc.child(0).child(0).childCount === 0)
                            ) {
                              e.preventDefault();
                              subtitleEl?.focus();
                            }
                          }}
                          onpaste={(event) => {
                            if (event.clipboardData?.getData('text/html')) {
                              clipboardData = {
                                html: event.clipboardData.getData('text/html'),
                                text: event.clipboardData.getData('text/plain'),
                              };

                              if (app.preference.current.pasteMode === 'ask') {
                                openPasteModal = true;
                                return true;
                              }

                              onPasteConfirm(app.preference.current.pasteMode);
                              return true;
                            }

                            return false;
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
                          {#if app.preference.current.lineHighlightEnabled}
                            <Highlight {editor} scale={editorScale} />
                          {/if}
                        {/if}
                      </div>

                      {#if viewDoc}
                        <TiptapEditor style={css.raw({ size: 'full' })} doc={viewDoc} editable={false} bind:editor={viewEditor} />
                      {/if}
                    </EditorZoom>
                  </div>
                </div>
              </EditorLayout>
            </div>
            {#if editorScale !== 1}
              <div
                class={css({
                  position: 'absolute',
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
            {#if editor && app.state.findReplaceOpenByViewId[splitViewId] && !editorContext?.timeline}
              <FloatingFindReplace close={() => (app.state.findReplaceOpenByViewId[splitViewId] = false)} {editor} />
            {/if}

            {#if editor && anchors && !editorContext?.timeline}
              <Anchors {anchors} {editor} showOutline={showAnchorOutline} />
            {/if}
          </div>
        </div>
        {#if currentViewZenModeEnabled}
          <div
            class={flex({
              position: 'fixed',
              top: '18px',
              right: '18px',
              zIndex: 'editor',
              alignItems: 'center',
              gap: '8px',
            })}
          >
            {#if !entity.user.subscription}
              <button
                class={flex({
                  alignItems: 'center',
                  gap: '4px',
                  height: '[31.5px]',
                  paddingX: '8px',
                  borderRadius: '6px',
                  borderWidth: '1px',
                  borderColor: 'border.brand',
                  fontSize: '11px',
                  fontWeight: 'semibold',
                  color: 'text.brand',
                  backgroundColor: 'surface.default',
                  cursor: 'pointer',
                  transition: 'common',
                  _hover: { backgroundColor: 'accent.brand.subtle' },
                })}
                onclick={() => {
                  planUpgradeModalOpen = true;
                  mixpanel.track('open_plan_upgrade_modal', { via: 'editor_zen_mode' });
                }}
                type="button"
              >
                <Icon icon={CrownIcon} size={12} />
                <span>업그레이드</span>
              </button>
            {/if}

            <button
              class={center({
                height: '32px',
                width: '32px',
                borderWidth: '1px',
                borderColor: 'border.strong',
                borderRadius: '8px',
                color: 'text.subtle',
                backgroundColor: { base: 'surface.default', _hover: 'surface.subtle' },
              })}
              onclick={() => {
                app.preference.current.zenModeEnabled = false;
                mixpanel.track('zen_mode_disabled', { via: 'close_button' });
              }}
              type="button"
              use:tooltip={{
                message: '집중 모드 끄기',
                keys: ['Esc'],
              }}
            >
              <Icon icon={XIcon} />
            </button>
          </div>
        {/if}

        {#if app.preference.current.noteExpanded}
          <div
            class={flex({
              flexShrink: '0',
              borderLeftWidth: '1px',
              borderColor: 'border.subtle',
              width: '1/4',
              height: 'full',
            })}
          >
            <PanelNote $entity={entity} />
          </div>
        {/if}

        <Panel $post={entity.node} $user={$query.me} {doc} {editor} {viewEditor} bind:viewDoc />
      </div>
    </div>
  </div>
{/if}

<Limit {$query} $site={entity.site} {editor} />
<PasteModal
  onconfirm={(mode) => {
    onPasteConfirm(mode);
    openPasteModal = false;
  }}
  bind:open={
    () => openPasteModal,
    (v) => {
      if (!v) {
        clipboardData = undefined;
        openPasteModal = false;
      }
    }
  }
/>
<PlanUpgradeModal bind:open={planUpgradeModalOpen}>
  FULL ACCESS로 업그레이드하면
  <br />
  모든 프리미엄 기능을 무제한으로 사용할 수 있어요.
</PlanUpgradeModal>
