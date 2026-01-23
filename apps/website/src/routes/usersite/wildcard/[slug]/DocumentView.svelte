<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, HorizontalDivider } from '@typie/ui/components';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import { Editor as EditorComponent } from '$lib/components/editor';
  import { setEditor } from '$lib/editor/context';
  import { Editor } from '$lib/editor/editor.svelte';
  import ShareLinkPopover from './ShareLinkPopover.svelte';
  import type { UsersiteWildcardSlugPage_DocumentView_entityView } from '$graphql';

  type Props = {
    $entityView: UsersiteWildcardSlugPage_DocumentView_entityView;
  };

  let { $entityView: _entityView }: Props = $props();

  const entityView = fragment(
    _entityView,
    graphql(`
      fragment UsersiteWildcardSlugPage_DocumentView_entityView on EntityView {
        id
        slug
        url

        ancestors {
          id
          slug

          node {
            __typename

            ... on FolderView {
              id
              name
            }
          }
        }

        node {
          __typename

          ... on DocumentView {
            id
            title
            subtitle
            excerpt
            snapshot

            assets {
              __typename

              ... on Image {
                id
                url
                width
                height
                placeholder
              }

              ... on File {
                id
                url
                name
                size
              }

              ... on Embed {
                id
                url
                title
                description
                thumbnailUrl
                html
              }
            }
          }
        }

        site {
          id

          fonts {
            id
            weight
            url

            family {
              id
            }
          }
        }
      }
    `),
  );

  const editor = new Editor();
  setEditor(editor);

  const document = $derived($entityView.node.__typename === 'DocumentView' ? $entityView.node : null);

  const snapshot = $derived(document?.snapshot ? Uint8Array.fromBase64(document.snapshot) : undefined);
  const assets = $derived(document?.assets);

  $effect(() => {
    if (assets) {
      for (const asset of assets) {
        if (asset.__typename === 'Image') {
          editor.imageAssets.set(asset.id, {
            id: asset.id,
            url: asset.url,
            width: asset.width,
            height: asset.height,
            placeholder: asset.placeholder,
          });
        } else if (asset.__typename === 'File') {
          editor.fileAssets.set(asset.id, {
            id: asset.id,
            url: asset.url,
            name: asset.name,
            size: asset.size,
          });
        } else if (asset.__typename === 'Embed') {
          editor.embedAssets.set(asset.id, {
            id: asset.id,
            url: asset.url,
            title: asset.title ?? null,
            description: asset.description ?? null,
            thumbnailUrl: asset.thumbnailUrl ?? null,
            html: asset.html ?? null,
          });
        }
      }
    }
  });

  const fontFaces = $derived(
    $entityView.site.fonts
      .flatMap((font) => [
        `@font-face { font-family: ${font.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
        `@font-face { font-family: ${font.family.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
      ])
      .join('\n'),
  );
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />

  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html '<style type="text/css"' + `>${fontFaces}</` + 'style>'}
</svelte:head>

{#if document}
  <Helmet
    description={document.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/${$entityView.id}` }}
    title={document.title}
  />

  {#if snapshot}
    <EditorComponent {editor} readOnly {snapshot}>
      {#snippet header()}
        <div class={css({ paddingTop: { base: '24px', md: '48px' } })}>
          <div class={flex({ alignItems: 'center', gap: '4px', wrap: 'wrap', marginBottom: { base: '4px', lg: '8px' } })}>
            {#each $entityView.ancestors as ancestor (ancestor.id)}
              {#if ancestor.node.__typename === 'FolderView'}
                <a class={css({ fontSize: { base: '12px', lg: '13px' }, color: 'text.disabled' })} href={`/${ancestor.slug}`}>
                  {ancestor.node.name}
                </a>
                <div class={css({ fontSize: { base: '12px', lg: '13px' }, color: 'text.disabled' })}>/</div>
              {/if}
            {/each}

            {#if $entityView.ancestors.length > 0}
              <div class={css({ fontSize: { base: '12px', lg: '13px' }, color: 'text.subtle' })}>{document.title}</div>
            {/if}
          </div>

          <div class={css({ fontSize: { base: '24px', lg: '28px' }, fontWeight: 'bold' })}>
            {document.title}
          </div>

          {#if document.subtitle}
            <div class={css({ marginTop: '4px', fontSize: { base: '14px', lg: '16px' }, fontWeight: 'medium' })}>
              {document.subtitle}
            </div>
          {/if}

          <div class={flex({ align: 'center', justify: 'flex-end', marginTop: '20px', paddingBottom: '10px' })}>
            <ShareLinkPopover href={$entityView.url} />
          </div>

          <HorizontalDivider style={css.raw({ marginBottom: '20px' })} />
        </div>
      {/snippet}
    </EditorComponent>
  {:else}
    <div class={css({ marginTop: '42px', fontSize: '16px', fontWeight: 'medium', textAlign: 'center', color: 'text.muted' })}>
      문서를 불러올 수 없습니다.
    </div>
  {/if}
{/if}
