<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';

  const createPost = graphql(`
    mutation Editor_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id
      }
    }
  `);

  onMount(async () => {
    const resp = await createPost({});

    await goto(`/editor/${resp.id}`);
  });
</script>
