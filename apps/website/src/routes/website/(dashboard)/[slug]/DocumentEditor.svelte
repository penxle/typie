<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Helmet, HorizontalDivider, Icon, Menu } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Tip } from '@typie/ui/notification';
  import { LocalStore } from '@typie/ui/state';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { match } from 'ts-pattern';
  import { DocumentSyncType } from '@/enums';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CrownIcon from '~icons/lucide/crown';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FolderIcon from '~icons/lucide/folder';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import XIcon from '~icons/lucide/x';
  import { fragment, graphql } from '$graphql';
  import { BottomToolbar, Editor as EditorComponent, TopToolbar } from '$lib/components/editor';
  import { IS_MAC } from '$lib/editor/constants';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { Editor } from '$lib/editor/editor.svelte';
  import { IndexeddbPersistence } from '$lib/editor/persistence';
  import DocumentMenu from '../@context-menu/DocumentMenu.svelte';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import DocumentPanel from './@document-panel/DocumentPanel.svelte';
  import CloseSplitView from './@split-view/CloseSplitView.svelte';
  import { getSplitViewContext, getViewContext } from './@split-view/context.svelte';
  import { getDragDropContext } from './@split-view/drag-context.svelte';
  import { dragView } from './@split-view/drag-view-action';
  import { getEditorRegistry } from './@split-view/editor-registry.svelte';
  import DocumentFindReplace from './DocumentFindReplace.svelte';
  import DocumentTemplateModal from './DocumentTemplateModal.svelte';
  import SpellcheckPopover from './SpellcheckPopover.svelte';
  import type { DocumentEditor_query } from '$graphql';
  import type { Affinity, Position } from '$lib/editor/types';

  type Props = {
    $query: DocumentEditor_query;
    slug: string;
    focused: boolean;
  };

  let { $query: _query, slug, focused }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment DocumentEditor_query on Query {
        me @required {
          id
          ...DocumentPanel_user
          ...DashboardLayout_PlanUpgradeModal_user

          sites {
            id
            ...DocumentTemplateModal_site
          }
        }

        entities(slugs: $slugs) {
          id
          slug
          url
          visibility
          availability

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

          user {
            id
            ...DocumentEditor_TopToolbar_user

            subscription {
              id
            }
          }

          node {
            __typename

            ... on Document {
              id
              title
              nullableTitle
              subtitle
              documentType: type
              characterCount
              snapshot
              version
              generation
              createdAt
              updatedAt

              assets {
                __typename

                ... on Image {
                  id
                  url
                  width
                  height
                  placeholder
                }

                ... on File {
                  id
                  url
                  name
                  size
                }

                ... on Embed {
                  id
                  url
                  title
                  description
                  thumbnailUrl
                  html
                }

                ... on DocumentArchivedNode {
                  id
                  content
                }
              }

              ...DocumentPanel_document
            }
          }
        }
      }
    `),
  );

  const entity = $derived($query.entities.find((e) => e.slug === slug));

  const syncDocument = graphql(`
    mutation Document_SyncDocument_Mutation($input: SyncDocumentInput!) {
      syncDocument(input: $input) {
        type
        data
      }
    }
  `);

  const documentSyncStream = graphql(`
    subscription Document_DocumentSyncStream_Subscription($clientId: String!, $documentId: ID!) {
      documentSyncStream(clientId: $clientId, documentId: $documentId) {
        documentId
        type
        data
      }
    }
  `);

  const updateDocument = graphql(`
    mutation Document_UpdateDocument_Mutation($input: UpdateDocumentInput!) {
      updateDocument(input: $input) {
        id
        title
        nullableTitle
        subtitle
      }
    }
  `);

  const app = getAppContext();
  const splitView = getSplitViewContext();
  const viewContext = getViewContext();
  const dragDropContext = getDragDropContext();
  const editorRegistry = getEditorRegistry();
  const dragViewProps = $derived({ dragDropContext, viewId: viewContext.id });

  const ctx = getEditorContext();
  const editor = new Editor();
  ctx.editor = editor;

  const document = $derived(entity?.node.__typename === 'Document' ? entity.node : null);
  const documentId = $derived(document?.id ?? null);
  const title = $derived(document?.title ?? '');
  const serverSnapshot = ctx.serverSnapshot;
  const serverVersion = $derived(ctx.serverVersion);
  const assets = $derived(document?.assets);

  $effect(() => {
    if (assets) {
      for (const asset of assets) {
        if (asset.__typename === 'Image') {
          editor.imageAssets.set(asset.id, {
            id: asset.id,
            url: asset.url,
            width: asset.width,
            height: asset.height,
            placeholder: asset.placeholder,
          });
        } else if (asset.__typename === 'File') {
          editor.fileAssets.set(asset.id, {
            id: asset.id,
            url: asset.url,
            name: asset.name,
            size: asset.size,
          });
        } else if (asset.__typename === 'Embed') {
          editor.embedAssets.set(asset.id, {
            id: asset.id,
            url: asset.url,
            title: asset.title ?? null,
            description: asset.description ?? null,
            thumbnailUrl: asset.thumbnailUrl ?? null,
            html: asset.html ?? null,
          });
        } else if (asset.__typename === 'DocumentArchivedNode') {
          editor.archivedAssets.set(asset.id, {
            id: asset.id,
            content: asset.content,
          });
        }
      }
    }
  });

  const DISCONNECT_THRESHOLD = 3;
  const clientId = nanoid();
  let syncUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let persistence: IndexeddbPersistence | null = null;
  let connectionStatus = $state<'connecting' | 'connected' | 'disconnected'>('connecting');
  let lastHeartbeatAt = $state(dayjs());
  let planUpgradeModalOpen = $state(false);
  let showFindReplace = $state(false);

  const selectionsStore = new LocalStore<Record<string, { selection?: unknown; type?: string; element?: string; timestamp: number }>>(
    'typie:selections',
    {},
  );

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();
  let localTitle = $state('');
  let localSubtitle = $state('');
  let titleDirty = $state(false);
  let subtitleDirty = $state(false);

  $effect(() => {
    if (document) {
      const serverTitle = document.nullableTitle ?? '';
      const serverSubtitle = document.subtitle ?? '';

      if (titleDirty && serverTitle === localTitle) {
        titleDirty = false;
      }
      if (subtitleDirty && serverSubtitle === localSubtitle) {
        subtitleDirty = false;
      }

      if (!titleDirty) {
        localTitle = serverTitle;
      }
      if (!subtitleDirty) {
        localSubtitle = serverSubtitle;
      }
    }
  });

  async function handleTitleChanged() {
    if (!documentId) return;

    titleDirty = true;
    await updateDocument({
      documentId,
      title: localTitle || null,
    });
  }

  async function handleSubtitleChanged() {
    if (!documentId) return;

    subtitleDirty = true;
    await updateDocument({
      documentId,
      subtitle: localSubtitle || null,
    });
  }

  const currentViewZenModeEnabled = $derived(
    app.preference.current.zenModeEnabled && viewContext.id === splitView.state.current.focusedViewId,
  );

  $effect(() => {
    if (currentViewZenModeEnabled) {
      Tip.show('editor.zen-mode.enabled', '집중 모드가 활성화되었어요. Esc 키를 눌러 빠져나올 수 있어요.');
    }
  });

  $effect(() => {
    if (focused && entity) {
      app.state.ancestors = entity.ancestors.map((ancestor) => ancestor.id);
      app.state.current = entity.id;
    }
  });

  $effect(() => {
    void app.preference.current.autoSurroundEnabled;
    const enabled = app.preference.current.autoSurroundEnabled;
    editor.setAutoSurroundEnabled(enabled);
  });

  $effect(() => {
    const _slug = slug;
    editorRegistry.registerNative(viewContext.id, slug, editor);

    return () => {
      editorRegistry.unregister(viewContext.id, _slug);
    };
  });

  async function handleSyncPayload(payload: { type: DocumentSyncType; data: string }) {
    switch (payload.type) {
      case DocumentSyncType.HEARTBEAT: {
        lastHeartbeatAt = dayjs(payload.data);
        connectionStatus = 'connected';
        break;
      }
      case DocumentSyncType.UPDATE: {
        editor.importUpdates(Uint8Array.fromBase64(payload.data));
        break;
      }
      case DocumentSyncType.VECTOR: {
        if (persistence) await persistence.saveCheckpoint(Uint8Array.fromBase64(payload.data));
        break;
      }
      case DocumentSyncType.RESET: {
        if (persistence) await persistence.clear();
        ctx.serverSnapshot = Uint8Array.fromBase64(payload.data);
        ctx.resetKey++;
        break;
      }
    }
  }

  async function doSync(
    input: { clientId: string; documentId: string; type: DocumentSyncType; data: string },
    options?: { transport?: 'fetch' | 'sse' | 'ws' },
  ) {
    const results = await syncDocument(input, options);
    for (const payload of results) {
      await handleSyncPayload(payload);
    }
  }

  $effect(() => {
    const currentDocumentId = documentId;
    if (!currentDocumentId) return;

    persistence = new IndexeddbPersistence(currentDocumentId);

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

    const heartbeatInterval = setInterval(() => {
      if (dayjs().diff(lastHeartbeatAt, 'seconds') > DISCONNECT_THRESHOLD) {
        connectionStatus = 'disconnected';
      }
    }, 1000);

    let fullSyncInterval: ReturnType<typeof setInterval> | null = null;
    let forceSyncInterval: ReturnType<typeof setInterval> | null = null;
    let unsubscribe: (() => void) | null = null;

    editor.ready.then(async () => {
      if (currentDocumentId !== documentId) return;

      const local = await persistence?.load();
      if (local && persistence && persistence.generation === ctx.serverGeneration) {
        editor.importUpdatesBatch([local.snapshot, ...local.updates]);
      } else if (persistence) {
        await persistence.clear();
        const snapshot = editor.export({ type: 'snapshot' });
        const version = editor.export({ type: 'version' });
        if (snapshot && version && serverVersion) {
          await persistence.saveSnapshot(snapshot, version, ctx.serverGeneration);
          await persistence.saveCheckpoint(Uint8Array.fromBase64(serverVersion));
        }
      }

      fullSyncInterval = setInterval(() => fullSync(), 60_000);
      forceSyncInterval = setInterval(() => forceSync(), 10_000);

      await fullSync();

      unsubscribe = documentSyncStream.subscribe({ clientId, documentId: currentDocumentId }, async (payload) => {
        if (currentDocumentId !== documentId) {
          return;
        }

        await handleSyncPayload(payload);
      });
    });

    return () => {
      unsubscribe?.();
      if (fullSyncInterval) clearInterval(fullSyncInterval);
      if (forceSyncInterval) clearInterval(forceSyncInterval);
      clearInterval(heartbeatInterval);
      if (syncUpdateTimeout) {
        clearTimeout(syncUpdateTimeout);
        syncUpdateTimeout = null;
      }
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
      if (currentDocumentId && persistence && persistence.checkpoint.length > 0) {
        const updates = editor.export({ type: 'updates-from', version: persistence.checkpoint });
        if (updates?.length) {
          doSync({
            clientId,
            documentId: currentDocumentId,
            type: DocumentSyncType.UPDATE,
            data: updates.toBase64(),
          });
        }
      }
      persistence?.destroy();
      persistence = null;
    };
  });

  async function fullSync() {
    if (!documentId) return;

    const snapshot = editor.export({ type: 'snapshot' });
    const version = editor.export({ type: 'version' });
    if (persistence && snapshot && version) {
      await persistence.saveSnapshot(snapshot, version);
    }

    const update = editor.export({ type: 'all-updates' });
    if (update?.length) {
      await doSync({
        clientId,
        documentId,
        type: DocumentSyncType.UPDATE,
        data: update.toBase64(),
      });
    }
  }

  async function forceSync() {
    if (!documentId) return;

    const version = editor.export({ type: 'version' });
    if (!version) return;

    await doSync(
      {
        clientId,
        documentId,
        type: DocumentSyncType.VECTOR,
        data: version.toBase64(),
      },
      { transport: 'ws' },
    );
  }

  function handleDocChanged() {
    if (!documentId) return;

    if (persistence && persistence.version.length > 0) {
      const update = editor.export({ type: 'updates-from', version: persistence.version });
      if (update?.length) {
        persistence.saveUpdate(update);
      }
    }

    if (syncUpdateTimeout) {
      clearTimeout(syncUpdateTimeout);
    }

    syncUpdateTimeout = setTimeout(async () => {
      if (!documentId) return;

      const update =
        persistence && persistence.checkpoint.length > 0
          ? editor.export({ type: 'updates-from', version: persistence.checkpoint })
          : undefined;

      if (update?.length) {
        await doSync(
          {
            clientId,
            documentId,
            type: DocumentSyncType.UPDATE,
            data: update.toBase64(),
          },
          { transport: 'ws' },
        );
      }
    }, 1000);
  }

  let editorReady = false;

  function handleSelectionChanged(anchor: Position, head: Position) {
    if (!documentId || !editorReady || !editor.isFocused) return;
    selectionsStore.current = {
      ...selectionsStore.current,
      [documentId]: { selection: { anchor, head }, timestamp: dayjs().valueOf() },
    };
  }

  function handleEditorReady() {
    if (!documentId) return;
    const saved = selectionsStore.current[documentId];
    if (!saved) {
      titleEl?.focus();
      return;
    }
    if (saved.type === 'element') {
      if (saved.element === 'title') titleEl?.focus();
      else if (saved.element === 'subtitle') subtitleEl?.focus();
    } else if (saved.selection) {
      const sel = saved.selection as {
        anchor: { nodeId: string; offset: number; affinity: Affinity };
        head: { nodeId: string; offset: number; affinity: Affinity };
      };
      editor.dispatch({
        type: 'setSelection',
        anchorNodeId: sel.anchor.nodeId,
        anchorOffset: sel.anchor.offset,
        anchorAffinity: sel.anchor.affinity,
        headNodeId: sel.head.nodeId,
        headOffset: sel.head.offset,
        headAffinity: sel.head.affinity,
      });
      editor.focus();
    }
    editorReady = true;
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    if ((IS_MAC ? e.metaKey : e.ctrlKey) && e.code === 'KeyF' && focused) {
      e.preventDefault();
      showFindReplace = true;
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

{#if document && entity}
  {#if focused}
    <Helmet title={`${title || '(제목 없음)'} 작성 중`} />
  {/if}

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

          <div class={css({ flex: 'none', fontSize: '12px', color: 'text.disabled' })}>내 문서</div>
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
              titleEl?.select();
            }}
            type="button"
          >
            {title || '(제목 없음)'}
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
                whiteSpace: 'nowrap',
                color: 'text.brand',
                backgroundColor: 'transparent',
                cursor: 'pointer',
                transition: 'common',
                _hover: { backgroundColor: 'accent.brand.subtle' },
              })}
              onclick={() => {
                planUpgradeModalOpen = true;
                mixpanel.track('open_plan_upgrade_modal', { via: 'document_header' });
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
                  <Icon icon={EllipsisIcon} size={16} />
                </button>
              {/snippet}

              <DocumentMenu {document} {entity} via="editor" />
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
                mixpanel.track('zen_mode_enabled', { via: 'document' });
              } else {
                mixpanel.track('zen_mode_disabled', { via: 'document' });
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
          <CloseSplitView>
            <Icon icon={XIcon} size={16} />
          </CloseSplitView>
        </div>
      </div>

      <HorizontalDivider color="secondary" />

      <TopToolbar $user={entity.user} />

      <div class={flex({ position: 'relative', flexGrow: '1', overflowY: 'hidden' })}>
        <div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', overflowX: 'auto' })}>
          <BottomToolbar onSearchClick={() => (showFindReplace = !showFindReplace)} />

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
            <EditorComponent
              {editor}
              onDocChanged={handleDocChanged}
              onEditorReady={handleEditorReady}
              onExitedDocumentStart={() => subtitleEl?.focus()}
              onSelectionChanged={handleSelectionChanged}
              snapshot={serverSnapshot}
              unit="cm"
            >
              {#snippet header()}
                <div
                  class={flex({
                    flexDirection: 'column',
                    alignItems: 'center',
                    paddingTop: '60px',
                    width: 'full',
                    ...(editor.layout.layoutMode.type === 'paginated' && { paddingBottom: '20px' }),
                  })}
                >
                  <div
                    class={flex({
                      flexDirection: 'column',
                      flexShrink: '0',
                      width: 'full',
                    })}
                  >
                    <textarea
                      bind:this={titleEl}
                      class={css({ width: 'full', fontSize: '28px', fontWeight: 'bold', resize: 'none' })}
                      autocapitalize="off"
                      autocomplete="off"
                      maxlength={100}
                      onfocus={() => {
                        if (documentId) {
                          selectionsStore.current = {
                            ...selectionsStore.current,
                            [documentId]: { type: 'element', element: 'title', timestamp: dayjs().valueOf() },
                          };
                        }
                      }}
                      oninput={handleTitleChanged}
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
                      bind:value={localTitle}
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
                      maxlength={100}
                      onfocus={() => {
                        if (documentId) {
                          selectionsStore.current = {
                            ...selectionsStore.current,
                            [documentId]: { type: 'element', element: 'subtitle', timestamp: dayjs().valueOf() },
                          };
                        }
                      }}
                      oninput={handleSubtitleChanged}
                      onkeydown={(e) => {
                        if (e.isComposing) {
                          return;
                        }

                        if ((!e.altKey && e.key === 'ArrowUp') || (e.key === 'Backspace' && !localSubtitle)) {
                          e.preventDefault();
                          titleEl?.focus();
                        }

                        if (e.key === 'Enter' || (!e.altKey && e.key === 'ArrowDown') || (e.key === 'Tab' && !e.shiftKey)) {
                          e.preventDefault();
                          editor.focus().dispatch({ type: 'navigate', direction: 'documentStart', extend: false });
                        }
                      }}
                      placeholder="부제목을 입력하세요"
                      rows={1}
                      spellcheck="false"
                      bind:value={localSubtitle}
                      use:autosize
                    ></textarea>

                    <HorizontalDivider style={css.raw({ marginTop: '10px' })} />
                  </div>
                </div>
              {/snippet}
              <SpellcheckPopover {editor} />
            </EditorComponent>
            {#if showFindReplace}
              <DocumentFindReplace close={() => (showFindReplace = false)} {editor} />
            {/if}
          </div>
        </div>

        <DocumentPanel $document={document} $user={$query.me} {editor} />
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
                mixpanel.track('open_plan_upgrade_modal', { via: 'document_zen_mode' });
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
    </div>
  </div>

  <PlanUpgradeModal $user={$query.me} bind:open={planUpgradeModalOpen}>
    FULL ACCESS로 업그레이드하면
    <br />
    모든 프리미엄 기능을 무제한으로 사용할 수 있어요.
  </PlanUpgradeModal>

  {#if $query.me.sites[0]}
    <DocumentTemplateModal $site={$query.me.sites[0]} {editor} {focused} />
  {/if}
{/if}
