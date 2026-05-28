<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { graphql } from '$mearie';
  import DocumentEditor from './DocumentEditor.svelte';
  import type { DocumentV2_query$key } from '$mearie';

  type Props = {
    query$key: DocumentV2_query$key;
    focused: boolean;
    onReady?: () => void;
  };

  let { query$key, focused, onReady }: Props = $props();

  const query = createFragment(
    graphql(`
      fragment DocumentV2_query on Query {
        ...DocumentEditorV2_query

        entity(slug: $slug) {
          id
          slug

          node {
            __typename

            ... on Document {
              id
              state {
                graph
                updatedAt
              }

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

              ...DocumentPanelV2_document
            }
          }
        }
      }
    `),
    () => query$key,
  );

  const ctx = setupEditorContext();

  const entity = $derived(query.data.entity);
  const documentId = $derived(entity?.node.__typename === 'Document' ? entity.node.id : null);

  $effect(() => {
    ctx.documentId = documentId;
  });

  let mounted = $state(true);
  let mountedTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const prevDocumentId = documentId;
    void ctx.resetKey;

    return () => {
      if (prevDocumentId !== null) {
        if (mountedTimer !== null) clearTimeout(mountedTimer);
        mounted = false;
        mountedTimer = setTimeout(() => {
          mountedTimer = null;
          mounted = true;
        }, 0);
      }
    };
  });
</script>

{#if entity?.node.__typename === 'Document'}
  {#if mounted}
    <DocumentEditor {focused} {onReady} query$key={query.data} />
  {/if}
{/if}
