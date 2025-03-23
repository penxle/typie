<script lang="ts">
  import { nanoid } from 'nanoid';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';

  const createPost = graphql(`
    mutation Editor_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input)
    }
  `);

  onMount(async () => {
    const postId = nanoid();

    await createPost({
      postId,
    });

    await goto(`/editor/${postId}`);
  });
</script>
