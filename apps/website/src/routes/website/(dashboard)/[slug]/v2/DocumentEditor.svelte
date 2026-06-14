<script lang="ts">
  import { createFragment, createMutation, createSubscription } from '@mearie/svelte';
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
  import { registerLinkContextMenu } from '$lib/editor-ffi/handlers/link';
  import { graphql } from '$mearie';
  import DocumentMenu from '../../@context-menu/DocumentMenu.svelte';
  import FontUploadModal from '../../FontUploadModal.svelte';
  import { PlanUpgradeDialog } from '../../plan-upgrade-dialog.svelte';
  import TrialPopupExperimentModal from '../../TrialPopupExperimentModal.svelte';
  import CloseButton from '../@pane/CloseButton.svelte';
  import { getPane, getPaneGroup } from '../@pane/context.svelte';
  import { dragPane } from '../@pane/dnd';
  import CommentPopover from './@document-comments/CommentPopover.svelte';
  import DocumentComments from './@document-comments/DocumentComments.svelte';
  import DocumentPanel from './@document-panel/DocumentPanel.svelte';
  import DocumentFindReplace from './DocumentFindReplace.svelte';
  import DocumentTemplateModal from './DocumentTemplateModal.svelte';
  import FeedbackPopover from './FeedbackPopover.svelte';
  import SpellcheckPopover from './SpellcheckPopover.svelte';
  import { Pusher } from './sync/pusher.svelte';
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
          canStartTrial
          surveys
          subscription {
            id
          }
          ...EditorContextV2_user
          ...DocumentPanelV2_user
          ...TrialPopupExperimentModal_user
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

              state {
                graph
              }

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

  graphql(`
    fragment EditorContextV2_user on User {
      id
    }
  `);

  const app = getAppContext();
  const currentSite = $derived(query.data?.me.sites.find((s) => s.id === app.preference.current.currentSiteId) ?? query.data?.me.sites[0]);
  const paneGroup = getPaneGroup();
  const pane = getPane();
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

  const fontFamilies = $derived(document?.fontFamilies ?? []);

  let liveEditorCreated = false;
  let destroyed = false;

  $effect(() => {
    const doc = document;
    if (!doc || liveEditorCreated) return;

    liveEditorCreated = true;
    const graph = doc.state ? Uint8Array.fromBase64(doc.state.graph) : new Uint8Array();

    untrack(async () => {
      try {
        const liveEditor = await Editor.create(
          graph,
          { width: 1, height: 1, scale_factor: browserScaleFactor() },
          theme.currentThemeVariant,
        );

        if (destroyed) {
          liveEditor.destroy();
          return;
        }

        ctx.editor = liveEditor;
        ctx.liveEditor = liveEditor;
      } catch (err) {
        console.error(err);
      }
    });
  });

  $effect(() => {
    const editor = ctx.editor;
    if (!editor) return;

    if (assets) {
      for (const asset of assets) {
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
      }
    }
  });

  const clientId = crypto.randomUUID();
  let titleUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let subtitleUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let pusher = $state<Pusher | null>(null);
  let lastConfirmedHeads = $state<Uint8Array | null>(null);

  const hasHeads = $derived(!!lastConfirmedHeads);

  const [pushDocumentChangesets] = createMutation(
    graphql(`
      mutation DocumentEditorV2_PushChangesets($input: PushDocumentChangesetsInput!) {
        pushDocumentChangesets(input: $input) {
          heads
        }
      }
    `),
  );

  const [pullDocumentChangesets] = createMutation(
    graphql(`
      mutation DocumentEditorV2_PullChangesets($input: PullDocumentChangesetsInput!) {
        pullDocumentChangesets(input: $input) {
          changesets
          heads
        }
      }
    `),
  );

  createSubscription(
    graphql(`
      subscription DocumentEditorV2_ChangesetsUpdated($documentId: ID!, $clientId: String!, $heads: Binary!) {
        documentChangesetsUpdated(documentId: $documentId, clientId: $clientId, heads: $heads) {
          changesets
          heads
        }
      }
    `),
    () => ({
      documentId: documentId ?? '',
      clientId,
      heads: untrack(() => lastConfirmedHeads?.toBase64() ?? ''),
    }),
    () => ({
      skip: ctx.liveEditor === undefined || !hasHeads || !documentId,
      onData: (data) => {
        const editor = ctx.liveEditor;
        if (!editor) return;
        const payload = Uint8Array.fromBase64(data.documentChangesetsUpdated.changesets);
        if (payload.length > 0) {
          editor.receiveRemoteChangeset(payload);
        }
        lastConfirmedHeads = Uint8Array.fromBase64(data.documentChangesetsUpdated.heads);
      },
    }),
  );

  $effect(() => {
    const editor = ctx.liveEditor;
    if (!editor) return;

    const initialHeads = editor.currentHeads();
    lastConfirmedHeads = initialHeads;

    const currentDocumentId = documentId;
    if (!currentDocumentId) return;

    const ps = new Pusher({
      editor,
      documentId: currentDocumentId,
      clientId,
      initialServerHeads: initialHeads,
      pushFn: async (changesets) => {
        const result = await pushDocumentChangesets({
          input: {
            documentId: currentDocumentId,
            clientId,
            changesets: changesets.toBase64(),
          },
        });
        lastConfirmedHeads = Uint8Array.fromBase64(result.pushDocumentChangesets.heads);
      },
    });
    pusher = ps;

    const offStateChanged = editor.on('state_changed', (_, { fields }) => {
      if (fields.includes('doc')) ps.schedule();
    });

    const offExitedDocStart = editor.on('cursor_exited_document_start', () => {
      subtitleEl?.focus();
    });

    const pollIntervalId = setInterval(async () => {
      const ed = ctx.liveEditor;
      const heads = lastConfirmedHeads;
      if (!ed || heads === null) return;
      const result = await pullDocumentChangesets({
        input: { documentId: currentDocumentId, heads: heads.toBase64() },
      });
      const missing = Uint8Array.fromBase64(result.pullDocumentChangesets.changesets);
      if (missing.length > 0) {
        ed.receiveRemoteChangeset(missing);
      }
      lastConfirmedHeads = Uint8Array.fromBase64(result.pullDocumentChangesets.heads);
    }, 10_000);

    return () => {
      clearInterval(pollIntervalId);
      offStateChanged();
      offExitedDocStart();
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
  >('typie:selections:v3', {});

  let titleEl = $state<HTMLTextAreaElement>();
  let subtitleEl = $state<HTMLTextAreaElement>();
  let localTitle = $state('');
  let localSubtitle = $state('');
  let titleFocused = $state(false);
  let subtitleFocused = $state(false);
  let titleDirty = $state(false);
  let subtitleDirty = $state(false);
  let trialPopupExperimentOpen = $state(false);

  const shouldOfferTrialPopupExperiment = $derived(
    Boolean(
      documentId &&
      entity &&
      entity.user.id === query.data.me.id &&
      query.data.me.canStartTrial &&
      !query.data.me.subscription &&
      query.data.me.surveys.includes('trial_popup_content_entry_202605'),
    ),
  );

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

  $effect(() => {
    if (
      trialPopupExperimentOpen ||
      !focused ||
      !(ctx.editor?.focused ?? false) ||
      !shouldOfferTrialPopupExperiment ||
      PlanUpgradeDialog.current
    ) {
      return;
    }

    trialPopupExperimentOpen = true;
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
    const editor = ctx.liveEditor;
    if (editor) {
      editor.readOnly = document?.locked ?? false;
    }
  });

  let showEditLockedToast = $state(false);
  let lockedToastTimer: ReturnType<typeof setTimeout> | null = null;

  function toggleEditLock() {
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
    if ((IS_MAC ? e.metaKey : e.ctrlKey) && e.code === 'KeyF' && focused) {
      e.preventDefault();
      showFindReplace = true;
    }
  }

  onDestroy(() => {
    destroyed = true;
    pusher?.stop();
    flushTitleUpdate();
    flushSubtitleUpdate();
    ctx.liveEditor?.destroy();
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
                                ctx.editor?.focus();
                                ctx.editor?.enqueue({
                                  type: 'navigation',
                                  op: { type: 'move', movement: { type: 'document', direction: 'backward' }, extend: false },
                                });
                              }
                            }}
                            placeholder="부제목을 입력하세요"
                            rows={1}
                            spellcheck="false"
                            bind:value={localSubtitle}
                            use:autosize
                          ></textarea>

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

  {#if documentId}
    <TrialPopupExperimentModal {documentId} user$key={query.data.me} bind:open={trialPopupExperimentOpen} />
  {/if}

  {#if currentSite}
    <DocumentTemplateModal editor={ctx.editor} {focused} site$key={currentSite} />
  {/if}
{/if}
