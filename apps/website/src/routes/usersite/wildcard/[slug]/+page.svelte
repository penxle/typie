<script lang="ts">
  import { graphql } from '$graphql';
  import { HorizontalDivider } from '$lib/components';
  import { TiptapRenderer } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

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
              subtitle
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

{#if $query.entityView.node.__typename === 'PostView'}
  <div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full', backgroundColor: 'gray.100' })}>
    <div
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        paddingY: '80px',
        width: 'full',
        maxWidth: '1200px',
        backgroundColor: 'white',
      })}
    >
      <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: '1000px' })}>
        <div class={css({ fontSize: '28px', fontWeight: 'bold' })}>
          {$query.entityView.node.content.title}
        </div>

        {#if $query.entityView.node.content.subtitle}
          <div class={css({ marginTop: '4px', fontSize: '16px', fontWeight: 'medium' })}>
            {$query.entityView.node.content.subtitle}
          </div>
        {/if}

        <HorizontalDivider style={css.raw({ marginTop: '10px', marginBottom: '20px' })} />
      </div>

      <TiptapRenderer style={css.raw({ width: 'full' })} content={$query.entityView.node.content.body} />
    </div>
  </div>
{/if}
