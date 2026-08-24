<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { z } from 'zod';
  import { graphql } from '$mearie';
  import { consequenceClass } from './action-cards.ts';
  import type { ActionBodyProps } from './action-cards.ts';

  let { input, onReady }: ActionBodyProps = $props();

  const TargetSchema = z.object({ noteId: z.string().min(1) });

  const parsed = $derived(TargetSchema.safeParse(input));
  const noteId = $derived(parsed.success ? parsed.data.noteId : '');

  const query = createQuery(
    graphql(`
      query DashboardLayout_PrismDeleteNoteBody_Query($noteId: ID!) {
        note(noteId: $noteId) {
          id
          content
        }
      }
    `),
    () => ({ noteId }),
    () => ({ skip: noteId === '' }),
  );

  const note = $derived(query.data?.note);
  const loading = $derived(noteId !== '' && query.loading);
  const resolved = $derived(noteId !== '' && !loading && query.error === undefined && note !== undefined);
  const preview = $derived((note?.content.split('\n', 1)[0] ?? '').trim());

  $effect(() => {
    onReady(resolved);
  });

  const itemClass = css({ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', lineHeight: '[1.5]' });
  const noticeClass = css({ color: 'text.faint' });
</script>

{#if loading}
  <p class={noticeClass}>노트를 확인하고 있어요</p>
{:else if !resolved}
  <p class={noticeClass}>노트를 찾지 못했어요</p>
{:else}
  {#if preview === ''}
    <p class={itemClass}>내용이 없는 노트예요</p>
  {:else}
    <p class={itemClass}>노트 「{preview}」</p>
  {/if}

  <p class={consequenceClass}>지운 노트는 되돌릴 수 없어요.</p>
{/if}
