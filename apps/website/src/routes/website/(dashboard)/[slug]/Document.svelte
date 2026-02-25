<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { settled } from 'svelte';
  import { setupEditorContext } from '$lib/editor/context.svelte';
  import { graphql } from '$mearie';
  import DocumentEditor from './DocumentEditor.svelte';
  import type { Document_query$key } from '$mearie';

  type Props = {
    query$key: Document_query$key;
    slug: string;
    focused: boolean;
  };

  let { query$key, slug, focused }: Props = $props();

  const query = createFragment(
    graphql(`
      fragment Document_query on Query {
        ...DocumentEditor_query

        entities(slugs: $slugs) {
          id
          slug

          node {
            __typename

            ... on Document {
              id
              snapshot
              version
              generation

              title
              nullableTitle
              subtitle
              documentType: type
              characterCount
              createdAt
              updatedAt

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

                ... on DocumentArchivedNode {
                  id
                  content
                }
              }

              ...DocumentPanel_document
            }
          }
        }
      }
    `),
    () => query$key,
  );

  const ctx = setupEditorContext();

  const entity = $derived(query.data.entities.find((e) => e.slug === slug));

  $effect.pre(() => {
    ctx.documentId = entity?.node.__typename === 'Document' ? entity.node.id : null;
    ctx.serverSnapshot =
      entity?.node.__typename === 'Document' && entity.node.snapshot ? Uint8Array.fromBase64(entity.node.snapshot) : undefined;
    ctx.serverVersion = entity?.node.__typename === 'Document' ? entity.node.version : null;
    ctx.serverGeneration = entity?.node.__typename === 'Document' ? entity.node.generation : 0;
  });

  let mounted = $state(true);

  $effect(() => {
    void ctx.resetKey;

    return () => {
      mounted = false;
      settled().then(() => {
        mounted = true;
      });
    };
  });
</script>

{#if entity?.node.__typename === 'Document'}
  {#if mounted}
    <DocumentEditor {focused} query$key={query.data} {slug} />
  {/if}
{/if}
