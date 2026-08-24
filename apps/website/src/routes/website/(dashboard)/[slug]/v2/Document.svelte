<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { tick } from 'svelte';
  import { setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { getOpenDocuments } from '$lib/prism/open-documents.svelte';
  import { graphql } from '$mearie';
  import { getPane } from '../@pane/context.svelte';
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
          id
          slug

          node {
            __typename

            ... on Document {
              id
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

  const openDocuments = getOpenDocuments();
  const pane = getPane();

  const entity = $derived(query.data.entity);
  const documentId = $derived(entity?.node.__typename === 'Document' ? entity.node.id : null);

  $effect(() => {
    ctx.documentId = documentId;
  });

  $effect(() => {
    const node = entity?.node;
    if (!node || node.__typename !== 'Document') return;

    const paneKey = `${pane.id}:${node.id}`;
    openDocuments.upsert(paneKey, {
      id: node.id,
      entityId: entity.id,
      title: node.nullableTitle ?? null,
      subtitle: node.subtitle ?? null,
      active: focused,
      charCount: ctx.editor?.characterCounts.docWithWhitespace || node.characterCount,
    });

    return () => openDocuments.remove(paneKey);
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
