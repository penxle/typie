<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { fade } from 'svelte/transition';
  import { EntityState } from '#/enums';
  import FileXIcon from '~icons/lucide/file-x';
  import XIcon from '~icons/lucide/x';
  import { fb } from '$lib/analytics';
  import { graphql } from '$mearie';
  import Document from '../Document.svelte';
  import CloseButton from './CloseButton.svelte';
  import { getPaneGroup, setupPane } from './context.svelte';
  import PaneSkeleton from './PaneSkeleton.svelte';
  import type { Pane } from './types';

  type EntityPane = Extract<Pane, { kind: 'entity' }>;

  type Props = {
    pane: EntityPane;
  };

  let { pane }: Props = $props();

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
            }
          }
        }

        ...Document_query
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

  const focused = $derived(pane.id === paneGroup.state.current.focusedPaneId);
  const entity = $derived(query.data?.entity);
  const documentLayoutMode = $derived(entity?.node.__typename === 'Document' ? entity.node.layoutMode : null);

  $effect(() => {
    if (entity && entity.slug !== pane.slug) {
      paneGroup.replacePane(pane.id, { kind: 'entity', slug: entity?.slug });
    }
  });

  $effect(() => {
    if (focused && entity) {
      app.state.ancestors = entity.ancestors.map((ancestor) => ancestor.id);
    }
  });

  let trackedEntityId: string | null = null;

  $effect(() => {
    if (
      focused &&
      entity &&
      query.data &&
      query.data.me.id === entity.user.id &&
      entity.state === EntityState.ACTIVE &&
      trackedEntityId !== entity.id
    ) {
      trackedEntityId = entity.id;
      viewEntity({ input: { entityId: entity.id } });
      fb.track('ViewContent');
    }
  });

  let editorReady = $state(false);

  const showSkeleton = $derived(
    !query.data || !entity || (entity.state === EntityState.ACTIVE && entity.node.__typename === 'Document' && !editorReady),
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
        <Document {focused} onReady={() => (editorReady = true)} query$key={query.data} slug={entity.slug} />
      {/if}
    {:else}
      {@const name = '문서'}
      {#if focused}
        <Helmet title={`삭제된 ${name}`} />
      {/if}

      <div class={center({ flexDirection: 'column', gap: '20px', size: 'full', textAlign: 'center' })}>
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
    {/if}
  {/if}

  {#if showSkeleton}
    <div
      class={css({
        position: 'absolute',
        inset: '0',
        zIndex: 'overEditor',
        backgroundColor: 'surface.default',
      })}
      out:fade={{ duration: 150 }}
    >
      <PaneSkeleton {documentLayoutMode} {pane} />
    </div>

    {#if !app.preference.current.zenModeEnabled}
      <CloseButton style={css.raw({ position: 'absolute', top: '6px', right: '8px', zIndex: 'overEditor' })}>
        <Icon icon={XIcon} size={16} />
      </CloseButton>
    {/if}
  {/if}
</div>
