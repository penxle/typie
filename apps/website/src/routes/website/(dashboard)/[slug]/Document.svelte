<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Helmet, HorizontalDivider, Icon, Menu } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Tip } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { match } from 'ts-pattern';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import CrownIcon from '~icons/lucide/crown';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FolderIcon from '~icons/lucide/folder';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import XIcon from '~icons/lucide/x';
  import { fragment, graphql } from '$graphql';
  import { BottomToolbar, Editor as EditorComponent, TopToolbar } from '$lib/components/editor';
  import { setEditor } from '$lib/editor/context';
  import { Editor } from '$lib/editor/editor.svelte';
  import DocumentMenu from '../@context-menu/DocumentMenu.svelte';
  import PlanUpgradeModal from '../PlanUpgradeModal.svelte';
  import DocumentPanel from './@document-panel/DocumentPanel.svelte';
  import CloseSplitView from './@split-view/CloseSplitView.svelte';
  import { getSplitViewContext, getViewContext } from './@split-view/context.svelte';
  import { getDragDropContext } from './@split-view/drag-context.svelte';
  import { dragView } from './@split-view/drag-view-action';
  import { getEditorRegistry } from './@split-view/editor-registry.svelte';
  import type { Document_query } from '$graphql';

  type Props = {
    $query: Document_query;
    slug: string;
    focused: boolean;
  };

  let { $query: _query, slug, focused }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment Document_query on Query {
        me @required {
          id
        }

        entities(slugs: $slugs) {
          id
          slug
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
              snapshot
              createdAt
              updatedAt
            }
          }
        }
      }
    `),
  );

  const saveDocumentSnapshotMutation = graphql(`
    mutation Document_SaveDocumentSnapshot_Mutation($input: SaveDocumentSnapshotInput!) {
      saveDocumentSnapshot(input: $input)
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

  const entity = $derived($query.entities.find((e) => e.slug === slug));
  const documentId = $derived(entity?.node.__typename === 'Document' ? entity.node.id : null);
  const title = $derived(entity?.node.__typename === 'Document' ? entity.node.title : '');
  const snapshot = $derived(
    entity?.node.__typename === 'Document' && entity.node.snapshot ? Uint8Array.fromBase64(entity.node.snapshot) : undefined,
  );
  const hasSubscription = $derived(!!entity?.user.subscription);

  const editor = new Editor();
  setEditor(editor);

  const contentWidth = $derived(editor.layout.pageWidth - editor.layout.pageMargin * 2);

  let syncTimer: ReturnType<typeof setTimeout> | null = null;
  let syncStatus = $state<'syncing' | 'synced' | 'error'>('synced');
  let planUpgradeModalOpen = $state(false);

  let subtitleEl = $state<HTMLTextAreaElement>();
  let localTitle = $state('');
  let localSubtitle = $state('');
  let titleDirty = $state(false);
  let subtitleDirty = $state(false);

  $effect(() => {
    if (entity?.node.__typename === 'Document') {
      const serverTitle = entity.node.nullableTitle ?? '';
      const serverSubtitle = entity.node.subtitle ?? '';

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

  function handleTitleKeydown(e: KeyboardEvent) {
    if (e.isComposing) return;
    if (e.key === 'Enter') {
      e.preventDefault();
      subtitleEl?.focus();
    }
  }

  function handleSubtitleKeydown(e: KeyboardEvent) {
    if (e.isComposing) return;
    if (e.key === 'Enter') {
      e.preventDefault();
      // TODO: focus editor
    }
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
    if (!documentId) return;

    return () => {
      if (syncTimer) {
        clearTimeout(syncTimer);
        flushSync();
      }
    };
  });

  $effect(() => {
    const _slug = slug;
    editorRegistry.registerPenxle(viewContext.id, slug, editor);

    return () => {
      editorRegistry.unregister(viewContext.id, _slug);
    };
  });

  function handleDocChanged() {
    syncStatus = 'syncing';
    if (syncTimer) {
      clearTimeout(syncTimer);
    }
    syncTimer = setTimeout(() => {
      flushSync();
    }, 1000);
  }

  async function flushSync() {
    syncTimer = null;
    if (!documentId) return;

    const currentSnapshot = editor.getSnapshot();
    if (currentSnapshot) {
      try {
        await saveDocumentSnapshotMutation({ documentId, snapshot: currentSnapshot.toBase64() });
        syncStatus = 'synced';
      } catch {
        syncStatus = 'error';
      }
    }
  }
</script>

{#if focused}
  <Helmet title={`${title || '(제목 없음)'} 작성 중`} />
{/if}

{#if entity?.node.__typename === 'Document'}
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
              style:background-color={match(syncStatus)
                .with('syncing', () => '#eab308')
                .with('synced', () => '#22c55e')
                .with('error', () => '#ef4444')
                .exhaustive()}
              class={css({ size: '8px', borderRadius: 'full' })}
              use:tooltip={{
                message: match(syncStatus)
                  .with('syncing', () => '저장 중...')
                  .with('synced', () => '저장됨')
                  .with('error', () => '저장 실패')
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

              <DocumentMenu document={entity.node} {entity} via="editor" />
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

      <TopToolbar {hasSubscription} />

      <div class={flex({ position: 'relative', flexGrow: '1', overflowY: 'hidden' })}>
        <div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', overflowX: 'auto' })}>
          <BottomToolbar />

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
            <EditorComponent {editor} onDocChanged={handleDocChanged} {snapshot} unit="cm">
              {#snippet header()}
                <div
                  class={flex({
                    flexDirection: 'column',
                    alignItems: 'center',
                    paddingTop: '40px',
                    paddingX: '48px',
                    backgroundColor: 'surface.muted',
                  })}
                >
                  <textarea
                    style:width={`${contentWidth}px`}
                    class={css({
                      fontSize: '20px',
                      fontWeight: 'bold',
                      textAlign: 'center',
                      overflow: 'hidden',
                      resize: 'none',
                      color: 'text.default',
                      _placeholder: { color: 'text.disabled' },
                    })}
                    maxlength={100}
                    oninput={handleTitleChanged}
                    onkeydown={handleTitleKeydown}
                    placeholder="제목"
                    rows={1}
                    bind:value={localTitle}
                    use:autosize
                  ></textarea>

                  <textarea
                    bind:this={subtitleEl}
                    style:width={`${contentWidth}px`}
                    class={css({
                      marginTop: '4px',
                      fontSize: '16px',
                      fontWeight: 'medium',
                      textAlign: 'center',
                      overflow: 'hidden',
                      resize: 'none',
                      color: 'text.subtle',
                      _placeholder: { color: 'text.disabled' },
                    })}
                    maxlength={100}
                    oninput={handleSubtitleChanged}
                    onkeydown={handleSubtitleKeydown}
                    placeholder="부제목"
                    rows={1}
                    bind:value={localSubtitle}
                    use:autosize
                  ></textarea>
                </div>
              {/snippet}
            </EditorComponent>
          </div>
        </div>

        <DocumentPanel {editor} />
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
{/if}

<PlanUpgradeModal bind:open={planUpgradeModalOpen}>
  FULL ACCESS로 업그레이드하면
  <br />
  모든 프리미엄 기능을 무제한으로 사용할 수 있어요.
</PlanUpgradeModal>
