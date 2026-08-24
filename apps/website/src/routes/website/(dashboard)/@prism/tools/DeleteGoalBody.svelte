<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { z } from 'zod';
  import { graphql } from '$mearie';
  import { consequenceClass } from './action-cards.ts';
  import type { ActionBodyProps } from './action-cards.ts';

  let { input, onReady }: ActionBodyProps = $props();

  const TargetSchema = z.object({ id: z.string().min(1).optional() });

  const parsed = $derived(TargetSchema.safeParse(input));
  const id = $derived(parsed.success ? (parsed.data.id ?? '') : '');
  const scoped = $derived(id !== '');

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
          }
        }
      }
    `),
    () => ({ ids: [id] }),
    () => ({ skip: !scoped }),
  );

  const node = $derived(query.data?.prismEntities[0]?.node);
  const title = $derived(node === undefined ? undefined : node.__typename === 'Folder' ? node.name : node.title);

  const loading = $derived(scoped && query.loading);
  const resolved = $derived(parsed.success && (!scoped || (!loading && query.error === undefined && title !== undefined)));

  $effect(() => {
    onReady(resolved);
  });

  const itemClass = css({ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', lineHeight: '[1.5]' });
  const noticeClass = css({ color: 'text.faint' });
</script>

{#if loading}
  <p class={noticeClass}>대상을 확인하고 있어요</p>
{:else if !resolved}
  <p class={noticeClass}>대상을 찾지 못했어요</p>
{:else if scoped}
  <p class={itemClass}>「{title}」의 목표</p>
  <p class={consequenceClass}>설정한 목표 글자 수가 사라져요.</p>
{:else}
  <p class={itemClass}>하루 목표</p>
  <p class={consequenceClass}>설정한 하루 목표 글자 수가 사라져요.</p>
{/if}
