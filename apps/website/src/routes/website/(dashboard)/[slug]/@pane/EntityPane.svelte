<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { EntityState } from '@/enums';
  import FileXIcon from '~icons/lucide/file-x';
  import XIcon from '~icons/lucide/x';
  import Logo from '$assets/logos/logo.svg?component';
  import { fb } from '$lib/analytics';
  import { graphql } from '$mearie';
  import Document from '../Document.svelte';
  import Editor from '../Editor.svelte';
  import CloseButton from './CloseButton.svelte';
  import { getPaneGroup, setupPane } from './context.svelte';
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

            ... on Post {
              id

              document {
                id

                entity {
                  id
                  slug
                }
              }
            }
          }
        }

        ...Document_query
        ...Editor_query
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

  $effect(() => {
    if (focused && entity) {
      app.state.ancestors = entity.ancestors.map((ancestor) => ancestor.id);
    }
  });

  $effect(() => {
    if (entity?.node.__typename === 'Post' && entity.node.document) {
      const documentSlug = entity.node.document.entity.slug;
      paneGroup.replacePane(pane.id, { kind: 'entity', slug: documentSlug });
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
    paneGroup.state.current.focusedPaneId = pane.id;
  }}
  onfocusin={() => {
    paneGroup.state.current.focusedPaneId = pane.id;
  }}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      paneGroup.state.current.focusedPaneId = pane.id;
    }
  }}
  role="tabpanel"
  tabindex={0}
>
  {#if query.data && entity}
    {#if entity?.state === EntityState.ACTIVE}
      {#if entity?.node.__typename === 'Post'}
        <Editor {focused} query$key={query.data} slug={entity.slug} />
      {:else if entity?.node.__typename === 'Document'}
        <Document {focused} query$key={query.data} slug={entity.slug} />
      {/if}
    {:else}
      {@const name = entity?.node.__typename === 'Post' ? '포스트' : '문서'}
      {#if focused}
        <Helmet title={`삭제된 ${name}`} />
      {/if}

      <CloseButton style={css.raw({ position: 'absolute', top: '6px', right: '8px' })}>
        <Icon icon={XIcon} size={16} />
      </CloseButton>

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
  {:else}
    <div class={center({ size: 'full' })}>
      <CloseButton style={css.raw({ position: 'absolute', top: '6px', right: '8px' })}>
        <Icon icon={XIcon} size={16} />
      </CloseButton>

      <Logo
        class={css({
          size: '32px',
          filter: '[grayscale(100%)]',
          animation: 'pulse 2s ease-in-out infinite',
        })}
      />
    </div>
  {/if}
</div>
