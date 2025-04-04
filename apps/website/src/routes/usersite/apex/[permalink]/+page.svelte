<script lang="ts">
  import { graphql } from '$graphql';

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = graphql(`
    query UsersiteApexPermalinkPage_Query($permalink: String!) {
      entityViewByPermalink(permalink: $permalink) {
        id
        slug

        site {
          id
          url
        }
      }
    }
  `);
</script>
