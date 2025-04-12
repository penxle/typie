<script lang="ts">
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { Helmet, HorizontalDivider, Img, ProtectiveRegion } from '$lib/components';
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
            title
            subtitle
            body
            excerpt
            maxWidth

            coverImage {
              id
              ...Img_image
            }

            option {
              id
              allowCopies
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
  <Helmet
    description={$query.entityView.node.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/${$query.entityView.id}` }}
    title={$query.entityView.node.title}
  />

  <div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full', minHeight: 'screen', backgroundColor: 'gray.100' })}>
    <div
      style:--prosemirror-max-width={`${$query.entityView.node.maxWidth}px`}
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        flexGrow: '1',
        paddingY: '80px',
        width: 'full',
        maxWidth: '1200px',
        backgroundColor: 'white',
      })}
    >
      {#if $query.entityView.node.coverImage}
        <div class={css({ width: 'full', marginBottom: '40px' })}>
          <Img
            style={css.raw({ width: 'full' })}
            $image={$query.entityView.node.coverImage}
            alt="커버 이미지"
            progressive
            ratio={5 / 2}
            size="full"
          />
        </div>
      {/if}

      <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: 'var(--prosemirror-max-width)' })}>
        <div class={css({ fontSize: '28px', fontWeight: 'bold' })}>
          {$query.entityView.node.title}
        </div>

        {#if $query.entityView.node.subtitle}
          <div class={css({ marginTop: '4px', fontSize: '16px', fontWeight: 'medium' })}>
            {$query.entityView.node.subtitle}
          </div>
        {/if}

        <HorizontalDivider style={css.raw({ marginTop: '10px', marginBottom: '20px' })} />
      </div>

      {#if $query.entityView.node.option.allowCopies}
        <TiptapRenderer style={css.raw({ width: 'full' })} content={$query.entityView.node.body} />
      {:else}
        <ProtectiveRegion>
          <TiptapRenderer style={css.raw({ width: 'full' })} content={$query.entityView.node.body} />
        </ProtectiveRegion>
      {/if}
    </div>
  </div>
{/if}
