<script lang="ts">
  import { token } from '@typie/styled-system/tokens';
  import { untrack } from 'svelte';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { getMarginContext } from './context.svelte.ts';
  import PrismReviewHighlightRect from './PrismReviewHighlightRect.svelte';
  import type { PageRect } from '@typie/editor-ffi/browser';

  type HighlightRect = { rangeId: string; fragmentIndex: number; pageRect: PageRect };

  const ctx = getEditorContext();
  const editor = $derived(ctx.editor);
  const margin = getMarginContext();
  const highlight = $derived.by<{ kind: 'issue' | 'strength' | null; rects: HighlightRect[] }>(() => {
    const activeId = margin.activeId;
    const published = editor?.published;
    if (activeId === null || !editor || !published) return { kind: null, rects: [] };

    // items는 applied snapshot에도 반응하지만 rect는 published canvas와만 함께 바뀌어야 한다.
    const active = untrack(() => margin.items.find((item) => item.id === activeId));
    if (!active) return { kind: null, rects: [] };

    const rects = active.rangeIds
      .flatMap((rangeId) =>
        (editor.trackedRangeForSnapshot(rangeId, published.snapshot)?.rects ?? []).map((pageRect, fragmentIndex) => ({
          rangeId,
          fragmentIndex,
          pageRect,
        })),
      )
      .filter(({ pageRect }) => published.frames.has(pageRect.page_idx));

    return {
      kind: active.kind,
      rects,
    };
  });
  const edge = $derived(token(highlight.kind === 'strength' ? 'colors.review.strength.default' : 'colors.review.issue.default'));
  const fill = $derived(token(highlight.kind === 'strength' ? 'colors.review.strength.highlight' : 'colors.review.issue.highlight'));
</script>

{#if editor}
  {#each highlight.rects as rect (`${margin.presentationRoundId}:${rect.rangeId}:${rect.fragmentIndex}`)}
    <PrismReviewHighlightRect {edge} {editor} {fill} kind={highlight.kind} rect={rect.pageRect} />
  {/each}
{/if}
