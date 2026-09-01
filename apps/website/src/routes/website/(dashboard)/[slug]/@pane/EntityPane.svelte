<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { EntityState } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';
  import FileXIcon from '~icons/lucide/file-x';
  import XIcon from '~icons/lucide/x';
  import { fb } from '$lib/analytics';
  import { getOpenDocuments } from '$lib/prism/open-documents.svelte';
  import { graphql } from '$mearie';
  import { resolveActiveTreeAncestorIds } from '../../@tree/entity-reveal.svelte';
  import DocumentV2 from '../v2/Document.svelte';
  import CloseButton from './CloseButton.svelte';
  import { getPaneGroup, setupPane } from './context.svelte';
  import PaneHeader from './PaneHeader.svelte';
  import PaneSkeleton from './PaneSkeleton.svelte';
  import TabIcon from './TabIcon.svelte';
  import type { Pane, PaneHeaderPlacement } from './types';

  type EntityPane = Extract<Pane, { kind: 'entity' }>;

  type Props = {
    headerPlacement: PaneHeaderPlacement;
    pane: EntityPane;
  };

  let { headerPlacement, pane }: Props = $props();

  const query = createQuery(
    graphql(`
      query EntityPane_Query($slug: String!) {
        me @required {
          id
        }

        entity(slug: $slug) {
          id
          slug
          state
          icon
          iconColor

          ancestors {
            id
          }

          user {
            id
          }

          node {
            __typename

            ... on Document {
              id
              layoutMode
              nullableTitle
              subtitle
            }
          }
        }

        ...DocumentV2_query
      }
    `),
    () => ({ slug: pane.slug }),
  );

  const [viewEntity] = createMutation(
    graphql(`
      mutation EntityPane_ViewEntity_Mutation($input: ViewEntityInput!) {
        viewEntity(input: $input) {
          id

          user {
            id

            recentlyViewedEntities {
              id
            }
          }
        }
      }
    `),
  );

  const app = getAppContext();
  const paneGroup = getPaneGroup();
  const openDocuments = getOpenDocuments();

  openDocuments.expectPane(pane.id);
  onDestroy(() => openDocuments.expectPane(pane.id));

  const focused = $derived(pane.id === paneGroup.state.current.focusedPaneId);
  const entity = $derived(query.data?.entity);
  const documentLayoutMode = $derived(entity?.node.__typename === 'Document' ? entity.node.layoutMode : null);
  const documentId = $derived(entity?.node.__typename === 'Document' ? entity.node.id : null);
  const documentHeaderVisible = $derived(entity?.state === EntityState.ACTIVE && entity.node.__typename === 'Document');

  $effect(() => {
    if (query.loading) {
      openDocuments.expectPane(pane.id);
      return;
    }

    if (!query.data) return;

    const currentEntity = query.data.entity;
    const node = currentEntity?.node;
    if (currentEntity?.state === EntityState.ACTIVE && node?.__typename === 'Document') {
      openDocuments.upsert(pane.id, {
        kind: 'document',
        documentId: node.id,
        entityId: currentEntity.id,
        title: node.nullableTitle ?? null,
        subtitle: node.subtitle ?? null,
        icon: currentEntity.icon,
        iconColor: currentEntity.iconColor,
        active: focused,
      });
    } else {
      openDocuments.resolvePane(pane.id);
    }
  });

  $effect(() => {
    if (entity && entity.slug !== pane.slug) {
      paneGroup.replacePane(pane.id, { kind: 'entity', slug: entity?.slug });
    }
  });

  $effect(() => {
    if (focused && entity) {
      app.state.ancestors = resolveActiveTreeAncestorIds(
        entity.state,
        entity.ancestors.map((ancestor) => ancestor.id),
      );
    }
  });

  let trackedEntityId: string | null = null;

  $effect(() => {
    if (
      !(focused && entity && query.data && query.data.me.id === entity.user.id && entity.state === EntityState.ACTIVE) ||
      trackedEntityId === entity.id
    ) {
      return;
    }

    trackedEntityId = entity.id;
    viewEntity({ input: { entityId: entity.id } });
    fb.track('ViewContent');
  });

  let editorReady = $state(false);
  let liveEditorFailed = $state(false);

  const showSkeleton = $derived(
    !liveEditorFailed &&
      (!query.data || !entity || (entity.state === EntityState.ACTIVE && entity.node.__typename === 'Document' && !editorReady)),
  );

  setupPane(pane);
</script>

<div
  class={flex({
    position: 'relative',
    size: 'full',
    backgroundColor: 'surface.default',
    overflow: 'hidden',
  })}
  data-pane-id={pane.id}
  onclick={() => {
    paneGroup.focusPane(pane.id);
  }}
  onfocusin={() => {
    paneGroup.focusPane(pane.id);
  }}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      paneGroup.focusPane(pane.id);
    }
  }}
  role="tabpanel"
  tabindex={0}
>
  {#if query.data && entity}
    {#if entity?.state === EntityState.ACTIVE}
      {#if entity?.node.__typename === 'Document'}
        <DocumentV2
          {focused}
          {headerPlacement}
          onEditorFailed={() => {
            liveEditorFailed = true;
          }}
          onEditorRetry={() => {
            liveEditorFailed = false;
            editorReady = false;
          }}
          onReady={() => (editorReady = true)}
          query$key={query.data}
        />
      {/if}
    {:else}
      {@const name = '문서'}
      <div class={flex({ flexDirection: 'column', size: 'full' })}>
        {#if focused}
          <Helmet title={`삭제된 ${name}`} />
          <TabIcon icon="file-x" />
        {/if}

        <PaneHeader placement={headerPlacement}>
          {#snippet fixedActions()}
            {#if !app.preference.current.zenModeEnabled}
              <CloseButton>
                <Icon icon={XIcon} size={16} />
              </CloseButton>
            {/if}
          {/snippet}

          <div class={flex({ alignItems: 'center', gap: '4px', paddingLeft: '8px', fontSize: '12px', color: 'text.subtle' })}>
            <Icon icon={FileXIcon} size={14} />
            <span>삭제된 {name}</span>
          </div>
        </PaneHeader>

        <div class={center({ flexDirection: 'column', gap: '20px', flexGrow: '1', minHeight: '0', textAlign: 'center' })}>
          <Icon style={css.raw({ size: '56px', color: 'text.subtle', '& *': { strokeWidth: '[1.25px]' } })} icon={FileXIcon} />

          <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
            <h1 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.subtle' })}>{name}가 삭제되었어요</h1>
            <p class={css({ fontSize: '14px', color: 'text.faint' })}>
              {name}가 삭제되어 더 이상 접근할 수 없어요.
              <br />
              다른 {name}를 선택해주세요
            </p>
          </div>
        </div>
      </div>
    {/if}
  {/if}

  {#if showSkeleton}
    <div
      class={css({
        position: 'absolute',
        top: documentHeaderVisible ? '37px' : '0',
        right: '0',
        bottom: '0',
        left: '0',
        zIndex: 'overEditor',
        backgroundColor: 'surface.default',
      })}
      out:fade={{ duration: 150 }}
    >
      <PaneSkeleton {documentId} {documentLayoutMode} {headerPlacement} {pane} showHeader={!documentHeaderVisible} />
    </div>
  {/if}
</div>
