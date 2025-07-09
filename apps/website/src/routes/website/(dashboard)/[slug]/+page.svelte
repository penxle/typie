<script lang="ts">
  import { afterNavigate } from '$app/navigation';
  import { graphql } from '$graphql';
  import { LocalStore } from '$lib/state';
  import Canvas from './@canvas/Canvas.svelte';
  import Post from './Post.svelte';

  const query = graphql(`
    query DashboardSlugPage_Query($slug: String!) {
      me @required {
        id
      }

      entity(slug: $slug) {
        id
        slug

        site {
          id
        }

        user {
          id
        }

        node {
          __typename
        }
      }

      ...DashboardSlugPage_Post_query
    }
  `);

  afterNavigate(() => {
    if ($query.me.id === $query.entity.user.id) {
      const lvp = LocalStore.get<Record<string, string>>('typie:lvp') ?? {};
      lvp[$query.entity.site.id] = $query.entity.slug;
      LocalStore.set('typie:lvp', lvp);
    }
  });
</script>

{#if $query.entity.node.__typename === 'Post'}
  <Post {$query} />
{:else if $query.entity.node.__typename === 'Canvas'}
  <Canvas />
{/if}
