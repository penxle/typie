<script lang="ts" module>
  import { mearieClient } from '$lib/graphql';
  import { graphql } from '$mearie';
  import type { VariablesOf } from '@mearie/svelte';

  const unfurlEmbedMutation = graphql(`
    mutation EmbedNodeView_UnfurlEmbed_Mutation($input: UnfurlEmbedInput!) {
      unfurlEmbed(input: $input) {
        id
        url
        title
        description
        thumbnailUrl
        html
      }
    }
  `);

  export const unfurlEmbed = (variables: VariablesOf<typeof unfurlEmbedMutation>) => mearieClient.mutation(unfurlEmbedMutation, variables);
</script>
