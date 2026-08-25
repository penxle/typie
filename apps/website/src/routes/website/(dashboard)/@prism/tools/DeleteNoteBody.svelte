<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { z } from 'zod';
  import { graphql } from '$mearie';
  import { consequenceClass } from './action-cards.ts';
  import type { ActionBodyProps } from './action-cards.ts';

  let { input, onReady }: ActionBodyProps = $props();

  const TargetsSchema = z.object({ noteIds: z.array(z.string().min(1)).min(1) });

  const parsed = $derived(TargetsSchema.safeParse(input));
  const noteIds = $derived(parsed.success ? [...new Set(parsed.data.noteIds)] : []);

  const query = createQuery(
    graphql(`
      query DashboardLayout_PrismDeleteNoteBody_Query($ids: [ID!]!) {
        prismNotes(ids: $ids) {
          id
          content
        }
      }
    `),
    () => ({ ids: noteIds }),
    () => ({ skip: noteIds.length === 0 }),
  );

  const found = $derived(query.data?.prismNotes ?? []);
  const previews = $derived(
    noteIds.map((noteId) => {
      const note = found.find((candidate) => candidate.id === noteId);
      if (note === undefined) return null;
      return (note.content.split('\n', 1)[0] ?? '').trim();
    }),
  );

  const loading = $derived(noteIds.length > 0 && query.loading);
  const resolved = $derived(noteIds.length > 0 && !loading && query.error === undefined && previews.every((preview) => preview !== null));

  $effect(() => {
    onReady(resolved);
  });

  const listClass = flex({ flexDirection: 'column', gap: '4px' });
  const itemClass = css({ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', lineHeight: '[1.5]' });
  const noticeClass = css({ color: 'text.faint' });
</script>

{#if loading}
  <p class={noticeClass}>노트를 확인하고 있어요</p>
{:else if !resolved}
  <p class={noticeClass}>노트를 찾지 못했어요</p>
{:else}
  <div class={listClass}>
    {#each previews as preview, index (noteIds[index])}
      {#if preview === ''}
        <p class={itemClass}>내용이 없는 노트</p>
      {:else}
        <p class={itemClass}>노트 「{preview}」</p>
      {/if}
    {/each}
  </div>

  <p class={consequenceClass}>지운 노트는 되돌릴 수 없어요.</p>
{/if}
