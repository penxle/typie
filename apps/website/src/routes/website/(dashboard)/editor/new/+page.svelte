<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';

  const query = graphql(`
    query EditorNewPage_Query {
      me @required {
        id

        sites {
          id
        }
      }
    }
  `);

  const createPost = graphql(`
    mutation EditorNewPage_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id
      }
    }
  `);

  onMount(async () => {
    const resp = await createPost({
      siteId: $query.me.sites[0].id,
    });

    await goto(`/editor/${resp.id}`);
  });
</script>
