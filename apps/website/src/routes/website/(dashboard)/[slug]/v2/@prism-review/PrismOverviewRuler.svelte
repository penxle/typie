<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { portal } from '@typie/ui/actions';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { getMarginContext } from './context.svelte.ts';
  import type { RailTone } from './rail-layout.ts';

  type Props = { marks: { itemId: string; ratio: number; tone: RailTone }[] };
  let { marks }: Props = $props();

  const ctx = getEditorContext();
  const margin = getMarginContext();

  // Scrollbar.svelte의 트랙 크기 — 룰러는 세로 트랙 안쪽에 서고, 가로 트랙이 뜨면 세로 트랙과 같은 만큼 짧아진다
  const SCROLLBAR_TRACK = 12;

  let rect = $state<DOMRect | null>(null);
  let bottomInset = $state(0);
  $effect(() => {
    // 스크롤 컨테이너는 스크롤바를 숨긴 overflow:auto라(View.svelte) 가로로 넘쳐도 제 박스가 그대로다 —
    // ResizeObserver가 못 보는 그 전환을, 가로 넘침을 정하는 상태를 읽어 붙잡는다
    void ctx.editor?.displayZoom;
    void ctx.editor?.pageSizes;

    const el = ctx.editor?.scrollContainerEl;
    if (!el) {
      rect = null;
      return;
    }
    const sync = () => {
      rect = el.getBoundingClientRect();
      bottomInset = el.scrollWidth > el.clientWidth ? SCROLLBAR_TRACK : 0;
    };
    sync();
    const observer = new ResizeObserver(sync);
    observer.observe(el);
    return () => observer.disconnect();
  });

  const markRecipe = cva({
    base: {
      position: 'absolute',
      right: '0',
      height: '3px',
      borderRadius: '2px',
      cursor: 'pointer',
      pointerEvents: 'auto',
      transition: '[width 0.25s cubic-bezier(0.2, 0, 0, 1), opacity 0.25s cubic-bezier(0.2, 0, 0, 1)]',
      _hover: { width: '10px', opacity: '100' },
    },
    variants: {
      tone: {
        open: { backgroundColor: 'accent.pink.default' },
        closed: { backgroundColor: 'border.strong' },
        strength: { backgroundColor: 'accent.emerald.default' },
      },
      active: { true: { width: '10px', opacity: '100' }, false: { width: '6px', opacity: '55' } },
    },
  });

  const labelOf = (itemId: string) => {
    const item = margin.items.find((candidate) => candidate.id === itemId);
    if (!item) return '';
    // 레일 막대와 룰러 마크는 같은 항목을 켜는 쌍둥이다 — 화면을 못 보는 사용자에게 같은 이름으로 읽혀야 한다
    if (item.kind === 'strength') return `읽는 사람에게 잘 닿은 대목 ${item.number}`;
    return `피드백 ${item.number}: ${item.thread?.trait ?? '피드백'}`;
  };
</script>

<!-- 에디터 스크롤바는 1초 뒤 사라지므로 룰러는 제 트랙을 갖는다. 트랙은 클릭을 통과시키고 마크만 받는다. -->
{#if rect && marks.length > 0}
  <div
    style:top={`${rect.top}px`}
    style:right={`${window.innerWidth - rect.right + SCROLLBAR_TRACK}px`}
    style:height={`${rect.height - bottomInset}px`}
    class={css({ position: 'fixed', zIndex: '10', width: '10px', pointerEvents: 'none' })}
    use:portal
  >
    {#each marks as mark (mark.itemId)}
      <button
        style:top={`${mark.ratio * 100}%`}
        class={css(markRecipe.raw({ tone: mark.tone, active: margin.activeId === mark.itemId }))}
        aria-label={labelOf(mark.itemId)}
        data-prism-margin-activator
        onclick={() => margin.activate(mark.itemId, 'jump')}
        title={labelOf(mark.itemId)}
        type="button"
      ></button>
    {/each}
  </div>
{/if}
