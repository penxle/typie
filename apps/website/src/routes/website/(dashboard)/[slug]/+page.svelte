<script lang="ts">
  import { graphql } from '$graphql';
  import Editor from './Editor.svelte';

  const query = graphql(`
    query DashboardSlugPage_Query($slug: String!) {
      ...Editor_query
    }
  `);
</script>

<Editor {$query} />
