<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { z } from 'zod';
  import { graphql } from '$mearie';
  import { consequenceClass } from './action-cards.ts';
  import type { ActionBodyProps } from './action-cards.ts';

  let { input, onReady }: ActionBodyProps = $props();

  const TargetsSchema = z.object({ items: z.array(z.object({ id: z.string().min(1).optional() })).min(1) });

  const parsed = $derived(TargetsSchema.safeParse(input));
  const items = $derived(parsed.success ? parsed.data.items : []);
  const ids = $derived([...new Set(items.flatMap((item) => (item.id === undefined ? [] : [item.id])))]);
  const hasUserGoal = $derived(items.some((item) => item.id === undefined));

  const query = createQuery(
    graphql(`
      query DashboardLayout_PrismDeleteGoalBody_Query($ids: [ID!]!) {
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
  const titles = $derived(
    ids.map((id) => {
      const node = found.find((entity) => entity.id === id || entity.node.id === id)?.node;
      if (node === undefined) return null;
      if (node.__typename === 'Folder') return node.name;
      if (node.__typename === 'Document') return node.title;
      return null;
    }),
  );

  const loading = $derived(ids.length > 0 && query.loading);
  const resolved = $derived(items.length > 0 && !loading && query.error === undefined && titles.every((title) => title !== null));

  $effect(() => {
    onReady(resolved);
  });

  const listClass = flex({ flexDirection: 'column', gap: '4px' });
  const itemClass = css({ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', lineHeight: '[1.5]' });
  const noticeClass = css({ color: 'text.hint' });
</script>

{#if loading}
  <p class={noticeClass}>대상을 확인하고 있어요</p>
{:else if !resolved}
  <p class={noticeClass}>대상을 찾지 못했어요</p>
{:else}
  <div class={listClass}>
    {#if hasUserGoal}
      <p class={itemClass}>일일 목표</p>
    {/if}
    {#each titles as title, index (ids[index])}
      <p class={itemClass}>「{title}」의 목표</p>
    {/each}
  </div>

  <p class={consequenceClass}>설정한 목표 글자 수가 사라져요.</p>
{/if}
