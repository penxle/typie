<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { RingSpinner } from '@typie/ui/components';
  import { nanoid } from 'nanoid';
  import { untrack } from 'svelte';
  import { EntityState } from '@/enums';
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { fb } from '$lib/analytics';
  import { getSplitViewContext } from './@split-view/context.svelte';
  import SplitViews from './@split-view/SplitViews.svelte';
  import { collectSlug, findViewIdBySlug, replaceSplitView } from './@split-view/utils';

  const query = graphql(`
    query DashboardSlugPage_Query($slugs: [String!]!) @client {
      me @required {
        id
      }

      entities(slugs: $slugs) {
        id
        slug
        state

        user {
          id
        }

        node {
          __typename
        }
      }

      ...SplitViews_View_query
    }
  `);

  const viewEntity = graphql(`
    mutation DashboardSlugPage_ViewEntity_Mutation($input: ViewEntityInput!) {
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
  `);

  const splitView = getSplitViewContext();

  const slug = $derived(page.params.slug);

  const view = $derived.by(() => splitView.state.current.view);
  const focusedSplitViewId = $derived.by(() => splitView.state.current.focusedViewId);
  const slugs = $derived(collectSlug(view));

  $effect(() => {
    splitView.state.current.enabled = !!(
      splitView.state.current.view &&
      splitView.state.current.view.type === 'container' &&
      splitView.state.current.view.children.length > 1
    );
  });

  const focusedEntity = $derived.by(() => $query && $query.entities.find((entity) => entity.slug === slug));

  $effect(() => {
    if (!slug) return;

    untrack(() => {
      if (!splitView.state.current.view) {
        splitView.state.current.view = { id: nanoid(), slug, type: 'item' };
        splitView.state.current.focusedViewId = splitView.state.current.view.id;
      } else if (slugs.includes(slug)) {
        splitView.state.current.focusedViewId = findViewIdBySlug(splitView.state.current.view, slug);
      } else {
        if (focusedSplitViewId) {
          const newId = nanoid();
          splitView.state.current.view = replaceSplitView(splitView.state.current.view, focusedSplitViewId, slug, newId);
          splitView.state.current.focusedViewId = newId;
        } else {
          splitView.state.current.view = { id: nanoid(), slug, type: 'item' };
          splitView.state.current.focusedViewId = splitView.state.current.view.id;
        }
      }
    });
  });

  afterNavigate(async () => {
    if ($query && $query.me.id === focusedEntity?.user.id && focusedEntity?.state === EntityState.ACTIVE) {
      await viewEntity({ entityId: focusedEntity.id });

      fb.track('ViewContent');
    }
  });

  let loaded = $state(false);
  const load = async () => {
    await query.load({ slugs });
    loaded = true;
  };

  $effect(() => {
    void splitView.state.current.view;
    load();
  });
</script>

{#if loaded && $query && slug && view}
  <SplitViews {$query} {slug} {view} />
{:else}
  <div class={center({ size: 'full' })}>
    <RingSpinner style={css.raw({ size: '24px', color: 'text.subtle' })} />
  </div>
{/if}
