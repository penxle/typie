<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { setupEditorContext } from '$lib/editor/context.svelte';
  import { graphql } from '$mearie';
  import DocumentEditor from './DocumentEditor.svelte';
  import type { Document_query$key } from '$mearie';

  type Props = {
    query$key: Document_query$key;
    slug: string;
    focused: boolean;
    onReady?: () => void;
  };

  let { query$key, slug, focused, onReady }: Props = $props();

  // Document는 slug마다 {#key}로 새로 생성/삭제되므로 생성 시점의 값을 캡처.
  const mountedSlug = slug;

  const query = createFragment(
    graphql(`
      fragment Document_query on Query {
        ...DocumentEditor_query

        entity(slug: $slug) {
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

  const entity = $derived(query.data.entity);
  const documentId = $derived(entity?.node.__typename === 'Document' ? entity.node.id : null);

  $effect(() => {
    ctx.documentId = documentId;
    ctx.serverSnapshot =
      entity?.node.__typename === 'Document' && entity.node.snapshot ? Uint8Array.fromBase64(entity.node.snapshot) : undefined;
    ctx.serverVersion = entity?.node.__typename === 'Document' ? entity.node.version : null;
    ctx.serverGeneration = entity?.node.__typename === 'Document' ? entity.node.generation : 0;
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
    <DocumentEditor {focused} {onReady} query$key={query.data} slug={mountedSlug} />
  {/if}
{/if}
