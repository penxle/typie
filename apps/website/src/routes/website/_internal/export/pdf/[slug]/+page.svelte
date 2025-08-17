<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { TiptapRenderer } from '@typie/ui/tiptap';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { graphql } from '$graphql';

  const query = graphql(`
    query ExportPdfSlugPage_query($slug: String!) {
      entity(slug: $slug) {
        id

        user {
          id
          name
        }

        site {
          id
          fonts {
            id
            name
            weight
            url
          }
        }

        node {
          __typename

          ... on Post {
            id
            title
            subtitle
            body
            createdAt
          }
        }
      }
    }
  `);

  const post = $derived($query.entity.node.__typename === 'Post' ? $query.entity.node : null);

  const urlParams = $derived(page.url.searchParams);

  const pageLayout = $derived({
    width: Number(urlParams.get('width')),
    height: Number(urlParams.get('height')),
    marginTop: Number(urlParams.get('margin-top') ?? 20),
    marginBottom: Number(urlParams.get('margin-bottom') ?? 20),
    marginLeft: Number(urlParams.get('margin-left') ?? 20),
    marginRight: Number(urlParams.get('margin-right') ?? 20),
  });

  $effect(() => {
    if (pageLayout.width && pageLayout.height) {
      document.documentElement.style.setProperty('--page-width', `${pageLayout.width}mm`);
      document.documentElement.style.setProperty('--page-height', `${pageLayout.height}mm`);
      document.documentElement.style.setProperty('--page-margin-top', `${pageLayout.marginTop}mm`);
      document.documentElement.style.setProperty('--page-margin-bottom', `${pageLayout.marginBottom}mm`);
      document.documentElement.style.setProperty('--page-margin-left', `${pageLayout.marginLeft}mm`);
      document.documentElement.style.setProperty('--page-margin-right', `${pageLayout.marginRight}mm`);
    }
  });

  onMount(() => {
    window.notifyExportReady?.();
  });
</script>

{#if post}
  {#if pageLayout.width && pageLayout.height}
    <div style:height={`${pageLayout.height}mm`} class={cx('page-export-viewport', css({ overflowY: 'hidden' }))}>
      <TiptapRenderer
        style={css.raw({ size: 'full' })}
        content={{
          type: 'doc',
          content: post.body.content,
        }}
        forPdf
        {pageLayout}
      />
    </div>
  {:else}
    <TiptapRenderer
      style={css.raw({ size: 'full' })}
      content={{
        type: 'doc',
        content: post.body.content,
      }}
      forPdf
    />
  {/if}
{/if}
