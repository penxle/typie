<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { graphql } from '$mearie';
  import { saveDocumentView } from '../lib/save-document-view.ts';
  import { consequenceClass } from './action-cards.ts';
  import type { ActionBodyProps } from './action-cards.ts';

  let { input, onReady }: ActionBodyProps = $props();

  const view = $derived(saveDocumentView(input));
  const ids = $derived(view === null ? [] : [view.documentId]);

  const query = createQuery(
    graphql(`
      query DashboardLayout_PrismSaveDocumentBody_Query($ids: [ID!]!) {
        prismEntities(ids: $ids) {
          id

          node {
            __typename

            ... on Document {
              id
              title
            }
          }
        }
      }
    `),
    () => ({ ids }),
    () => ({ skip: ids.length === 0 }),
  );

  const found = $derived(query.data?.prismEntities ?? []);

  const title = $derived.by(() => {
    const documentId = view?.documentId;
    if (documentId === undefined) return;
    const node = found.find(
      (entity) => entity.id === documentId || (entity.node.__typename === 'Document' && entity.node.id === documentId),
    )?.node;
    return node?.__typename === 'Document' ? node.title : undefined;
  });

  const loading = $derived(ids.length > 0 && query.loading);
  const resolved = $derived(ids.length > 0 && !loading && query.error === undefined && title !== undefined);

  $effect(() => {
    onReady(resolved);
  });

  const itemClass = css({ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', lineHeight: '[1.5]' });
  const summaryClass = css({ marginTop: '6px', lineHeight: '[1.5]' });
  const noticeClass = css({ color: 'text.faint' });
</script>

{#if loading}
  <p class={noticeClass}>대상을 확인하고 있어요</p>
{:else if !resolved || view === null}
  <p class={noticeClass}>대상을 찾지 못했어요</p>
{:else}
  <p class={itemClass}>문서 「{title}」</p>
  <p class={summaryClass}>{view.summary}</p>
  <p class={consequenceClass}>타임라인에서 되돌릴 수 있어요.</p>
{/if}
