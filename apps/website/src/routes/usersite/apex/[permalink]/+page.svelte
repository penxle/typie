<script lang="ts">
  import { graphql } from '$graphql';

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = graphql(`
    query UsersiteApexPermalinkPage_Query($permalink: String!) {
      permalink(permalink: $permalink) {
        siteUrl
        entitySlug
      }
    }
  `);
</script>
