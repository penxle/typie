<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { tick } from 'svelte';
  import { setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { graphql } from '$mearie';
  import DocumentEditor from './DocumentEditor.svelte';
  import type { DocumentV2_query$key } from '$mearie';

  type Props = {
    query$key: DocumentV2_query$key;
    focused: boolean;
    onReady?: () => void;
    onEditorFailed?: (error: unknown) => void;
    onEditorRetry?: () => void;
  };

  let { query$key, focused, onReady, onEditorFailed, onEditorRetry }: Props = $props();

  const query = createFragment(
    graphql(`
      fragment DocumentV2_query on Query {
        ...DocumentEditorV2_query

        entity(slug: $slug) {
          node {
            __typename

            ... on Document {
              id
              title
              documentType: type
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
  let remounting = false;

  const remountEditor = async () => {
    if (remounting) return;
    remounting = true;

    const previousEditor = ctx.editor;
    const previousLiveEditor = ctx.liveEditor;
    onEditorRetry?.();
    mounted = false;

    await tick();

    if (ctx.editor === previousEditor) ctx.editor = undefined;
    if (ctx.liveEditor === previousLiveEditor) ctx.liveEditor = undefined;

    mounted = true;
    remounting = false;
  };
</script>

{#if entity?.node.__typename === 'Document'}
  {#if mounted}
    <DocumentEditor {focused} {onEditorFailed} onEditorRetry={remountEditor} {onReady} query$key={query.data} />
  {/if}
{/if}
