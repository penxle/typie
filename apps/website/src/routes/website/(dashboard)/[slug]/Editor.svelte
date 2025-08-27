<script lang="ts">
  import { random } from '@ctrl/tinycolor';
  import stringHash from '@sindresorhus/string-hash';
  import { isiOS, isMacOS } from '@tiptap/core';
  import { Selection } from '@tiptap/pm/state';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { cleanOrphanAnchors } from '@typie/ui/anchor';
  import { EditorLayout, Helmet, HorizontalDivider, Icon, InEditorBody, Menu, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Tip } from '@typie/ui/notification';
  import { getNodeView, setupEditorContext, TiptapEditor } from '@typie/ui/tiptap';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { base64 } from 'rfc4648';
  import { onMount, untrack } from 'svelte';
  import { on } from 'svelte/events';
  import { match } from 'ts-pattern';
  import { IndexeddbPersistence } from 'y-indexeddb';
  import { defaultDeleteFilter, defaultProtectedNodes, ySyncPluginKey } from 'y-prosemirror';
  import * as YAwareness from 'y-protocols/awareness';
  import * as Y from 'yjs';
  import { PostLayoutMode, PostSyncType, UserRole } from '@/enums';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import IconClockFading from '~icons/lucide/clock-fading';
  import ElipsisIcon from '~icons/lucide/ellipsis';
  import FolderIcon from '~icons/lucide/folder';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import PanelRightCloseIcon from '~icons/lucide/panel-right-close';
  import PanelRightOpenIcon from '~icons/lucide/panel-right-open';
  import XIcon from '~icons/lucide/x';
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { unfurlEmbed, uploadBlobAsFile, uploadBlobAsImage } from '$lib/utils';
  import PostMenu from '../@context-menu/PostMenu.svelte';
  import Anchors from './@anchor/Anchors.svelte';
  import FloatingFindReplace from './FloatingFindReplace.svelte';
  import Highlight from './Highlight.svelte';
  import Limit from './Limit.svelte';
  import Panel from './Panel.svelte';
  import PanelNote from './PanelNote.svelte';
  import PasteModal from './PasteModal.svelte';
  import Placeholder from './Placeholder.svelte';
  import { YState } from './state.svelte';
  import Timeline from './Timeline.svelte';
  import Toolbar from './Toolbar.svelte';
  import type { Editor } from '@tiptap/core';
  import type { PageLayout, Ref } from '@typie/ui/utils';
  import type { Editor_query } from '$graphql';

  const DISCONNECT_THRESHOLD = 3;

  type Props = {
    $query: Editor_query;
  };

  let { $query: _query }: Props = $props();

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

        entity(slug: $slug) {
          id
          slug
          url
          visibility
          availability

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

            fonts {
              id
              name
              weight
              url
            }

            ...Editor_Limit_site
            ...Editor_Placeholder_site
            ...Editor_Toolbar_site
          }

          user {
            id
          }

          node {
            __typename

            ... on Post {
              id
              title
              type
              update

              ...Editor_Panel_post
              ...Editor_Timeline_post
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

  setupEditorContext();

  const app = getAppContext();
  const clientId = nanoid();
  const postId = $derived($query && $query.entity.node.__typename === 'Post' ? $query.entity.node.id : null);

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();

  let editor = $state<Ref<Editor>>();

  let connectionStatus = $state<'connecting' | 'connected' | 'disconnected'>('connecting');
  let lastHeartbeatAt = $state(dayjs());

  let mounted = $state(false);

  let showTimeline = $state(false);
  let showAnchorOutline = $state(false);

  let clipboardData = $state<{ html: string; text?: string }>();

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
  const anchors = new YState<Record<string, string | null>>(doc, 'anchors', {});
  const pageLayout = new YState<PageLayout | undefined>(doc, 'pageLayout', undefined);
  const layoutMode = new YState<PostLayoutMode>(doc, 'layoutMode', PostLayoutMode.SCROLL);

  const effectiveTitle = $derived(title.current || '(제목 없음)');

  const persistSelection = () => {
    if (!editor?.current || !postId) return;

    const { selection } = editor.current.state;

    const selections = JSON.parse(localStorage.getItem('typie:selections') || '{}');
    selections[postId] = { ...selection.toJSON(), timestamp: dayjs().valueOf() };
    localStorage.setItem('typie:selections', JSON.stringify(selections));
  };

  const fontFaces = $derived(
    $query.entity.site.fonts
      .map(
        (font) =>
          `@font-face { font-family: ${font.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
      )
      .join('\n'),
  );

  let syncUpdateTimeout: NodeJS.Timeout | null = null;
  let pendingUpdate: Uint8Array | null = null;
  let lastSyncTime = Date.now();

  doc.on('updateV2', async (update, origin) => {
    if (browser && origin !== 'remote' && postId) {
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
        await syncPost(
          {
            clientId,
            postId,
            type: PostSyncType.UPDATE,
            data: base64.stringify(pendingUpdate),
          },
          { transport: 'ws' },
        );

        pendingUpdate = null;
        lastSyncTime = Date.now();
      } else {
        const remainingTime = Math.max(0, 100 - timeSinceLastSync);

        syncUpdateTimeout = setTimeout(async () => {
          if (pendingUpdate && postId) {
            await syncPost(
              {
                clientId,
                postId,
                type: PostSyncType.UPDATE,
                data: base64.stringify(pendingUpdate),
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

  let syncAwarenessTimeout: NodeJS.Timeout | null = null;
  let pendingAwarenessStates: { added: number[]; updated: number[]; removed: number[] } | null = null;
  let lastAwarenessSyncTime = Date.now();

  awareness.on('update', async (states: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
    if (browser && origin !== 'remote' && postId) {
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
            data: base64.stringify(update),
          },
          { transport: 'ws' },
        );

        pendingAwarenessStates = null;
        lastAwarenessSyncTime = Date.now();
      } else {
        const remainingTime = Math.max(0, 100 - timeSinceLastSync);

        syncAwarenessTimeout = setTimeout(async () => {
          if (pendingAwarenessStates && postId) {
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
                data: base64.stringify(update),
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

  const forceSync = async () => {
    if (!postId) return;

    const vector = Y.encodeStateVector(doc);

    await syncPost(
      {
        clientId,
        postId,
        type: PostSyncType.VECTOR,
        data: base64.stringify(vector),
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
      data: base64.stringify(update),
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
    if (app.preference.current.zenModeEnabled) {
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

  onMount(() => {
    if (!postId) return;

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

    const unsubscribe = postSyncStream.subscribe({ clientId, postId }, async (payload) => {
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
            postId,
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
            postId,
            type: PostSyncType.AWARENESS,
            data: base64.stringify(update),
          },
          { transport: 'ws' },
        );
      }
    });

    const persistence = new IndexeddbPersistence(`typie:editor:${postId}`, doc);

    if ($query.entity.node.__typename === 'Post') {
      Y.applyUpdateV2(doc, base64.parse($query.entity.node.update), 'remote');

      if (![PostLayoutMode.SCROLL, PostLayoutMode.PAGE].includes(layoutMode.current)) {
        layoutMode.current = PostLayoutMode.SCROLL;
      }
    }

    awareness.setLocalStateField('user', {
      name: $query.me.name,
      color: random({ luminosity: 'bright', seed: stringHash($query.me.id) }).toHexString(),
    });

    if (editor?.current) {
      editor.current.storage.anchors = anchors;
      cleanOrphanAnchors(editor.current, doc);
    }

    editor?.current.once('create', ({ editor }) => {
      const selections = JSON.parse(localStorage.getItem('typie:selections') || '{}');
      if (postId && selections[postId]) {
        if (selections[postId].type === 'element') {
          if (selections[postId].element === 'title') {
            titleEl?.focus();
          } else if (selections[postId].element === 'subtitle') {
            subtitleEl?.focus();
          }
        } else {
          try {
            const selection = Selection.fromJSON(editor.state.doc, selections[postId]);
            editor.commands.command(({ tr, dispatch }) => {
              tr.setSelection(selection);
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
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        e.stopPropagation();

        forceSync();
        Tip.show('editor.shortcut.save', '따로 저장 키를 누르지 않아도 모든 변경 사항은 실시간으로 저장돼요.');
      }

      if (e.altKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
        e.preventDefault();
        e.stopPropagation();

        const currentEntityId = $query.entity.id;

        let siblingEntities: { id: string; slug: string; node: { __typename: string } }[] = [];

        if ($query.entity.parent) {
          siblingEntities = $query.entity.parent.children.filter((child) => child.node.__typename === 'Post');
        } else {
          siblingEntities = $query.entity.site.entities.filter((entity) => entity.node.__typename === 'Post');
        }

        const currentIndex = siblingEntities.findIndex((entity) => entity.id === currentEntityId);
        if (currentIndex === -1) return;

        let targetIndex;
        if (e.key === 'ArrowUp') {
          targetIndex = currentIndex - 1;
          if (targetIndex < 0) targetIndex = siblingEntities.length - 1;
        } else {
          targetIndex = currentIndex + 1;
          if (targetIndex >= siblingEntities.length) targetIndex = 0;
        }

        const targetEntity = siblingEntities[targetIndex];
        if (targetEntity && targetEntity.slug) {
          await goto(`/${targetEntity.slug}`);
        }
      }
    });

    app.state.ancestors = $query.entity.ancestors.map((ancestor) => ancestor.id);
    app.state.current = $query.entity.id;

    editor?.current.on('selectionUpdate', persistSelection);

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

<svelte:window
  onkeydown={(e) => {
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

<Helmet title={`${effectiveTitle} 작성 중`} />

{#if $query.entity.node.__typename === 'Post'}
  <div class={flex({ height: 'full' })}>
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
        })}
      >
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <Icon style={css.raw({ color: 'text.disabled' })} icon={FolderIcon} size={12} />

          <div class={css({ flex: 'none', fontSize: '12px', color: 'text.disabled' })}>내 포스트</div>
          <Icon style={css.raw({ color: 'text.disabled' })} icon={ChevronRightIcon} size={12} />

          {#each $query.entity.ancestors as ancestor (ancestor.id)}
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
            {effectiveTitle}
          </button>
        </div>

        <div class={flex({ alignItems: 'center', gap: '4px' })}>
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

          {#if $query.me.id === $query.entity.user.id}
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

              {#if $query.entity.node.__typename === 'Post'}
                <PostMenu
                  entity={$query.entity}
                  layoutMode={layoutMode.current}
                  pageLayout={pageLayout.current}
                  post={$query.entity.node}
                  via="editor"
                />
              {/if}

              <HorizontalDivider color="secondary" />

              {#if $query.me.role === UserRole.ADMIN}
                <MenuItem
                  icon={IconClockFading}
                  onclick={() => {
                    showTimeline = !showTimeline;
                  }}
                >
                  {#if showTimeline}
                    타임라인 닫기
                  {:else}
                    타임라인
                  {/if}
                </MenuItem>
              {/if}
            </Menu>
          {/if}

          <button
            class={center({
              borderRadius: '4px',
              size: '24px',
              color: 'text.faint',
              transition: 'common',
              _hover: { backgroundColor: 'surface.muted' },
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
            <Icon style={css.raw({ color: 'text.faint' })} icon={Maximize2Icon} size={16} />
          </button>

          <button
            class={center({
              borderRadius: '4px',
              size: '24px',
              color: 'text.faint',
              transition: 'common',
              _hover: { backgroundColor: 'surface.muted' },
            })}
            onclick={() => {
              app.preference.current.panelExpanded = !app.preference.current.panelExpanded;
              mixpanel.track('toggle_panel_expanded', { expanded: app.preference.current.panelExpanded });
            }}
            type="button"
            use:tooltip={{
              message: app.preference.current.panelExpanded ? '패널 닫기' : '패널 열기',
              keys: ['Mod', 'Shift', 'P'],
            }}
          >
            <Icon
              style={css.raw({ color: 'text.faint' })}
              icon={app.preference.current.panelExpanded ? PanelRightCloseIcon : PanelRightOpenIcon}
              size={16}
            />
          </button>
        </div>
      </div>

      <HorizontalDivider color="secondary" />

      <Toolbar $site={$query.entity.site} {doc} {editor} {undoManager} />

      <div class={flex({ position: 'relative', flexGrow: '1', overflowY: 'hidden' })}>
        <div
          id="editor-container"
          style:position={app.preference.current.zenModeEnabled ? 'fixed' : 'relative'}
          style:top={app.preference.current.zenModeEnabled ? '0' : 'auto'}
          style:left={app.preference.current.zenModeEnabled ? '0' : 'auto'}
          style:right={app.preference.current.zenModeEnabled ? '0' : 'auto'}
          style:bottom={app.preference.current.zenModeEnabled ? '0' : 'auto'}
          class={flex({
            position: 'relative',
            flexGrow: '1',
            zIndex: 'editor',
            backgroundColor: 'surface.default',
          })}
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
            class={cx(
              'editor',
              css({
                position: 'relative',
                flexGrow: '1',
                height: 'full',
                overflowY: 'auto',
                scrollbarGutter: 'stable',
              }),
            )}
            layoutMode={layoutMode.current}
            maxWidth={maxWidth.current}
            pageLayout={pageLayout.current}
            typewriterEnabled={app.preference.current.typewriterEnabled}
            typewriterPosition={app.preference.current.typewriterPosition}
          >
            <div
              class={flex({
                flexDirection: 'column',
                alignItems: 'center',
                paddingTop: '60px',
                size: 'full',
              })}
            >
              <div class={center({ width: 'full', paddingX: '80px' })}>
                <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: 'var(--prosemirror-max-width)' })}>
                  <textarea
                    bind:this={titleEl}
                    class={css({ width: 'full', fontSize: '28px', fontWeight: 'bold', resize: 'none' })}
                    autocapitalize="off"
                    autocomplete="off"
                    maxlength="100"
                    onfocus={() => {
                      if (postId) {
                        const selections = JSON.parse(localStorage.getItem('typie:selections') || '{}');
                        selections[postId] = { type: 'element', element: 'title', timestamp: dayjs().valueOf() };
                        localStorage.setItem('typie:selections', JSON.stringify(selections));
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
                      overflow: 'hidden',
                      resize: 'none',
                    })}
                    autocapitalize="off"
                    autocomplete="off"
                    maxlength="100"
                    onfocus={() => {
                      if (postId) {
                        const selections = JSON.parse(localStorage.getItem('typie:selections') || '{}');
                        selections[postId] = { type: 'element', element: 'subtitle', timestamp: dayjs().valueOf() };
                        localStorage.setItem('typie:selections', JSON.stringify(selections));
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
                    bind:value={subtitle.current}
                    use:autosize
                  ></textarea>

                  <HorizontalDivider style={css.raw({ marginTop: '10px' })} />
                </div>
              </div>

              <div class={css({ position: 'relative', flexGrow: '1', width: 'full' })}>
                <TiptapEditor
                  style={css.raw({ size: 'full', paddingX: '80px', paddingTop: '20px' })}
                  {awareness}
                  {doc}
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
                  <InEditorBody {editor} pageLayout={pageLayout.current ?? null}>
                    <Placeholder $site={$query.entity.site} {doc} {editor} />
                  </InEditorBody>
                  {#if app.preference.current.lineHighlightEnabled}
                    <Highlight {editor} />
                  {/if}
                {/if}
              </div>
            </div>

            {#if showTimeline}
              <div class={css({ position: 'absolute', inset: '0', backgroundColor: 'surface.default' })}>
                <Timeline $post={$query.entity.node} {doc} />
              </div>
            {/if}
          </EditorLayout>

          <Anchors
            anchors={anchors.current}
            {editor}
            showOutline={showAnchorOutline}
            updateAnchorName={(nodeId, name) => {
              const newAnchors = { ...anchors.current };
              newAnchors[nodeId] = name;
              anchors.current = newAnchors;
            }}
          />

          {#if editor && app.state.findReplaceOpen}
            <FloatingFindReplace close={() => (app.state.findReplaceOpen = false)} {editor} />
          {/if}
        </div>

        {#if app.preference.current.zenModeEnabled}
          <button
            class={css({
              position: 'fixed',
              top: '18px',
              right: '18px',
              zIndex: 'editor',
              borderWidth: '1px',
              borderColor: 'border.strong',
              borderRadius: '8px',
              padding: '5px',
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
        {/if}

        {#if app.preference.current.noteExpanded}
          <div
            class={flex({
              flexShrink: '0',
              borderLeftWidth: '1px',
              borderColor: 'border.subtle',
              paddingTop: '16px',
              width: '1/4',
              height: 'full',
              overflowY: 'auto',
              scrollbarGutter: 'stable',
            })}
          >
            <PanelNote {doc} />
          </div>
        {/if}
      </div>
    </div>

    <Panel $post={$query.entity.node} $user={$query.me} {doc} {editor} />
  </div>
{/if}

<Limit {$query} $site={$query.entity.site} {editor} />
<PasteModal
  onconfirm={(mode) => {
    if (!editor || !clipboardData) {
      return;
    }

    if (mode === 'html') {
      editor.current.view.pasteHTML(clipboardData.html);
    } else if (mode === 'text') {
      if (clipboardData.text) {
        editor?.current.view.pasteText(clipboardData.text);
      } else {
        const dom = new DOMParser().parseFromString(clipboardData.html, 'text/html');
        editor.current.view.pasteText(dom.body.textContent);
      }
    }

    clipboardData = undefined;
  }}
  bind:open={
    () => !!clipboardData,
    (v) => {
      if (!v) {
        clipboardData = undefined;
      }
    }
  }
/>
