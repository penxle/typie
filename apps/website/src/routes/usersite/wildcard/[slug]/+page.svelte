<script lang="ts">
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { TiptapRenderer } from '$lib/tiptap';
  import { css } from '$styled-system/css';

  const query = graphql(`
    query UsersiteWildcardSlugPage_Query($origin: String!, $slug: String!) {
      entityView(origin: $origin, slug: $slug) {
        id

        node {
          __typename

          ... on PostView {
            id

            content {
              id
              title
              body
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
{#if $query.entityView.node.__typename === 'PostView'}
  <TiptapRenderer style={css.raw({ borderWidth: '1px', padding: '16px' })} content={$query.entityView.node.content.body} />
{/if}
