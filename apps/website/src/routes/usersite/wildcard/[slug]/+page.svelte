<script lang="ts">
  import { page } from '$app/state';
  import { graphql } from '$graphql';

  const query = graphql(`
    query UsersiteWildcardSlugPage_Query($slug: String!) {
      entityView(slug: $slug) {
        id

        node {
          ... on PostView {
            id

            content {
              id
              title
            }
          }

          ... on FolderView {
            id
            name
          }
        }
      }
    }
  `);
</script>

usersite wildcard
<br />
host: {page.url.hostname}
<br />
slug: {page.params.slug}
<br />
{JSON.stringify($query)}
