<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { z } from 'zod';
  import { graphql } from '$mearie';
  import { consequenceClass } from './action-cards.ts';
  import type { ActionBodyProps } from './action-cards.ts';

  let { input, onReady }: ActionBodyProps = $props();

  const TargetsSchema = z.object({ ids: z.array(z.string()).min(1) });

  const parsed = $derived(TargetsSchema.safeParse(input));
  const ids = $derived(parsed.success ? [...new Set(parsed.data.ids)] : []);

  const query = createQuery(
    graphql(`
      query DashboardLayout_PrismDeleteEntitiesBody_Query($ids: [ID!]!) {
        prismEntities(ids: $ids) {
          id

          node {
            __typename

            ... on Document {
              id
              title
            }

            ... on Folder {
              id
              name
            }

            ... on Divider {
              id
            }
          }
        }
      }
    `),
    () => ({ ids }),
    () => ({ skip: ids.length === 0 }),
  );

  const found = $derived(query.data?.prismEntities ?? []);

  const targets = $derived(
    ids.map((id) => {
      const node = found.find((entity) => entity.id === id || entity.node.id === id)?.node;
      if (node === undefined) return null;
      if (node.__typename === 'Folder') return { kind: 'folder' as const, name: node.name };
      if (node.__typename === 'Document') return { kind: 'document' as const, name: node.title };
      if (node.__typename === 'Divider') return { kind: 'divider' as const };
      return null;
    }),
  );

  const loading = $derived(ids.length > 0 && query.loading);
  const resolved = $derived(ids.length > 0 && !loading && query.error === undefined && targets.every((target) => target !== null));
  const hasFolder = $derived(resolved && targets.some((target) => target?.kind === 'folder'));
  const hasRecoverable = $derived(resolved && targets.some((target) => target !== null && target.kind !== 'divider'));

  $effect(() => {
    onReady(resolved);
  });

  const listClass = flex({ flexDirection: 'column', gap: '4px' });
  const itemClass = css({ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', lineHeight: '[1.5]' });
  const noticeClass = css({ color: 'text.faint' });
</script>

{#if loading}
  <p class={noticeClass}>대상을 확인하고 있어요</p>
{:else if !resolved}
  <p class={noticeClass}>대상을 찾지 못했어요</p>
{:else}
  <div class={listClass}>
    {#each targets as target, index (ids[index])}
      {#if target}
        {#if target.kind === 'divider'}
          <p class={itemClass}>구분선</p>
        {:else}
          <p class={itemClass}>{target.kind === 'folder' ? '폴더' : '문서'} 「{target.name}」</p>
        {/if}
      {/if}
    {/each}
  </div>

  {#if hasRecoverable}
    <p class={consequenceClass}>{hasFolder ? '폴더 안의 것도 함께 삭제돼요. ' : ''}삭제 후 30일 동안 휴지통에 보관돼요.</p>
  {/if}
{/if}
