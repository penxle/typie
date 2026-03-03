<script lang="ts">
  import { createFragment, createMutation, createSubscription } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Helmet, HorizontalDivider, Icon, Menu } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Tip, Toast } from '@typie/ui/notification';
  import { LocalStore } from '@typie/ui/state';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import { setContext } from 'svelte';
  import { fly } from 'svelte/transition';
  import { DocumentSyncType } from '@/enums';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CrownIcon from '~icons/lucide/crown';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FolderIcon from '~icons/lucide/folder';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import WifiOffIcon from '~icons/lucide/wifi-off';
  import XIcon from '~icons/lucide/x';
  import { dev } from '$app/environment';
  import { BottomToolbar, Editor as EditorComponent, TopToolbar } from '$lib/components/editor';
  import { IS_MAC } from '$lib/editor/constants';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { Editor } from '$lib/editor/editor.svelte';
  import { IndexeddbPersistence } from '$lib/editor/persistence';
  import { initWasm } from '$lib/wasm.svelte';
  import { graphql } from '$mearie';
  import DocumentMenu from '../@context-menu/DocumentMenu.svelte';
  import FontUploadModal from '../FontUploadModal.svelte';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import DocumentPanel from './@document-panel/DocumentPanel.svelte';
  import CloseButton from './@pane/CloseButton.svelte';
  import { getPane, getPaneGroup } from './@pane/context.svelte';
  import { dragPane } from './@pane/dnd';
  import { getEditorRegistry } from './@pane/editor-registry.svelte';
  import DocumentFindReplace from './DocumentFindReplace.svelte';
  import DocumentTemplateModal from './DocumentTemplateModal.svelte';
  import EditorV2NoticeModal from './EditorV2NoticeModal.svelte';
  import FeedbackPopover from './FeedbackPopover.svelte';
  import SpellcheckPopover from './SpellcheckPopover.svelte';
  import type { Affinity, Position } from '$lib/editor/types';
  import type { DocumentEditor_query$key } from '$mearie';

  type Props = {
    query$key: DocumentEditor_query$key;
    slug: string;
    focused: boolean;
    onReady?: () => void;
  };

  let { query$key, slug, focused, onReady }: Props = $props();

  // DocumentEditor는 slug마다 새로 생성/삭제(mounted 메커니즘)되므로 생성 시점의 값을 캡처.
  // slug를 $derived 안에서 reactive하게 읽으면 언마운트 도중 View.svelte의 prop getter 체인이
  // entity = undefined인 상태에서 호출되어 크래시가 발생한다.
  const mountedSlug = slug;

  const query = createFragment(
    graphql(`
      fragment DocumentEditor_query on Query {
        me @required {
          id
          role
          ...EditorContext_user
          ...DocumentPanel_user
          ...DashboardLayout_PlanUpgradeModal_user

          sites {
            id
            ...DocumentTemplateModal_site
          }
        }

        impersonation {
          admin {
            role
          }
        }

        entity(slug: $slug) {
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
              locked
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

              fontFamilies {
                id
                familyName
                displayName
                state

                fonts {
                  id
                  weight
                  subfamilyDisplayName
                  url
                  state
                }
              }

              ...DocumentPanel_document
            }
          }
        }
      }
    `),
    () => query$key,
  );

  const entity = $derived(query.data.entity);

  const [syncDocument] = createMutation(
    graphql(`
      mutation Document_SyncDocument_Mutation($input: SyncDocumentInput!) {
        syncDocument(input: $input) {
          type
          data
        }
      }
    `),
  );

  let documentSyncReady = $state(false);

  createSubscription(
    graphql(`
      subscription Document_DocumentSyncStream_Subscription($clientId: String!, $documentId: ID!) {
        documentSyncStream(clientId: $clientId, documentId: $documentId) {
          documentId
          type
          data
        }
      }
    `),
    () => ({ clientId, documentId: documentId ?? '' }),
    () => ({
      skip: !documentSyncReady || !documentId,
      onData: async (data) => {
        const currentDocumentId = documentId;
        const currentPersistence = persistence;
        if (!currentDocumentId || !currentPersistence) return;

        await handleSyncPayload(data.documentSyncStream, {
          documentId: currentDocumentId,
          persistence: currentPersistence,
        });
      },
    }),
  );

  const [updateDocument] = createMutation(
    graphql(`
      mutation Document_UpdateDocument_Mutation($input: UpdateDocumentInput!) {
        updateDocument(input: $input) {
          id
          title
          nullableTitle
          subtitle
          locked
        }
      }
    `),
  );

  graphql(`
    fragment EditorContext_user on User {
      id
      ...RemarkPopover_user
    }
  `);

  const app = getAppContext();
  const paneGroup = getPaneGroup();
  const pane = getPane();
  const editorRegistry = getEditorRegistry();
  const dragPaneProps = $derived({ paneGroup, paneId: pane.id });

  const ctx = getEditorContext();
  const editor = new Editor();
  ctx.editor = editor;
  ctx.user = query.data.me;

  $effect(() => {
    ctx.paneFocused = focused;
  });

  const document = $derived(entity?.node.__typename === 'Document' ? entity.node : null);
  const documentId = $derived(document?.id ?? null);
  const title = $derived(document?.title ?? '');
  const serverSnapshot = $derived(
    ctx.serverVersion === null ? (document?.snapshot ? Uint8Array.fromBase64(document.snapshot) : undefined) : ctx.serverSnapshot,
  );
  const serverVersion = $derived(ctx.serverVersion ?? document?.version ?? null);
  const serverGeneration = $derived(ctx.serverVersion === null ? (document?.generation ?? 0) : ctx.serverGeneration);
  const assets = $derived(document?.assets);

  const fontFamilies = $derived(document?.fontFamilies ?? []);
  $effect(() => {
    if (fontFamilies.length > 0) {
      const availableFonts = Object.fromEntries(
        fontFamilies
          .filter((f) => f.state === 'ACTIVE')
          .map((f) => [f.familyName, f.fonts.filter((font) => font.state === 'ACTIVE').map((font) => font.weight)]),
      );
      initWasm().then((wasm) => {
        wasm.setAvailableFonts(availableFonts);
      });
      editor.fontFamilies = fontFamilies;
    }
  });

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
  let titleUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let subtitleUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let persistence: IndexeddbPersistence | null = null;
  let syncPrimed = false;
  let connectionStatus = $state<'connecting' | 'connected' | 'disconnected'>('connecting');
  let showOfflineBanner = $state(false);
  let lastHeartbeatAt = $state(dayjs());

  $effect(() => {
    if (connectionStatus === 'connected') {
      showOfflineBanner = false;
      return;
    }

    const timer = setTimeout(() => {
      showOfflineBanner = true;
    }, 60_000);

    return () => clearTimeout(timer);
  });

  let totalCharacterCountPlanUpgradeModalOpen = $state(false);
  let totalBlobSizePlanUpgradeModalOpen = $state(false);

  let planUpgradeModalOpen = $state(false);
  let fontUploadModalOpen = $state(false);
  let fontPlanUpgradeModalOpen = $state(false);
  let showFindReplace = $state(false);
  const debugStore = new LocalStore<{
    renderDebugEnabled: boolean;
    layoutDebugEnabled: boolean;
  }>('typie:editor:debug', {
    renderDebugEnabled: false,
    layoutDebugEnabled: false,
  });
  let renderDebugEnabled = $state(debugStore.current.renderDebugEnabled);
  let layoutDebugEnabled = $state(debugStore.current.layoutDebugEnabled);
  const showRenderDebugToggle = $derived(dev || query.data.me.role === 'ADMIN' || query.data.impersonation?.admin.role === 'ADMIN');

  setContext('setTotalBlobSizePlanUpgradeModalOpen', () => {
    totalBlobSizePlanUpgradeModalOpen = true;
  });

  const selectionsStore = new LocalStore<Record<string, { selection?: unknown; type?: string; element?: string; timestamp: number }>>(
    'typie:selections',
    {},
  );

  $effect(() => {
    if (!showRenderDebugToggle) {
      return;
    }

    editor.setRenderDebug(renderDebugEnabled);
    editor.setLayoutDebug(layoutDebugEnabled);
    debugStore.current = {
      renderDebugEnabled,
      layoutDebugEnabled,
    };
  });

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();
  let localTitle = $state('');
  let localSubtitle = $state('');
  let titleFocused = $state(false);
  let subtitleFocused = $state(false);
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

      if (!titleDirty && !titleFocused) {
        localTitle = serverTitle;
      }
      if (!subtitleDirty && !subtitleFocused) {
        localSubtitle = serverSubtitle;
      }
    }
  });

  function flushTitleUpdate() {
    if (!titleUpdateTimeout) return;
    clearTimeout(titleUpdateTimeout);
    titleUpdateTimeout = null;
    if (documentId) {
      updateDocument({ input: { documentId, title: localTitle || null } });
    }
  }

  function flushSubtitleUpdate() {
    if (!subtitleUpdateTimeout) return;
    clearTimeout(subtitleUpdateTimeout);
    subtitleUpdateTimeout = null;
    if (documentId) {
      updateDocument({ input: { documentId, subtitle: localSubtitle || null } });
    }
  }

  function handleTitleChanged() {
    if (!documentId) return;
    titleDirty = true;
    if (titleUpdateTimeout) clearTimeout(titleUpdateTimeout);
    titleUpdateTimeout = setTimeout(flushTitleUpdate, 300);
  }

  function handleSubtitleChanged() {
    if (!documentId) return;
    subtitleDirty = true;
    if (subtitleUpdateTimeout) clearTimeout(subtitleUpdateTimeout);
    subtitleUpdateTimeout = setTimeout(flushSubtitleUpdate, 300);
  }

  const currentViewZenModeEnabled = $derived(app.preference.current.zenModeEnabled && pane.id === paneGroup.state.current.focusedPaneId);

  $effect(() => {
    editor.locked = document?.locked ?? false;
  });

  $effect(() => {
    if (app.state.usage.limit.totalCharacterCount === -1) {
      editor.restrictedText = false;
    } else {
      editor.restrictedText = app.state.usage.current.totalCharacterCount >= app.state.usage.limit.totalCharacterCount;
    }

    if (Number(app.state.usage.limit.totalBlobSize) === -1) {
      editor.restrictedBlob = false;
    } else {
      editor.restrictedBlob = Number(app.state.usage.current.totalBlobSize) >= Number(app.state.usage.limit.totalBlobSize);
    }
  });

  let showEditLockedToast = $state(false);
  let lockedToastTimer: ReturnType<typeof setTimeout> | null = null;

  editor.setEditBlockedHandler((reason) => {
    if (reason === 'locked') {
      if (showEditLockedToast) return;
      showEditLockedToast = true;
      lockedToastTimer = setTimeout(() => {
        showEditLockedToast = false;
      }, 5000);
    } else if (reason === 'restrictedText') {
      totalCharacterCountPlanUpgradeModalOpen = true;
    } else if (reason === 'restrictedBlob') {
      totalBlobSizePlanUpgradeModalOpen = true;
    }
  });

  function toggleEditLock() {
    const newValue = !editor.locked;

    if (documentId) {
      updateDocument(
        { input: { documentId, locked: newValue } },
        {
          metadata: {
            cache: {
              optimisticResponse: {
                updateDocument: {
                  id: documentId,
                  title: localTitle || '(제목 없음)',
                  nullableTitle: localTitle || null,
                  subtitle: localSubtitle,
                  locked: newValue,
                },
              },
            },
          },
        },
      );
    }

    Toast.success(
      newValue
        ? '편집 잠금이 설정되었어요. 편집 잠금을 해제하기 전까지 문서를 편집할 수 없어요.'
        : '편집 잠금이 해제되었어요. 이제 문서를 편집할 수 있어요.',
    );

    mixpanel.track(newValue ? 'document_locked' : 'document_unlocked', { via: 'document' });
  }

  $effect(() => {
    if (currentViewZenModeEnabled) {
      Tip.show('editor.zen-mode.enabled', '집중 모드가 활성화되었어요. Esc 키를 눌러 빠져나올 수 있어요.');
    }
  });

  $effect(() => {
    editorRegistry.register(pane.id, mountedSlug, editor);

    return () => {
      editorRegistry.unregister(pane.id, mountedSlug);
    };
  });

  type SyncGuard = {
    documentId: string;
    persistence: IndexeddbPersistence;
  };

  function isSyncGuardActive(guard?: SyncGuard): boolean {
    if (!guard) return true;
    return documentId === guard.documentId && persistence === guard.persistence;
  }

  async function handleSyncPayload(payload: { type: DocumentSyncType; data: string; documentId?: string }, guard?: SyncGuard) {
    if (payload.documentId && payload.documentId !== documentId) return;
    if (!isSyncGuardActive(guard)) return;

    const targetPersistence = guard?.persistence ?? persistence;

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
        if (targetPersistence) await targetPersistence.saveCheckpoint(Uint8Array.fromBase64(payload.data));
        break;
      }
      case DocumentSyncType.RESET: {
        if (targetPersistence) await targetPersistence.clear();
        if (!isSyncGuardActive(guard)) return;
        const reset = JSON.parse(payload.data);
        ctx.serverSnapshot = Uint8Array.fromBase64(reset.snapshot);
        ctx.serverVersion = reset.version;
        ctx.serverGeneration = reset.generation;
        ctx.resetKey++;
        break;
      }
    }
  }

  async function doSync(
    input: { clientId: string; documentId: string; type: DocumentSyncType; data: string },
    options?: { metadata?: { subscription?: { transport?: boolean } } },
    guard?: SyncGuard,
  ) {
    const results = await syncDocument({ input }, options);
    if (!isSyncGuardActive(guard)) return;

    for (const payload of results.syncDocument) {
      if (!isSyncGuardActive(guard)) return;
      await handleSyncPayload(payload, guard);
    }
  }

  $effect(() => {
    const currentDocumentId = documentId;
    if (!currentDocumentId) return;

    documentSyncReady = false;
    syncPrimed = false;
    const runPersistence = new IndexeddbPersistence(currentDocumentId);
    persistence = runPersistence;
    const syncGuard: SyncGuard = { documentId: currentDocumentId, persistence: runPersistence };

    const isActiveRun = () => currentDocumentId === documentId && persistence === runPersistence;

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

    editor.ready.then(async () => {
      if (!isActiveRun()) return;

      const local = await runPersistence.load();
      if (!isActiveRun()) return;

      if (local && runPersistence.generation === serverGeneration) {
        editor.importUpdatesBatch([local.snapshot, ...local.updates]);
      } else {
        await runPersistence.clear();
        if (!isActiveRun()) return;

        if (serverSnapshot) {
          editor.importUpdatesBatch([serverSnapshot]);
        }
        const snapshot = editor.export({ type: 'snapshot' });
        const version = editor.export({ type: 'version' });
        if (snapshot && version && serverVersion) {
          await runPersistence.saveSnapshot(snapshot, version, serverGeneration);
          await runPersistence.saveCheckpoint(Uint8Array.fromBase64(serverVersion));
        }
      }

      if (!isActiveRun()) return;
      editor.contentReady = true;

      // eslint-disable-next-line @typescript-eslint/no-empty-function
      await forceSync(syncGuard).catch(() => {});
      if (!isActiveRun()) return;

      fullSyncInterval = setInterval(() => fullSync(syncGuard), 60_000);
      forceSyncInterval = setInterval(() => forceSync(syncGuard), 10_000);

      await fullSync(syncGuard);
      if (!isActiveRun()) return;

      documentSyncReady = true;
    });

    return () => {
      const canFlushRemoteUpdate = syncPrimed;
      documentSyncReady = false;
      syncPrimed = false;
      if (fullSyncInterval) clearInterval(fullSyncInterval);
      if (forceSyncInterval) clearInterval(forceSyncInterval);
      clearInterval(heartbeatInterval);
      if (syncUpdateTimeout) {
        clearTimeout(syncUpdateTimeout);
        syncUpdateTimeout = null;
      }
      flushTitleUpdate();
      flushSubtitleUpdate();
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
      if (canFlushRemoteUpdate && currentDocumentId && runPersistence.checkpoint.length > 0) {
        const updates = editor.export({ type: 'updates-from', version: runPersistence.checkpoint });
        if (updates?.length) {
          doSync(
            {
              clientId,
              documentId: currentDocumentId,
              type: DocumentSyncType.UPDATE,
              data: updates.toBase64(),
            },
            undefined,
            syncGuard,
          );
        }
      }
      runPersistence.destroy();
      if (persistence === runPersistence) {
        persistence = null;
      }
    };
  });

  async function fullSync(guard?: SyncGuard) {
    const targetDocumentId = guard?.documentId ?? documentId;
    const targetPersistence = guard?.persistence ?? persistence;
    if (!targetDocumentId) return;
    if (!isSyncGuardActive(guard)) return;

    const snapshot = editor.export({ type: 'snapshot' });
    const version = editor.export({ type: 'version' });
    if (targetPersistence && snapshot && version) {
      await targetPersistence.saveSnapshot(snapshot, version);
    }

    if (!isSyncGuardActive(guard)) return;
    if (!syncPrimed) return;

    const update = editor.export({ type: 'all-updates' });
    if (update?.length) {
      await doSync(
        {
          clientId,
          documentId: targetDocumentId,
          type: DocumentSyncType.UPDATE,
          data: update.toBase64(),
        },
        undefined,
        guard,
      );
    }
  }

  async function forceSync(guard?: SyncGuard) {
    const targetDocumentId = guard?.documentId ?? documentId;
    if (!targetDocumentId) return;
    if (!isSyncGuardActive(guard)) return;

    const version = editor.export({ type: 'version' });
    if (!version) return;

    await doSync(
      {
        clientId,
        documentId: targetDocumentId,
        type: DocumentSyncType.VECTOR,
        data: version.toBase64(),
      },
      { metadata: { subscription: { transport: true } } },
      guard,
    );
    if (!isSyncGuardActive(guard)) return;
    syncPrimed = true;
  }

  function handleDocChanged() {
    const currentDocumentId = documentId;
    const currentPersistence = persistence;
    if (!currentDocumentId || !currentPersistence) return;
    const syncGuard: SyncGuard = {
      documentId: currentDocumentId,
      persistence: currentPersistence,
    };

    if (currentPersistence.version.length > 0) {
      const update = editor.export({ type: 'updates-from', version: currentPersistence.version });
      if (update?.length) {
        currentPersistence.saveUpdate(update);
      }
    }

    if (!syncPrimed) return;

    if (syncUpdateTimeout) {
      clearTimeout(syncUpdateTimeout);
    }

    syncUpdateTimeout = setTimeout(async () => {
      if (!isSyncGuardActive(syncGuard)) return;

      const update =
        syncGuard.persistence.checkpoint.length > 0
          ? editor.export({ type: 'updates-from', version: syncGuard.persistence.checkpoint })
          : undefined;

      if (update?.length) {
        await doSync(
          {
            clientId,
            documentId: syncGuard.documentId,
            type: DocumentSyncType.UPDATE,
            data: update.toBase64(),
          },
          { metadata: { subscription: { transport: true } } },
          syncGuard,
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
    editorReady = true;
    onReady?.();

    const saved = selectionsStore.current[documentId];

    if (saved?.selection) {
      const sel = saved.selection as {
        anchor: { nodeId: string; offset: number; affinity: Affinity };
        head: { nodeId: string; offset: number; affinity: Affinity };
      };
      const chain = editor
        .dispatch({
          type: 'setSelection',
          anchorNodeId: sel.anchor.nodeId,
          anchorOffset: sel.anchor.offset,
          anchorAffinity: sel.anchor.affinity,
          headNodeId: sel.head.nodeId,
          headOffset: sel.head.offset,
          headAffinity: sel.head.affinity,
        })
        .scrollIntoView({ mode: 'typewriter' });
      if (focused) {
        chain.focus();
      }
    }

    if (!focused) return;

    if (!saved) {
      titleEl?.focus();
    } else if (saved.type === 'element') {
      if (saved.element === 'title') titleEl?.focus();
      else if (saved.element === 'subtitle') subtitleEl?.focus();
    }
  }

  function focusTitleFromHeader() {
    if (editor.scrollContainerEl) {
      editor.scrollContainerEl.scrollTop = 0;
    }

    titleEl?.focus();
    titleEl?.select();
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    if ((IS_MAC ? e.metaKey : e.ctrlKey) && e.code === 'KeyF' && focused) {
      e.preventDefault();
      showFindReplace = true;
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

{#if document && entity && fontFamilies.length > 0}
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
        use:dragPane={dragPaneProps}
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
            onclick={focusTitleFromHeader}
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

          <FeedbackPopover />

          {#if showRenderDebugToggle}
            <button
              class={flex({
                alignItems: 'center',
                justifyContent: 'center',
                height: '24px',
                paddingX: '7px',
                borderRadius: '4px',
                fontSize: '10px',
                fontWeight: 'semibold',
                whiteSpace: 'nowrap',
                color: layoutDebugEnabled ? 'text.subtle' : 'text.faint',
                backgroundColor: layoutDebugEnabled ? 'surface.muted' : 'transparent',
                transition: 'common',
                _hover: {
                  color: 'text.subtle',
                  backgroundColor: 'surface.muted',
                },
              })}
              aria-pressed={layoutDebugEnabled}
              onclick={() => {
                layoutDebugEnabled = !layoutDebugEnabled;
              }}
              type="button"
              use:tooltip={{
                message: layoutDebugEnabled ? '레이아웃 디버거 끄기' : '레이아웃 디버거 켜기',
                placement: 'left',
                offset: 12,
              }}
            >
              LAYOUT
            </button>

            <button
              class={flex({
                alignItems: 'center',
                justifyContent: 'center',
                height: '24px',
                paddingX: '7px',
                borderRadius: '4px',
                fontSize: '10px',
                fontWeight: 'semibold',
                whiteSpace: 'nowrap',
                color: renderDebugEnabled ? 'text.subtle' : 'text.faint',
                backgroundColor: renderDebugEnabled ? 'surface.muted' : 'transparent',
                transition: 'common',
                _hover: {
                  color: 'text.subtle',
                  backgroundColor: 'surface.muted',
                },
              })}
              aria-pressed={renderDebugEnabled}
              onclick={() => {
                renderDebugEnabled = !renderDebugEnabled;
              }}
              type="button"
              use:tooltip={{
                message: renderDebugEnabled ? '렌더 디버거 끄기' : '렌더 디버거 켜기',
                placement: 'left',
                offset: 12,
              }}
            >
              RENDER
            </button>
          {/if}

          {#if query.data.me.id === entity.user.id}
            <Menu placement="bottom-end">
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

            <button
              class={center({
                borderRadius: '4px',
                size: '24px',
                color: editor.locked ? 'accent.brand.default' : 'text.faint',
                transition: 'common',
                _hover: {
                  color: editor.locked ? 'accent.brand.hover' : 'text.subtle',
                  backgroundColor: 'surface.muted',
                },
              })}
              onclick={() => toggleEditLock()}
              onpointerdown={(e) => e.preventDefault()}
              type="button"
              use:tooltip={{ message: editor.locked ? '편집 잠금 해제' : '편집 잠금' }}
            >
              <Icon icon={editor.locked ? LockIcon : LockOpenIcon} size={16} />
            </button>
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
            onpointerdown={(e) => e.preventDefault()}
            type="button"
            use:tooltip={{
              message: app.preference.current.zenModeEnabled ? '집중 모드 끄기' : '집중 모드 켜기',
              keys: ['Mod', 'Shift', 'M'],
            }}
          >
            <Icon icon={Maximize2Icon} size={16} />
          </button>
          <CloseButton>
            <Icon icon={XIcon} size={16} />
          </CloseButton>
        </div>
      </div>

      <HorizontalDivider color="secondary" />

      <TopToolbar user$key={entity.user} />

      <div class={flex({ position: 'relative', flexGrow: '1', overflowY: 'hidden' })}>
        <div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', overflowX: 'auto' })}>
          <BottomToolbar
            onFontUploadClick={() => {
              if (entity.user.subscription) {
                fontUploadModalOpen = true;
              } else {
                fontPlanUpgradeModalOpen = true;
                mixpanel.track('open_plan_upgrade_modal', { via: 'font_family_upload' });
              }
            }}
            onSearchClick={() => (showFindReplace = !showFindReplace)}
          />

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
              overflowY: 'hidden',
              zIndex: app.preference.current.zenModeEnabled && !currentViewZenModeEnabled ? 'underEditor' : 'editor',
              backgroundColor: 'surface.default',
            })}
          >
            {#if showOfflineBanner}
              <div
                class={flex({
                  position: 'absolute',
                  top: '0',
                  left: '0',
                  right: '0',
                  zIndex: '1',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '8px',
                  paddingX: '20px',
                  paddingY: '8px',
                  backgroundColor: { base: 'amber.50', _dark: 'dark.amber.950' },
                  fontSize: '13px',
                  color: { base: 'amber.700', _dark: 'dark.amber.100' },
                })}
              >
                <Icon
                  style={css.raw({ flexShrink: '0', color: { base: 'amber.400', _dark: 'dark.amber.500' } })}
                  icon={WifiOffIcon}
                  size={14}
                />
                <span>오프라인 상태예요. 변경사항이 기기에 자동으로 저장되고, 온라인일 때 다시 동기화돼요.</span>
              </div>
            {/if}

            {#if showEditLockedToast}
              <div
                class={flex({
                  position: 'absolute',
                  top: currentViewZenModeEnabled ? '60px' : editor.layout?.layoutMode.type === 'paginated' ? '36px' : '12px',
                  right: '12px',
                  zIndex: 'sidebar',
                  alignItems: 'center',
                  gap: '10px',
                  paddingX: '14px',
                  paddingY: '10px',
                  borderRadius: '6px',
                  borderWidth: '1px',
                  borderColor: 'border.default',
                  backgroundColor: 'surface.default',
                  boxShadow: 'small',
                  fontSize: '13px',
                  color: 'text.subtle',
                })}
                onpointerenter={() => {
                  if (lockedToastTimer) {
                    clearTimeout(lockedToastTimer);
                    lockedToastTimer = null;
                  }
                }}
                onpointerleave={() => {
                  lockedToastTimer = setTimeout(() => {
                    showEditLockedToast = false;
                  }, 5000);
                }}
                role="alert"
                transition:fly={{ y: -8, duration: 150 }}
              >
                <Icon style={css.raw({ flexShrink: '0' })} icon={LockIcon} size={14} />
                <span>편집이 잠겨있는 문서예요.</span>
                {#if query.data.me.id === entity.user.id}
                  <button
                    class={css({
                      marginLeft: '4px',
                      paddingX: '8px',
                      paddingY: '4px',
                      borderRadius: '4px',
                      fontSize: '12px',
                      fontWeight: 'medium',
                      color: 'text.default',
                      backgroundColor: 'surface.subtle',
                      cursor: 'pointer',
                      transition: 'common',
                      _hover: { backgroundColor: 'surface.muted' },
                    })}
                    onclick={() => {
                      toggleEditLock();
                      showEditLockedToast = false;
                      if (lockedToastTimer) clearTimeout(lockedToastTimer);
                    }}
                    type="button"
                  >
                    해제하기
                  </button>
                {/if}
              </div>
            {/if}

            <EditorComponent
              active={focused}
              {editor}
              {fontFamilies}
              onDocChanged={handleDocChanged}
              onEditorReady={handleEditorReady}
              onExitedDocumentStart={() => subtitleEl?.focus()}
              onSelectionChanged={handleSelectionChanged}
              resizing={paneGroup.resizing}
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
                    ...(editor.layout?.layoutMode.type === 'paginated' && { paddingBottom: '20px' }),
                  })}
                >
                  <div
                    style:padding-left={editor.layout?.layoutMode.type === 'paginated'
                      ? `${editor.layout.layoutMode.pageMarginLeft * editor.displayZoom}px`
                      : '0'}
                    style:padding-right={editor.layout?.layoutMode.type === 'paginated'
                      ? `${editor.layout.layoutMode.pageMarginRight * editor.displayZoom}px`
                      : '0'}
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
                      onblur={() => {
                        titleFocused = false;
                        flushTitleUpdate();
                      }}
                      onfocus={() => {
                        titleFocused = true;
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
                      onblur={() => {
                        subtitleFocused = false;
                        flushSubtitleUpdate();
                      }}
                      onfocus={() => {
                        subtitleFocused = true;
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

                    {#if editor.layout?.layoutMode.type !== 'paginated'}
                      <HorizontalDivider style={css.raw({ marginTop: '10px' })} />
                    {/if}
                  </div>
                </div>
              {/snippet}
              <SpellcheckPopover {editor} />
            </EditorComponent>
            {#if showFindReplace}
              <DocumentFindReplace close={() => (showFindReplace = false)} {editor} {focused} />
            {/if}
          </div>
        </div>

        <DocumentPanel document$key={document} {editor} user$key={query.data.me} />
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
            onpointerdown={(e) => e.preventDefault()}
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

  <PlanUpgradeModal user$key={query.data.me} bind:open={totalCharacterCountPlanUpgradeModalOpen}>
    현재 플랜의 최대 입력 가능 글자 수를 초과했어요.
    <br />
    FULL ACCESS로 업그레이드하고 이어서 작성하세요.
  </PlanUpgradeModal>

  <PlanUpgradeModal user$key={query.data.me} bind:open={totalBlobSizePlanUpgradeModalOpen}>
    현재 플랜의 최대 업로드 가능 용량을 초과했어요.
    <br />
    FULL ACCESS로 업그레이드하고 이어서 업로드하세요.
  </PlanUpgradeModal>

  <PlanUpgradeModal user$key={query.data.me} bind:open={planUpgradeModalOpen}>
    FULL ACCESS로 업그레이드하면
    <br />
    모든 프리미엄 기능을 무제한으로 사용할 수 있어요.
  </PlanUpgradeModal>

  <EditorV2NoticeModal {focused} />

  <FontUploadModal userId={query.data.me.id} bind:open={fontUploadModalOpen} />
  <PlanUpgradeModal user$key={query.data.me} bind:open={fontPlanUpgradeModalOpen}>
    폰트 업로드 기능은 FULL ACCESS에서 사용할 수 있어요.
  </PlanUpgradeModal>

  {#if query.data.me.sites[0]}
    <DocumentTemplateModal {editor} {focused} site$key={query.data.me.sites[0]} />
  {/if}
{/if}
