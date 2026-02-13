<script lang="ts">
  import { fragment, graphql } from '$graphql';
  import { setupEditorContext } from '$lib/editor/context.svelte';
  import DocumentEditor from './DocumentEditor.svelte';
  import type { Document_query } from '$graphql';

  type Props = {
    $query: Document_query;
    slug: string;
    focused: boolean;
  };

  let { $query: _query, slug, focused }: Props = $props();

  const query = fragment(
    _query,
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
  );

  const ctx = setupEditorContext();

  const entity = $derived($query.entities.find((e) => e.slug === slug));

  $effect.pre(() => {
    ctx.documentId = entity?.node.__typename === 'Document' ? entity.node.id : null;
    ctx.serverSnapshot =
      entity?.node.__typename === 'Document' && entity.node.snapshot ? Uint8Array.fromBase64(entity.node.snapshot) : undefined;
    ctx.serverVersion = entity?.node.__typename === 'Document' ? entity.node.version : null;
    ctx.serverGeneration = entity?.node.__typename === 'Document' ? entity.node.generation : 0;
  });
</script>

{#if entity?.node.__typename === 'Document'}
  {#key ctx.resetKey}
    <DocumentEditor {$query} {focused} {slug} />
  {/key}
{/if}
