<script lang="ts" module>
  import { graphql } from '$graphql';

  export const unfurlEmbed = graphql(`
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
</script>
