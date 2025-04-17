<script lang="ts">
  import { onMount } from 'svelte';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { Helmet, HorizontalDivider, Img, ProtectiveRegion } from '$lib/components';
  import { TiptapRenderer } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Header from './Header.svelte';

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
            excerpt
            maxWidth

            option {
              id
              allowCopies
            }

            coverImage {
              id
              ...Img_image
            }

            body {
              __typename

              ... on PostViewBodyAvailable {
                content
              }

              ... on PostViewBodyUnavailable {
                reason
              }
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

  const meQuery = graphql(`
    query UsersiteWildcardSlugPage_Me_Query @client {
      me {
        id

        ...UsersiteWildcardSlugPage_Header_user
      }
    }
  `);

  let loading = $state(true);

  const load = async () => {
    try {
      await meQuery.load();
    } finally {
      loading = false;
    }
  };

  onMount(() => {
    load();
  });
</script>

{#if $query.entityView.node.__typename === 'PostView'}
  <Helmet
    description={$query.entityView.node.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/${$query.entityView.id}` }}
    title={$query.entityView.node.title}
  />

  <Header $user={$meQuery?.me ?? null} {loading} />

  <div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full', minHeight: 'screen', backgroundColor: 'gray.100' })}>
    <div
      style:--prosemirror-max-width={`${$query.entityView.node.maxWidth}px`}
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        flexGrow: '1',
        paddingX: '20px',
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

      {#if $query.entityView.node.body.__typename === 'PostViewBodyAvailable'}
        {#if $query.entityView.node.option.allowCopies}
          <TiptapRenderer style={css.raw({ width: 'full' })} content={$query.entityView.node.body.content} />
        {:else}
          <ProtectiveRegion>
            <TiptapRenderer style={css.raw({ width: 'full' })} content={$query.entityView.node.body.content} />
          </ProtectiveRegion>
        {/if}
      {:else if $query.entityView.node.body.__typename === 'PostViewBodyUnavailable'}
        <div class={css({ fontSize: '16px', fontWeight: 'medium' })}>
          {$query.entityView.node.body.reason}
        </div>
      {/if}
    </div>
  </div>
{/if}
