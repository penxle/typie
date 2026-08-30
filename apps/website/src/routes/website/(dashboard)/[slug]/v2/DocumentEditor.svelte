<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Helmet, HorizontalDivider, Icon, Menu, MenuItem, VerticalDivider } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
  import { Tip, Toast } from '@typie/ui/notification';
  import { LocalStore } from '@typie/ui/state';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { onDestroy, tick, untrack } from 'svelte';
  import { fly } from 'svelte/transition';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import ClockFadingIcon from '~icons/lucide/clock-fading';
  import CrownIcon from '~icons/lucide/crown';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FolderIcon from '~icons/lucide/folder';
  import InfoIcon from '~icons/lucide/info';
  import LightbulbIcon from '~icons/lucide/lightbulb';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import MessageSquareTextIcon from '~icons/lucide/message-square-text';
  import SettingsIcon from '~icons/lucide/settings';
  import SpellCheckIcon from '~icons/lucide/spell-check';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import XIcon from '~icons/lucide/x';
  import { desktop } from '$lib/desktop';
  import { Editor as EditorComponent, EditorFailureOverlay } from '$lib/editor-ffi/components';
  import { CONTEXT_BAR_TRANSIENT_VISIBLE_MS } from '$lib/editor-ffi/components/ui/editor-context-bar.svelte';
  import EditorBreadcrumb from '$lib/editor-ffi/components/ui/EditorBreadcrumb.svelte';
  import EditorFocusModeControl from '$lib/editor-ffi/components/ui/EditorFocusModeControl.svelte';
  import { CONTINUOUS_MIN_WIDTH, CONTINUOUS_VIEW_PADDING, IS_MAC } from '$lib/editor-ffi/constants';
  import { browserScaleFactor, Editor, getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { createAssetHydrator } from '$lib/editor-ffi/handlers/asset-hydration';
  import { registerLinkContextMenu } from '$lib/editor-ffi/handlers/link';
  import { cache, mearieClient } from '$lib/graphql';
  import { getDocumentChannels, getSyncConnection } from '$lib/sync';
  import { graphql } from '$mearie';
  import DocumentMenu from '../../@context-menu/DocumentMenu.svelte';
  import EntityIcon from '../../@context-menu/EntityIcon.svelte';
  import { SubscribeModal } from '../../@subscription/subscribe-modal.svelte';
  import FontUploadModal from '../../FontUploadModal.svelte';
  import CloseButton from '../@pane/CloseButton.svelte';
  import { getPane, getPaneGroup } from '../@pane/context.svelte';
  import { dragPane } from '../@pane/dnd';
  import { getEditorRegistry } from '../@pane/editor-registry.svelte';
  import TabIcon from '../@pane/TabIcon.svelte';
  import CommentPopover from './@document-comments/CommentPopover.svelte';
  import DocumentComments from './@document-comments/DocumentComments.svelte';
  import DocumentPanel from './@document-panel/DocumentPanel.svelte';
  import DocumentPanelTabButton from './@document-panel/DocumentPanelTabButton.svelte';
  import { setupDocumentPanelFocusReturn } from './@document-panel/focus-return.svelte';
  import PrismMarginLayer from './@prism-review/PrismMarginLayer.svelte';
  import PrismReviewButton from './@prism-review/PrismReviewButton.svelte';
  import PrismReviewMargin from './@prism-review/PrismReviewMargin.svelte';
  import DocumentFindReplace from './DocumentFindReplace.svelte';
  import DocumentTemplateModal from './DocumentTemplateModal.svelte';
  import DocumentToolbars from './DocumentToolbars.svelte';
  import { headerVerticalNavigation } from './header-vertical-navigation';
  import SpellcheckPopover from './SpellcheckPopover.svelte';
  import { GapBuffer } from './sync/gap-buffer';
  import { PeerChannel } from './sync/peer-channel';
  import { Pusher } from './sync/pusher.svelte';
  import { RemoteChangesetPipeline } from './sync/remote-changeset-pipeline';
  import { IndexeddbDeltaStore } from './sync/store';
  import type { StableSelection } from '@typie/editor-ffi/browser';
  import type { EditorContextBarSegmentRenderProps } from '$lib/editor-ffi/components/ui/EditorContextBar.svelte';
  import type { DocumentEditorV2_query$key } from '$mearie';
  import type { RemoteChangesetEvent } from './sync/remote-changeset-pipeline';

  type Props = {
    query$key: DocumentEditorV2_query$key;
    focused: boolean;
    onReady?: () => void;
    onEditorFailed?: (error: unknown) => void;
    onEditorRetry?: () => void;
  };

  let { query$key, focused, onReady, onEditorFailed, onEditorRetry }: Props = $props();

  setupDocumentPanelFocusReturn();

  const query = createFragment(
    graphql(`
      fragment DocumentEditorV2_query on Query {
        me @required {
          id
          role
          entitled
          ...EditorContextV2_user
          ...DocumentPanelV2_user
          ...CommentComposerV2_user
          sites {
            id
            ...DocumentTemplateModalV2_site
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
          icon
          iconColor
          ...EntityIcon_entity

          ancestors {
            id
            ...EntityIcon_entity

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
              createdAt
              updatedAt

              assets {
                __typename

                ... on Image {
                  id
                  url
                  originalUrl
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

              ...DocumentPanelV2_document
              ...Editor_document
            }
          }
        }
      }
    `),
    () => query$key,
  );

  const entity = $derived(query.data.entity);

  const [updateDocument] = createMutation(
    graphql(`
      mutation DocumentV2_UpdateDocument_Mutation($input: UpdateDocumentInput!) {
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

  const assetsByIdsQuery = graphql(`
    query DocumentEditorV2_AssetsByIds_Query($slug: String!, $ids: [ID!]!) {
      document(slug: $slug) {
        id
        assetsByIds(ids: $ids) {
          __typename

          ... on Image {
            id
            url
            originalUrl
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
      }
    }
  `);

  graphql(`
    fragment EditorContextV2_user on User {
      id
    }
  `);

  const app = getAppContext();
  const currentSite = $derived(query.data?.me.sites.find((s) => s.id === app.preference.current.currentSiteId) ?? query.data?.me.sites[0]);
  const paneGroup = getPaneGroup();
  const pane = getPane();
  const editorRegistry = getEditorRegistry();
  const dragPaneProps = $derived({ paneGroup, paneId: pane.id });

  const ctx = getEditorContext();
  const theme = getThemeContext();
  ctx.user = query.data.me;

  $effect(() => {
    ctx.paneFocused = focused;
  });

  const document = $derived(entity?.node.__typename === 'Document' ? entity.node : null);
  const documentId = $derived(document?.id ?? null);
  const breadcrumbPathIdentity = $derived(entity ? [...entity.ancestors.map((ancestor) => ancestor.id), entity.id].join('/') : '');
  const breadcrumbViewportId = `editor-breadcrumb-${pane.id}`;
  const isOwner = $derived(query.data.me.id === entity?.user.id || query.data.me.role === 'ADMIN');

  let editorAreaWidth = $state(0);
  const marginAvailable = $derived(Math.min(editorAreaWidth, ctx.editor?.publishedViewport?.width ?? 0));

  // 판이 아직 없으면 컬럼이 들어갈 수 있는지 판정할 수 없다 — 폭을 모르는 동안은 팝오버로 선다
  const marginBodyWidth = $derived.by(() => {
    const editor = ctx.editor;
    const layout = editor?.rootAttrs?.layout_mode;
    if (!editor || !layout) return Infinity;
    const pageWidth = editor.pageSizes[0]?.width;
    return pageWidth ? pageWidth * editor.safeDisplayZoom() : Infinity;
  });

  const title = $derived(document?.title ?? '');
  const assets = $derived(document?.assets);

  type DocumentAsset = NonNullable<typeof assets>[number];

  const putAsset = (editor: Editor, asset: DocumentAsset) => {
    if (asset.__typename === 'Image') {
      editor.imageAssets.set(asset.id, {
        id: asset.id,
        url: asset.url,
        originalUrl: asset.originalUrl,
        width: asset.width,
        height: asset.height,
        placeholder: asset.placeholder,
      });
    } else if (asset.__typename === 'File') {
      ctx.fileAssets.set(asset.id, {
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
  };

  const fontFamilies = $derived(document?.fontFamilies ?? []);

  let liveEditorFailed = $state(false);
  let previewEditorRetry = $state<() => void>();
  const editorSurfaceFailed = $derived(liveEditorFailed || previewEditorRetry !== undefined);

  const handlePreviewEditorFailed = (retry: () => void) => {
    previewEditorRetry = retry;
  };

  const handlePreviewEditorRecovered = () => {
    previewEditorRetry = undefined;
  };

  let liveEditorCreated = false;
  let editorFailureReported = false;
  let destroyed = false;
  let editorStore: IndexeddbDeltaStore | null = null;
  let editorServerHeads: Uint8Array = new Uint8Array();
  let editorServerDurableHeads: Uint8Array = new Uint8Array();

  let channelUnsubscribe: (() => void) | null = null;
  let pendingRemoteEvents: RemoteChangesetEvent[] = [];
  let remoteChangesetPipeline: RemoteChangesetPipeline | undefined;

  const failLiveEditor = (error: unknown) => {
    if (editorFailureReported) return;
    editorFailureReported = true;
    liveEditorFailed = true;
    channelUnsubscribe?.();
    channelUnsubscribe = null;
    pusher?.stop();
    onEditorFailed?.(error);
  };

  const handleEditorBoundaryError = (editor: Editor | undefined, error: unknown) => {
    console.error(error);
    if (editor) {
      editor.fail(error);
    } else {
      failLiveEditor(error);
    }
  };

  const retryLiveEditor = () => {
    onEditorRetry?.();
  };

  const handleEditorOperationError = (editor: Editor, error: unknown) => {
    if (editor.destroyed) return;
    const failure = editor.failure;
    if (failure !== undefined) {
      failLiveEditor(failure);
      return;
    }
    if (!destroyed) console.error(error);
  };

  $effect(() => {
    const failure = ctx.liveEditor?.failure;
    if (failure !== undefined) failLiveEditor(failure);
  });

  let resyncPrep: Promise<void> = Promise.resolve();
  let resyncing = false;

  // Single-flight: a second reload signal (server push racing the pull poll)
  // during the capture gap would re-capture the same editor/store and destroy
  // them twice. The in-flight prep plus the coming snapshot already cover it.
  const beginResync = () => {
    if (resyncing) return;
    resyncing = true;
    const oldEditor = ctx.liveEditor;
    const oldStore = editorStore;
    const oldPusher = pusher;
    resyncPrep = (async () => {
      try {
        await oldPusher?.captureNow();
      } catch (err) {
        console.warn('resync: capture failed; pending edits fall back to the last persisted delta', err);
      }
      if (destroyed) return;
      editorStore = null;
      pendingRemoteEvents = [];
      ctx.editor = undefined;
      ctx.liveEditor = undefined;
      await tick();
      oldEditor?.destroy();
      oldStore?.destroy();
    })().finally(() => {
      resyncing = false;
    });
  };

  $effect(() => {
    const doc = document;
    const currentDocumentId = documentId;
    if (!doc || !currentDocumentId || liveEditorCreated) return;

    liveEditorCreated = true;

    channelUnsubscribe = getDocumentChannels().subscribe(currentDocumentId, {
      onSnapshot: (graph, meta) => {
        untrack(async () => {
          let createdEditor: Editor | undefined;
          let createdStore: IndexeddbDeltaStore | undefined;
          try {
            await resyncPrep;
            const store = new IndexeddbDeltaStore();
            createdStore = store;
            const pendingRecords = await store.load(currentDocumentId);
            const pending = pendingRecords.map((r) => r.changeset);

            editorStore = store;
            editorServerDurableHeads = meta.durableHeads;
            syncSeq = meta.seq;

            let liveEditor: Editor;
            try {
              liveEditor = await Editor.createWithPending(
                graph,
                pending,
                { width: 1, height: 1, scale_factor: browserScaleFactor() },
                theme.currentThemeVariant,
              );
              createdEditor = liveEditor;
            } catch (err) {
              store.destroy();
              failLiveEditor(err);
              return;
            }

            if (destroyed) {
              liveEditor.destroy();
              store.destroy();
              return;
            }

            const queued = pendingRemoteEvents;
            pendingRemoteEvents = [];
            let latestHeads = meta.heads;
            let latestDurableHeads = meta.durableHeads;
            const queuedApplied: Promise<number>[] = [];
            for (const event of queued) {
              for (const payload of event.bundles) {
                if (payload.length === 0) continue;
                queuedApplied.push(liveEditor.receiveRemoteChangeset(payload));
              }
              if (event.seq) syncSeq = event.seq;
              latestHeads = event.heads;
              latestDurableHeads = event.durableHeads;
            }
            if (queuedApplied.length > 0) await Promise.all(queuedApplied);

            editorServerHeads = latestHeads;
            editorServerDurableHeads = latestDurableHeads;
            remoteChangesetPipeline = new RemoteChangesetPipeline(liveEditor, (event) => {
              if (ctx.liveEditor !== liveEditor) return;
              if (event.seq) syncSeq = event.seq;
              if (event.bundles.length === 0 && event.seq) return;
              pusher?.setConfirmedHeads(event.heads);
              pusher?.setDurableHeads(event.durableHeads);
            });
            ctx.editor = liveEditor;
            ctx.liveEditor = liveEditor;
          } catch (err) {
            if (createdEditor?.failure === undefined) {
              console.error(err);
            } else {
              failLiveEditor(createdEditor.failure);
            }
            createdEditor?.destroy();
            createdStore?.destroy();
          }
        });
      },
      onChangesets: (event) => {
        const receivingEditor = ctx.liveEditor;
        const pipeline = remoteChangesetPipeline;
        if (!receivingEditor || !pipeline) {
          pendingRemoteEvents.push(event);
          return;
        }
        void pipeline.apply(event).catch((err) => handleEditorOperationError(receivingEditor, err));
      },
      onReload: () => {
        beginResync();
      },
      onPermanentError: (code) => {
        console.error(`document sync permanently failed: ${code}`);
      },
      onHeadIsolated: (event) => {
        if (event.excluded) {
          Tip.show('stats.bulk-edit-excluded', '대량 편집이 통계에서 자동으로 제외되었어요.', {
            description: '환경설정에서 자동 제외를 끄거나, 타임라인에서 항목별로 포함 여부를 바꿀 수 있어요.',
          });
        }
      },
    });
  });

  $effect(() => {
    const editor = ctx.editor;
    if (!editor) return;

    if (assets) {
      for (const asset of assets) {
        putAsset(editor, asset);
      }
    }
  });

  $effect(() => {
    const editor = ctx.editor;
    const slug = entity?.slug;
    const currentDocumentId = documentId;
    if (!editor || !slug || !currentDocumentId || editor.terminal) return;

    const hydrator = createAssetHydrator<DocumentAsset>({
      hasAsset: (id) => editor.imageAssets.has(id) || ctx.fileAssets.has(id) || editor.embedAssets.has(id) || editor.archivedAssets.has(id),
      fetchAssets: async (ids) => {
        await cache.invalidate({ __typename: 'Document', id: currentDocumentId, $field: 'assetsByIds', $args: { ids } });
        const result = await mearieClient.query(assetsByIdsQuery, { slug, ids });
        return result.document.assetsByIds;
      },
      putAsset: (asset) => putAsset(editor, asset),
    });

    let hydrationQueued = false;
    let stopped = false;
    const updateReferences = () => {
      hydrationQueued = false;
      if (stopped) return;
      void hydrator.update(editor.appliedSnapshot.externalElements.flatMap(({ data }) => (data.id ? [data.id] : [])));
    };
    const scheduleHydration = () => {
      if (hydrationQueued) return;
      hydrationQueued = true;
      // Coalesce document bursts while reading the matching applied snapshot.
      queueMicrotask(updateReferences);
    };

    const stopDocumentWatch = $effect.root(() => {
      $effect(() => {
        void editor.documentRevision;
        untrack(scheduleHydration);
      });
    });
    const retry = () => void hydrator.retry();
    window.addEventListener('online', retry);

    return () => {
      stopped = true;
      stopDocumentWatch();
      window.removeEventListener('online', retry);
      hydrator.destroy();
    };
  });
  let titleUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let subtitleUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let pusher = $state<Pusher | null>(null);
  // Sync cursor: the last Redis-Stream id this client has fully caught up to.
  // Non-reactive — pull/subscription read and advance it without re-subscribing.
  let syncSeq = '';

  $effect(() => {
    const editor = ctx.liveEditor;
    if (!editor || editor.terminal) return;

    const store = editorStore;
    if (!store) return;

    const serverHeads = editorServerHeads;
    const serverDurableHeads = editorServerDurableHeads;

    const currentDocumentId = documentId;
    if (!currentDocumentId) return;

    // Full-load recovery when the client's cursor has fallen out of the stream's
    // retained window (offline past retention) — resync rebuilds the editor in
    // place from a fresh snapshot. Unpushed local edits survive via the IndexedDB
    // delta store, which the rebuild replays as pending.
    const reloadDocument = () => {
      getDocumentChannels().resync(currentDocumentId);
    };

    const refetchFromServer = async () => {
      const ed = ctx.liveEditor;
      if (!ed) return;
      const result = await getSyncConnection().pull(currentDocumentId, syncSeq || null);
      if (result.needsReload) {
        reloadDocument();
        return;
      }
      // O(missing) tail: each entry is a standalone bundle blob.
      const applied: Promise<number>[] = [];
      for (const bytes of result.changesets) {
        if (bytes.length === 0) continue;
        applied.push(ed.receiveRemoteChangeset(bytes));
      }
      if (applied.length > 0) await Promise.all(applied);
      if (result.seq) syncSeq = result.seq;
      pusher?.setConfirmedHeads(result.heads);
      pusher?.setDurableHeads(result.durableHeads);
    };
    const refetchInBackground = () => {
      void refetchFromServer().catch((err) => handleEditorOperationError(editor, err));
    };

    const gap = new GapBuffer({
      partition: (p) => editor.partitionRemoteChangesets(p),
      apply: (ready) => {
        void editor.receiveRemoteChangeset(ready).catch((err) => handleEditorOperationError(editor, err));
      },
      onStuck: () => {
        refetchInBackground();
      },
    });

    const peer = new PeerChannel(currentDocumentId, (cs) => gap.ingest(cs));

    const ps = new Pusher({
      editor,
      documentId: currentDocumentId,
      initialServerHeads: serverHeads,
      initialDurableHeads: serverDurableHeads,
      store,
      pushFn: async (changesets) => {
        return getSyncConnection().push(currentDocumentId, changesets);
      },
      broadcast: (cs) => peer.post(cs),
    });
    pusher = ps;

    let observedDocumentRevision = untrack(() => editor.documentRevision);
    const stopDocumentWatch = $effect.root(() => {
      $effect(() => {
        const revision = editor.documentRevision;
        if (revision === observedDocumentRevision) return;
        observedDocumentRevision = revision;
        untrack(() => ps.schedule());
      });
    });

    const offExitedDocStart = editor.on('cursor_exited_document_start', () => {
      subtitleEl?.focus();
    });

    const pollIntervalId = setInterval(() => {
      refetchInBackground();
    }, 10_000);

    return () => {
      clearInterval(pollIntervalId);
      stopDocumentWatch();
      offExitedDocStart();
      peer.close();
      ps.stop();
      pusher = null;
    };
  });

  $effect(() => {
    const editor = ctx.editor;
    if (!editor || editor.terminal) return;
    return registerLinkContextMenu(editor);
  });

  let fontUploadModalOpen = $state(false);
  let showFindReplace = $state(false);
  let findReplaceComponent = $state<DocumentFindReplace>();

  function toggleFindReplace() {
    if (showFindReplace) findReplaceComponent?.close();
    else showFindReplace = true;
  }

  const selectionsStore = new LocalStore<
    Record<
      string,
      {
        selection?: StableSelection;
        type?: string;
        element?: string;
        timestamp: number;
      }
    >
  >('typie:selections:v4', {});

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();
  let localTitle = $state('');
  let localSubtitle = $state('');
  let titleFocused = $state(false);
  let subtitleFocused = $state(false);
  let titleDirty = $state(false);
  let subtitleDirty = $state(false);

  function clearBodySelectionForHeaderFocus() {
    const editor = ctx.editor;
    if (!editor) return;

    editor.enqueue({ type: 'selection', op: { type: 'unset' } });
  }

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
    if (!SubscribeModal.gate('document_update')) return;
    if (titleUpdateTimeout) clearTimeout(titleUpdateTimeout);
    titleUpdateTimeout = setTimeout(flushTitleUpdate, 300);
  }

  function handleSubtitleChanged() {
    if (!documentId) return;
    subtitleDirty = true;
    if (!SubscribeModal.gate('document_update')) return;
    if (subtitleUpdateTimeout) clearTimeout(subtitleUpdateTimeout);
    subtitleUpdateTimeout = setTimeout(flushSubtitleUpdate, 300);
  }

  function enterDocumentFromHeader() {
    ctx.editor?.focus();
    ctx.editor?.enqueue({
      type: 'navigation',
      op: { type: 'move', movement: { type: 'document', direction: 'backward' }, extend: false },
    });
  }

  const currentViewZenModeEnabled = $derived(app.preference.current.zenModeEnabled && pane.id === paneGroup.state.current.focusedPaneId);

  function toggleZenMode() {
    const enabled = !app.preference.current.zenModeEnabled;
    app.preference.current.zenModeEnabled = enabled;
    mixpanel.track(enabled ? 'zen_mode_enabled' : 'zen_mode_disabled', { via: 'document' });
  }

  $effect(() => {
    const editor = ctx.liveEditor;
    if (editor) {
      editor.readOnly = (document?.locked ?? false) || !query.data.me.entitled;
    }
  });

  let showEditLockedToast = $state(false);
  let lockedToastTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const editor = ctx.liveEditor;
    if (!editor) return;

    editor.editBlockedHandler = () => {
      if (!document?.locked) {
        SubscribeModal.gate('editor_readonly');
        return;
      }
      if (showEditLockedToast) return;
      showEditLockedToast = true;
      lockedToastTimer = setTimeout(() => {
        showEditLockedToast = false;
      }, 5000);
    };

    return () => {
      editor.editBlockedHandler = null;
    };
  });

  function toggleEditLock() {
    if (!SubscribeModal.gate('document_lock')) return;

    const newValue = !(document?.locked ?? false);

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
    const editor = ctx.editor;
    const slug = entity?.slug;
    if (!editor || !slug || editor.terminal) return;

    editorRegistry.register(pane.id, slug, editor);

    return () => {
      editorRegistry.unregister(pane.id, slug);
    };
  });

  let editorReady = false;

  $effect(() => {
    const editor = ctx.liveEditor;
    void editor?.appliedSelectionRevision;
    const currentDocumentId = documentId;
    if (!editor || !currentDocumentId || !editorReady || !editor.focused) return;
    untrack(() => {
      const sel = editor.appliedSnapshot.selection;
      if (!sel) return;
      const frozen = editor.freezeSelection(sel);
      if (!frozen) return;
      selectionsStore.current = {
        ...selectionsStore.current,
        [currentDocumentId]: {
          selection: frozen,
          timestamp: dayjs().valueOf(),
        },
      };
    });
  });

  async function handleEditorReady() {
    const currentDocumentId = documentId;
    if (!currentDocumentId) return;
    const editor = ctx.editor;
    editorReady = true;

    editor?.installCommentDecorations();

    const saved = selectionsStore.current[currentDocumentId];
    const savedSelection = saved?.selection;

    if (savedSelection) {
      const restorePresentation = (() => {
        try {
          let presentation: Promise<void> | undefined;
          editor?.updateNow((request) => {
            request.enqueue({
              type: 'selection',
              op: {
                type: 'set_frozen',
                selection: savedSelection,
              },
            });
            request.enqueue({ type: 'view', op: { type: 'expand_folds_for_selection' } });
            presentation = editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'reveal' });
          });
          return presentation;
        } catch (err) {
          selectionsStore.current = { ...selectionsStore.current, [currentDocumentId]: { timestamp: Date.now() } };
          if (editor) handleEditorOperationError(editor, err);
          return;
        }
      })();
      try {
        await restorePresentation;
      } catch (err) {
        if (editor) handleEditorOperationError(editor, err);
      }
    }

    if (destroyed || documentId !== currentDocumentId || ctx.editor !== editor || editor?.terminal) return;

    if (savedSelection && focused && editor) {
      editor.focus();
    }

    onReady?.();

    if (!focused) return;

    if (!saved) {
      titleEl?.focus();
    } else if (saved.type === 'element') {
      if (saved.element === 'title') titleEl?.focus();
      else if (saved.element === 'subtitle') subtitleEl?.focus();
    }
  }

  function focusTitleFromHeader() {
    if (ctx.editor?.scrollContainerEl) {
      ctx.editor.scrollContainerEl.scrollTop = 0;
    }

    titleEl?.focus();
    titleEl?.select();
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    const targetPaneId = e.target instanceof Element ? e.target.closest<HTMLElement>('[data-pane-id]')?.dataset.paneId : undefined;

    if (!(focused && (IS_MAC ? e.metaKey : e.ctrlKey) && e.code === 'KeyF' && targetPaneId === pane.id)) {
      return;
    }

    e.preventDefault();
    showFindReplace = true;
  }

  onDestroy(() => {
    destroyed = true;
    const currentEditor = ctx.editor;
    const currentLiveEditor = ctx.liveEditor;
    const currentEditorStore = editorStore;
    channelUnsubscribe?.();
    channelUnsubscribe = null;
    pusher?.stop();
    flushTitleUpdate();
    flushSubtitleUpdate();
    queueMicrotask(() => {
      currentLiveEditor?.destroy();
      currentEditorStore?.destroy();
      if (ctx.editor === currentEditor) ctx.editor = undefined;
      if (ctx.liveEditor === currentLiveEditor) ctx.liveEditor = undefined;
    });
  });
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

{#if document && entity && fontFamilies.length > 0}
  {#if focused}
    <Helmet title={desktop ? title || '(제목 없음)' : `${title || '(제목 없음)'} 작성 중`} />
    <TabIcon color={entity.iconColor} icon={entity.icon} />
  {/if}

  <div class={flex({ height: 'full', flex: '1', overflowX: 'auto' })}>
    <div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', overflowX: 'auto' })}>
      <!-- 헤더의 리뷰 버튼도 여백 컨텍스트를 읽는다 — 헤더까지 감싼다 -->
      <PrismReviewMargin
        available={marginAvailable}
        bodyWidth={marginBodyWidth}
        {documentId}
        entityId={entity?.id ?? null}
        myId={query.data.me.id}
      >
        {#snippet children(insets)}
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
              <EntityIcon entity$key={entity} size={14} />
              <button
                class={css({
                  minWidth: '0',
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
              {#if document.locked}
                <span
                  class={center({ flexShrink: '0', color: 'text.faint' })}
                  aria-label="편집이 잠겨있는 문서예요."
                  role="img"
                  use:tooltip={{ message: '편집이 잠겨있는 문서예요.' }}
                >
                  <Icon icon={LockIcon} size={12} />
                </span>
              {/if}
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
                  onclick={() => SubscribeModal.show('document_header')}
                  type="button"
                >
                  <Icon icon={CrownIcon} size={12} />
                  <span>업그레이드</span>
                </button>
              {/if}

              {#if ctx.editor}
                <SpellcheckPopover editor={ctx.editor} />
              {/if}

              <PrismReviewButton />

              <DocumentPanelTabButton icon={InfoIcon} label="정보" tab="info" />
              <DocumentPanelTabButton icon={StickyNoteIcon} label="노트" tab="note" />
              <DocumentPanelTabButton icon={MessageSquareTextIcon} label="코멘트" tab="comment" />
              <DocumentPanelTabButton icon={SpellCheckIcon} label="맞춤법" tab="spellcheck" />
              <DocumentPanelTabButton icon={LightbulbIcon} label="AI 피드백" tab="ai" />
              <DocumentPanelTabButton icon={ClockFadingIcon} label="타임라인" tab="timeline" />
              <DocumentPanelTabButton icon={SettingsIcon} label="본문 설정" tab="settings" />

              <VerticalDivider style={css.raw({ height: '12px' })} />

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

                  <DocumentMenu {document} {entity} via="editor">
                    {#if query.data.me.entitled}
                      <MenuItem icon={document.locked ? LockOpenIcon : LockIcon} onclick={() => toggleEditLock()}>
                        {document.locked ? '편집 잠금 해제' : '편집 잠금'}
                      </MenuItem>
                    {/if}
                  </DocumentMenu>
                </Menu>
              {/if}

              <CloseButton>
                <Icon icon={XIcon} size={16} />
              </CloseButton>
            </div>
          </div>

          <HorizontalDivider color="secondary" />

          <div class={flex({ position: 'relative', flexGrow: '1', overflowY: 'hidden' })}>
            {#if document && documentId && entity}
              <DocumentComments {documentId} entityId={entity.id} {isOwner} me$key={query.data.me} myId={query.data.me.id}>
                <div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', overflowX: 'auto' })}>
                  <DocumentToolbars
                    {documentId}
                    {fontFamilies}
                    onFontUploadClick={() => {
                      if (query.data.me.entitled) {
                        fontUploadModalOpen = true;
                      } else {
                        SubscribeModal.show('font_family_upload');
                      }
                    }}
                    onSearchClick={toggleFindReplace}
                  />

                  <div
                    bind:this={ctx.editorAreaEl}
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
                      zIndex: !currentViewZenModeEnabled && app.preference.current.zenModeEnabled ? 'underEditor' : 'editor',
                      backgroundColor: 'surface.default',
                    })}
                    bind:clientWidth={editorAreaWidth}
                  >
                    {#if showEditLockedToast}
                      <div
                        class={flex({
                          position: 'absolute',
                          top: currentViewZenModeEnabled
                            ? '60px'
                            : ctx.editor?.rootAttrs?.layout_mode.type === 'paginated'
                              ? '36px'
                              : '12px',
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
                          if (!lockedToastTimer) {
                            return;
                          }

                          clearTimeout(lockedToastTimer);
                          lockedToastTimer = null;
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

                    <div
                      class={flex({
                        flexDirection: 'column',
                        flexGrow: '1',
                        minHeight: '0',
                        overflowY: 'hidden',
                      })}
                      inert={editorSurfaceFailed}
                    >
                      {#key ctx.editor}
                        {@const editor = ctx.editor}
                        <svelte:boundary onerror={(error) => handleEditorBoundaryError(editor, error)}>
                          <EditorComponent
                            active={focused}
                            contentInsetLeft={insets.left}
                            contentInsetRight={insets.right}
                            contentMotion={insets.contentMotion}
                            document$key={document}
                            onReady={handleEditorReady}
                          >
                            {#snippet breadcrumb({ state }: EditorContextBarSegmentRenderProps)}
                              <EditorBreadcrumb
                                onPathChange={() => state.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS)}
                                pathIdentity={breadcrumbPathIdentity}
                                viewportId={breadcrumbViewportId}
                              >
                                <nav
                                  class={css({
                                    display: 'flex',
                                    alignItems: 'center',
                                    height: '32px',
                                    paddingLeft: '24px',
                                    paddingRight: '16px',
                                    fontSize: '12px',
                                    fontWeight: 'medium',
                                    color: 'text.subtle',
                                  })}
                                  aria-label="문서 경로"
                                >
                                  <ol class={flex({ alignItems: 'center', gap: '2px', listStyle: 'none', whiteSpace: 'nowrap' })}>
                                    {#each entity.ancestors as ancestor (ancestor.id)}
                                      {#if ancestor.node.__typename === 'Folder'}
                                        <li class={flex({ alignItems: 'center', gap: '4px' })}>
                                          <EntityIcon entity$key={ancestor} fallback={FolderIcon} size={14} />
                                          <span>{ancestor.node.name}</span>
                                        </li>
                                        <li class={css({ display: 'grid', placeItems: 'center', color: 'text.faint' })} aria-hidden="true">
                                          <Icon icon={ChevronRightIcon} size={14} />
                                        </li>
                                      {/if}
                                    {/each}
                                    <li class={flex({ alignItems: 'center', gap: '4px' })} aria-current="page">
                                      <EntityIcon entity$key={entity} size={14} />
                                      <span>{localTitle || '(제목 없음)'}</span>
                                    </li>
                                  </ol>
                                </nav>
                              </EditorBreadcrumb>
                            {/snippet}
                            {#snippet viewControls({ state, presentation }: EditorContextBarSegmentRenderProps)}
                              <EditorFocusModeControl
                                enabled={currentViewZenModeEnabled}
                                onToggle={toggleZenMode}
                                segment={state}
                                visible={presentation.visible}
                              />
                            {/snippet}
                            {#snippet header()}
                              <div
                                class={flex({
                                  flexDirection: 'column',
                                  alignItems: 'center',
                                  paddingTop: '60px',
                                  width: 'full',
                                  ...(ctx.editor?.rootAttrs?.layout_mode.type === 'paginated' && { paddingBottom: '20px' }),
                                })}
                              >
                                <div class={flex({ flexDirection: 'column', flexShrink: '0', width: 'full' })}>
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
                                      clearBodySelectionForHeaderFocus();
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

                                      if (e.key === 'Enter') {
                                        e.preventDefault();
                                        subtitleEl?.focus();
                                      }
                                    }}
                                    placeholder="제목을 입력하세요"
                                    rows={1}
                                    spellcheck="false"
                                    bind:value={localTitle}
                                    use:autosize
                                    use:headerVerticalNavigation={{ down: () => subtitleEl?.focus() }}></textarea>

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
                                      clearBodySelectionForHeaderFocus();
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

                                      if (!localSubtitle && e.key === 'Backspace') {
                                        e.preventDefault();
                                        titleEl?.focus();
                                      }

                                      if (e.key === 'Enter' || (e.key === 'Tab' && !e.shiftKey)) {
                                        e.preventDefault();
                                        enterDocumentFromHeader();
                                      }
                                    }}
                                    placeholder="부제목을 입력하세요"
                                    rows={1}
                                    spellcheck="false"
                                    bind:value={localSubtitle}
                                    use:autosize
                                    use:headerVerticalNavigation={{ up: () => titleEl?.focus(), down: enterDocumentFromHeader }}></textarea>

                                  {#if ctx.editor?.rootAttrs?.layout_mode.type !== 'paginated'}
                                    <HorizontalDivider style={css.raw({ marginTop: '10px' })} />
                                  {/if}
                                </div>
                              </div>
                            {/snippet}
                            <CommentPopover />
                            <PrismMarginLayer contentMotion={insets.contentMotion} />
                          </EditorComponent>
                        </svelte:boundary>
                      {/key}
                    </div>
                    {#if showFindReplace}
                      <DocumentFindReplace bind:this={findReplaceComponent} onclose={() => (showFindReplace = false)} />
                    {/if}
                  </div>
                </div>

                <DocumentPanel
                  document$key={document}
                  editor={ctx.editor}
                  onPreviewEditorFailed={handlePreviewEditorFailed}
                  onPreviewEditorRecovered={handlePreviewEditorRecovered}
                  user$key={query.data.me}
                />
              </DocumentComments>
            {/if}
          </div>

          {#if liveEditorFailed && ctx.editorAreaEl}
            <EditorFailureOverlay
              id={`document-editor-${pane.id}`}
              actionLabel="다시 불러오기"
              contentPosition="viewport"
              minimumWidth={CONTINUOUS_MIN_WIDTH + CONTINUOUS_VIEW_PADDING * 2}
              onAction={retryLiveEditor}
              surfaceElement={ctx.editorAreaEl}
            />
          {:else if previewEditorRetry && ctx.editorAreaEl}
            <EditorFailureOverlay
              id={`document-preview-${pane.id}`}
              actionLabel="다시 불러오기"
              contentPosition="viewport"
              minimumWidth={CONTINUOUS_MIN_WIDTH + CONTINUOUS_VIEW_PADDING * 2}
              onAction={previewEditorRetry}
              surfaceElement={ctx.editorAreaEl}
            />
          {/if}
        {/snippet}
      </PrismReviewMargin>

      {#if currentViewZenModeEnabled && !entity.user.subscription}
        <div
          class={flex({
            position: 'fixed',
            top: '44px',
            right: '18px',
            zIndex: 'editor',
            alignItems: 'center',
            gap: '8px',
          })}
        >
          <button
            class={flex({
              alignItems: 'center',
              gap: '4px',
              height: '[31.5px]',
              paddingX: '8px',
              borderWidth: '1px',
              borderColor: 'border.brand',
              borderRadius: '6px',
              fontSize: '11px',
              fontWeight: 'semibold',
              color: 'text.brand',
              backgroundColor: 'surface.default',
              cursor: 'pointer',
              transition: 'common',
              _hover: { backgroundColor: 'accent.brand.subtle' },
            })}
            onclick={() => SubscribeModal.show('document_zen_mode')}
            type="button"
          >
            <Icon icon={CrownIcon} size={12} />
            <span>업그레이드</span>
          </button>
        </div>
      {/if}
    </div>
  </div>

  <FontUploadModal userId={query.data.me.id} bind:open={fontUploadModalOpen} />

  {#if currentSite}
    <DocumentTemplateModal editor={ctx.editor} {focused} site$key={currentSite} />
  {/if}
{/if}
