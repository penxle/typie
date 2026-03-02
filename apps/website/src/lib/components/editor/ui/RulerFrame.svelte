<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { PAGE_GAP } from '$lib/editor/constants';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import HorizontalRuler from './HorizontalRuler.svelte';
  import VerticalRuler from './VerticalRuler.svelte';

  type Props = {
    thickness?: number;
    unit?: 'px' | 'cm';
    pagePadding?: number;
    headerPadding?: number;
    scrollLeft?: number;
    scrollTop?: number;
  };

  let { thickness = 24, unit = 'px', pagePadding = 0, headerPadding = 0, scrollLeft = 0, scrollTop = 0 }: Props = $props();

  const { editor } = getEditorContext();

  const layoutMode = $derived(editor.layout?.layoutMode);
  const pages = $derived(editor.layout?.pages ?? []);
  const pageWidth = $derived(pages[0]?.width ?? 0);
  const marginTop = $derived(layoutMode?.type === 'paginated' ? layoutMode.pageMarginTop : 0);
  const marginBottom = $derived(layoutMode?.type === 'paginated' ? layoutMode.pageMarginBottom : 0);
  const marginLeft = $derived(layoutMode?.type === 'paginated' ? layoutMode.pageMarginLeft : 0);
  const marginRight = $derived(layoutMode?.type === 'paginated' ? layoutMode.pageMarginRight : 0);
  const zoom = $derived(layoutMode?.type === 'paginated' ? editor.displayZoom : 1);
  const pageGap = $derived(layoutMode?.type === 'paginated' ? PAGE_GAP : 0);
</script>

<div
  class={css({
    borderRightWidth: '1px',
    borderBottomWidth: '1px',
    borderColor: 'border.strong',
    backgroundColor: 'surface.default',
  })}
></div>

<div class={css({ overflow: 'hidden' })}>
  {#if pageWidth}
    <HorizontalRuler {marginLeft} {marginRight} offsetX={scrollLeft} padding={pagePadding} {pageWidth} {thickness} {unit} {zoom} />
  {/if}
</div>

<div class={css({ overflow: 'hidden' })}>
  {#if pages.length > 0}
    <VerticalRuler {marginBottom} {marginTop} offsetY={scrollTop} padding={headerPadding} {pageGap} {pages} {thickness} {unit} {zoom} />
  {/if}
</div>
