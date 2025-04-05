<script lang="ts">
  import { graphql } from '$graphql';
  import Editor from './Editor.svelte';

  const query = graphql(`
    query DashboardSlugPage_Query($slug: String!) {
      post(slug: $slug) {
        id
      }

      ...Editor_query
    }
  `);
</script>

{#key $query.post.id}
  <Editor {$query} />
{/key}
