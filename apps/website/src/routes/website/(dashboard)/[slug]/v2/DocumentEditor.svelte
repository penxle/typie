<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Helmet, HorizontalDivider, Icon, Menu } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
  import { Tip, Toast } from '@typie/ui/notification';
  import { LocalStore } from '@typie/ui/state';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { onDestroy, setContext, untrack } from 'svelte';
  import { fly } from 'svelte/transition';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CrownIcon from '~icons/lucide/crown';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FolderIcon from '~icons/lucide/folder';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import XIcon from '~icons/lucide/x';
  import { BottomToolbar, Editor as EditorComponent, TopToolbar } from '$lib/editor-ffi/components';
  import { IS_MAC } from '$lib/editor-ffi/constants';
  import { browserScaleFactor, Editor, getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { createAssetHydrator } from '$lib/editor-ffi/handlers/asset-hydration';
  import { registerLinkContextMenu } from '$lib/editor-ffi/handlers/link';
  import { cache, mearieClient } from '$lib/graphql';
  import { getDocumentChannels, getSyncConnection } from '$lib/sync';
  import { graphql } from '$mearie';
  import DocumentMenu from '../../@context-menu/DocumentMenu.svelte';
  import FontUploadModal from '../../FontUploadModal.svelte';
  import { PlanUpgradeDialog } from '../../plan-upgrade-dialog.svelte';
  import CloseButton from '../@pane/CloseButton.svelte';
  import { getPane, getPaneGroup } from '../@pane/context.svelte';
  import { dragPane } from '../@pane/dnd';
  import { getEditorRegistry } from '../@pane/editor-registry.svelte';
  import CommentPopover from './@document-comments/CommentPopover.svelte';
  import DocumentComments from './@document-comments/DocumentComments.svelte';
  import DocumentPanel from './@document-panel/DocumentPanel.svelte';
  import DocumentFindReplace from './DocumentFindReplace.svelte';
  import DocumentTemplateModal from './DocumentTemplateModal.svelte';
  import FeedbackPopover from './FeedbackPopover.svelte';
  import { headerVerticalNavigation } from './header-vertical-navigation';
  import SpellcheckPopover from './SpellcheckPopover.svelte';
  import { GapBuffer } from './sync/gap-buffer';
  import { PeerChannel } from './sync/peer-channel';
  import { Pusher } from './sync/pusher.svelte';
  import { IndexeddbDeltaStore } from './sync/store';
  import type { StableSelection } from '@typie/editor-ffi/browser';
  import type { DocumentEditorV2_query$key } from '$mearie';

  type Props = {
    query$key: DocumentEditorV2_query$key;
    focused: boolean;
    onReady?: () => void;
  };

  let { query$key, focused, onReady }: Props = $props();

  const query = createFragment(
    graphql(`
      fragment DocumentEditorV2_query on Query {
        me @required {
          id
          role
          subscription {
            id
          }
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

          site {
            id
            name
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
  const siteName = $derived(entity?.site.name ?? '내 스페이스');

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

  const paginatedHeaderPaddingLeft = $derived.by(() => {
    const editor = ctx.editor;
    const layoutMode = editor?.rootAttrs?.layout_mode;
    if (!editor || layoutMode?.type !== 'paginated') return '0';
    return `${layoutMode.page_margin_left * editor.safeDisplayZoom()}px`;
  });

  const paginatedHeaderPaddingRight = $derived.by(() => {
    const editor = ctx.editor;
    const layoutMode = editor?.rootAttrs?.layout_mode;
    if (!editor || layoutMode?.type !== 'paginated') return '0';
    return `${layoutMode.page_margin_right * editor.safeDisplayZoom()}px`;
  });

  const document = $derived(entity?.node.__typename === 'Document' ? entity.node : null);
  const documentId = $derived(document?.id ?? null);
  const isOwner = $derived(query.data.me.id === entity?.user.id || query.data.me.role === 'ADMIN');
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

  let liveEditorCreated = false;
  let destroyed = false;
  let editorStore: IndexeddbDeltaStore | null = null;
  let editorServerHeads: Uint8Array = new Uint8Array();
  let editorServerDurableHeads: Uint8Array = new Uint8Array();

  let channelUnsubscribe: (() => void) | null = null;
  let pendingLiveEvents: { seq: string; bundles: Uint8Array[]; heads: Uint8Array; durableHeads: Uint8Array }[] = [];

  const applyChangesets = (event: { seq: string; bundles: Uint8Array[]; heads: Uint8Array; durableHeads: Uint8Array }) => {
    const editor = ctx.liveEditor;
    if (!editor) return;
    let applied = false;
    for (const payload of event.bundles) {
      if (payload.length === 0) continue;
      editor.receiveRemoteChangeset(payload);
      applied = true;
    }
    if (applied) editor.flush();
    if (event.seq) syncSeq = event.seq;
    if (event.bundles.length === 0 && event.seq) return;
    pusher?.setConfirmedHeads(event.heads);
    pusher?.setDurableHeads(event.durableHeads);
  };

  $effect(() => {
    const doc = document;
    const currentDocumentId = documentId;
    if (!doc || !currentDocumentId || liveEditorCreated) return;

    liveEditorCreated = true;

    channelUnsubscribe = getDocumentChannels().subscribe(currentDocumentId, {
      onSnapshot: (graph, meta) => {
        untrack(async () => {
          try {
            const store = new IndexeddbDeltaStore();
            const pendingRecords = await store.load(currentDocumentId);
            const pending = pendingRecords.map((r) => r.changeset);

            editorStore = store;
            editorServerDurableHeads = meta.durableHeads;
            syncSeq = meta.seq;

            const liveEditor = await Editor.createWithPending(
              graph,
              pending,
              { width: 1, height: 1, scale_factor: browserScaleFactor() },
              theme.currentThemeVariant,
            );

            if (destroyed) {
              liveEditor.destroy();
              store.destroy();
              return;
            }

            const queued = pendingLiveEvents;
            pendingLiveEvents = [];
            let latestHeads = meta.heads;
            let latestDurableHeads = meta.durableHeads;
            let queuedApplied = false;
            for (const event of queued) {
              for (const payload of event.bundles) {
                if (payload.length === 0) continue;
                liveEditor.receiveRemoteChangeset(payload);
                queuedApplied = true;
              }
              if (event.seq) syncSeq = event.seq;
              latestHeads = event.heads;
              latestDurableHeads = event.durableHeads;
            }
            if (queuedApplied) liveEditor.flush();

            editorServerHeads = latestHeads;
            editorServerDurableHeads = latestDurableHeads;
            ctx.editor = liveEditor;
            ctx.liveEditor = liveEditor;
          } catch (err) {
            console.error(err);
          }
        });
      },
      onChangesets: (event) => {
        if (!ctx.liveEditor) {
          pendingLiveEvents.push(event);
          return;
        }
        applyChangesets(event);
      },
      onReload: () => {
        location.reload();
      },
      onPermanentError: (code) => {
        console.error(`document sync permanently failed: ${code}`);
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
    if (!editor || !slug || !currentDocumentId) return;

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
      void hydrator.update(editor.externalElements.flatMap(({ data }) => (data.id ? [data.id] : [])));
    };
    const scheduleHydration = () => {
      if (hydrationQueued) return;
      hydrationQueued = true;
      // Editor invalidates the lazy external-elements cache after emitting state_changed.
      queueMicrotask(updateReferences);
    };

    scheduleHydration();
    const offStateChanged = editor.on('state_changed', (_, { fields }) => {
      if (fields.includes('doc')) scheduleHydration();
    });
    const retry = () => void hydrator.retry();
    window.addEventListener('online', retry);

    return () => {
      stopped = true;
      offStateChanged();
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
    if (!editor) return;

    const store = editorStore;
    if (!store) return;

    const serverHeads = editorServerHeads;
    const serverDurableHeads = editorServerDurableHeads;

    const currentDocumentId = documentId;
    if (!currentDocumentId) return;

    // Full-load recovery when the client's cursor has fallen out of the stream's
    // retained window (offline past retention) — reload rebuilds the editor from
    // the fresh snapshot. Unpushed local edits survive via the IndexedDB delta
    // store, which the rebuild replays as pending.
    const reloadDocument = () => {
      location.reload();
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
      let applied = false;
      for (const bytes of result.changesets) {
        if (bytes.length === 0) continue;
        ed.receiveRemoteChangeset(bytes);
        applied = true;
      }
      if (applied) ed.flush();
      if (result.seq) syncSeq = result.seq;
      pusher?.setConfirmedHeads(result.heads);
      pusher?.setDurableHeads(result.durableHeads);
    };

    const gap = new GapBuffer({
      partition: (p) => editor.partitionRemoteChangesets(p),
      apply: (ready) => {
        editor.receiveRemoteChangeset(ready);
        editor.flush();
      },
      onStuck: () => {
        void refetchFromServer();
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

    const offStateChanged = editor.on('state_changed', (_, { fields }) => {
      if (fields.includes('doc')) ps.schedule();
    });

    const offExitedDocStart = editor.on('cursor_exited_document_start', () => {
      subtitleEl?.focus();
    });

    const pollIntervalId = setInterval(() => {
      void refetchFromServer();
    }, 10_000);

    return () => {
      clearInterval(pollIntervalId);
      offStateChanged();
      offExitedDocStart();
      peer.close();
      ps.stop();
      pusher = null;
    };
  });

  $effect(() => {
    const editor = ctx.editor;
    if (!editor) return;
    return registerLinkContextMenu(editor);
  });

  let fontUploadModalOpen = $state(false);
  let showFindReplace = $state(false);

  setContext('setTotalBlobSizePlanUpgradeModalOpen', () => {
    PlanUpgradeDialog.show({
      message: '현재 플랜의 최대 업로드 가능 용량을 초과했어요.\nFULL ACCESS로 업그레이드하고 이어서 업로드하세요.',
    });
  });

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

  function enterDocumentFromHeader() {
    ctx.editor?.focus();
    ctx.editor?.enqueue({
      type: 'navigation',
      op: { type: 'move', movement: { type: 'document', direction: 'backward' }, extend: false },
    });
  }

  const currentViewZenModeEnabled = $derived(app.preference.current.zenModeEnabled && pane.id === paneGroup.state.current.focusedPaneId);

  $effect(() => {
    const editor = ctx.liveEditor;
    if (editor) {
      editor.readOnly = (document?.locked ?? false) || !query.data.me.subscription;
    }
  });

  let showEditLockedToast = $state(false);
  let lockedToastTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const editor = ctx.liveEditor;
    if (!editor) return;

    editor.editBlockedHandler = () => {
      if (!document?.locked || showEditLockedToast) return;
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
    if (!query.data.me.subscription) return;

    const newValue = !(ctx.editor?.readOnly ?? false);

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
    if (!editor || !slug) return;

    editorRegistry.register(pane.id, slug, editor);

    return () => {
      editorRegistry.unregister(pane.id, slug);
    };
  });

  let editorReady = false;

  $effect(() => {
    const editor = ctx.liveEditor;
    if (!editor) return;

    return editor.on('state_changed', (_, { fields }) => {
      if (!fields.includes('selection')) return;
      const sel = editor.selection;
      if (!sel || !documentId || !editorReady || !editor.focused) return;
      const frozen = editor.freezeSelection(sel);
      if (!frozen) return;
      selectionsStore.current = {
        ...selectionsStore.current,
        [documentId]: {
          selection: frozen,
          timestamp: dayjs().valueOf(),
        },
      };
    });
  });

  function handleEditorReady() {
    if (!documentId) return;
    editorReady = true;
    onReady?.();

    ctx.editor?.installCommentDecorations();

    const saved = selectionsStore.current[documentId];

    if (saved?.selection) {
      try {
        ctx.editor?.enqueue({
          type: 'selection',
          op: {
            type: 'set_frozen',
            selection: saved.selection,
          },
        });
        ctx.scroll?.scrollIntoView({ target: { type: 'current_selection_head' } });
        if (focused) {
          ctx.editor?.focus();
        }
      } catch {
        selectionsStore.current = { ...selectionsStore.current, [documentId]: { timestamp: Date.now() } };
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
    if (ctx.editor?.scrollContainerEl) {
      ctx.editor.scrollContainerEl.scrollTop = 0;
    }

    titleEl?.focus();
    titleEl?.select();
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    if (!((IS_MAC ? e.metaKey : e.ctrlKey) && e.code === 'KeyF' && focused)) {
      return;
    }

    e.preventDefault();
    showFindReplace = true;
  }

  onDestroy(() => {
    destroyed = true;
    channelUnsubscribe?.();
    channelUnsubscribe = null;
    pusher?.stop();
    flushTitleUpdate();
    flushSubtitleUpdate();
    ctx.liveEditor?.destroy();
    editorStore?.destroy();
    ctx.editor = undefined;
    ctx.liveEditor = undefined;
  });
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

          <div
            class={css({
              flex: 'none',
              maxWidth: '160px',
              fontSize: '12px',
              color: 'text.disabled',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
            })}
            title={siteName}
          >
            {siteName}
          </div>
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
                PlanUpgradeDialog.show({
                  message: 'FULL ACCESS로 업그레이드하면\n모든 프리미엄 기능을 무제한으로 사용할 수 있어요.',
                });
                mixpanel.track('open_plan_upgrade_modal', { via: 'document_header' });
              }}
              type="button"
            >
              <Icon icon={CrownIcon} size={12} />
              <span>업그레이드</span>
            </button>
          {/if}

          <FeedbackPopover />
          {#if ctx.editor}
            <SpellcheckPopover editor={ctx.editor} />
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

            {#if query.data.me.subscription}
              <button
                class={center({
                  borderRadius: '4px',
                  size: '24px',
                  color: (ctx.editor?.readOnly ?? false) ? 'accent.brand.default' : 'text.faint',
                  transition: 'common',
                  _hover: {
                    color: (ctx.editor?.readOnly ?? false) ? 'accent.brand.hover' : 'text.subtle',
                    backgroundColor: 'surface.muted',
                  },
                })}
                onclick={() => toggleEditLock()}
                onpointerdown={(e) => e.preventDefault()}
                type="button"
                use:tooltip={{ message: (ctx.editor?.readOnly ?? false) ? '편집 잠금 해제' : '편집 잠금' }}
              >
                <Icon icon={(ctx.editor?.readOnly ?? false) ? LockIcon : LockOpenIcon} size={16} />
              </button>
            {/if}
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

      <TopToolbar />

      <div class={flex({ position: 'relative', flexGrow: '1', overflowY: 'hidden' })}>
        {#if document && documentId && entity}
          <DocumentComments {documentId} entityId={entity.id} {isOwner} me$key={query.data.me} myId={query.data.me.id}>
            <div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', overflowX: 'auto' })}>
              <BottomToolbar
                {fontFamilies}
                onFontUploadClick={() => {
                  if (entity.user.subscription) {
                    fontUploadModalOpen = true;
                  } else {
                    PlanUpgradeDialog.show({
                      message: '폰트 업로드 기능은 FULL ACCESS에서 사용할 수 있어요.',
                    });
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
                {#if showEditLockedToast}
                  <div
                    class={flex({
                      position: 'absolute',
                      top: currentViewZenModeEnabled ? '60px' : ctx.editor?.rootAttrs?.layout_mode.type === 'paginated' ? '36px' : '12px',
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

                {#key ctx.editor}
                  <EditorComponent active={focused} document$key={document} onReady={handleEditorReady}>
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
                        <div
                          style:padding-left={paginatedHeaderPaddingLeft}
                          style:padding-right={paginatedHeaderPaddingRight}
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

                              if (e.key === 'Backspace' && !localSubtitle) {
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
                  </EditorComponent>
                {/key}
                {#if showFindReplace}
                  <DocumentFindReplace close={() => (showFindReplace = false)} />
                {/if}
              </div>
            </div>

            <DocumentPanel document$key={document} editor={ctx.editor} user$key={query.data.me} />
          </DocumentComments>
        {/if}
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
                PlanUpgradeDialog.show({
                  message: 'FULL ACCESS로 업그레이드하면\n모든 프리미엄 기능을 무제한으로 사용할 수 있어요.',
                });
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

  <FontUploadModal userId={query.data.me.id} bind:open={fontUploadModalOpen} />

  {#if currentSite}
    <DocumentTemplateModal editor={ctx.editor} {focused} site$key={currentSite} />
  {/if}
{/if}
