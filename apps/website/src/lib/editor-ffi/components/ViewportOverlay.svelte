<script lang="ts" module>
  import { createStableContext } from '@typie/ui/context/stable';
  import type { Editor } from '../editor.svelte';

  class ViewportOverlayContext {
    #frame: number | null = null;
    #editor: Editor | undefined;
    #change = $state(0);

    requestSync = () => {
      if (this.#frame !== null) return;
      this.#frame = requestAnimationFrame(() => {
        this.#frame = null;
        this.#change += 1;
      });
    };

    constructor(editor: Editor | undefined) {
      this.#editor = editor;
    }

    // Presentation updates are already frame-coalesced, so consumers observe them directly.
    get change(): number {
      void this.#editor?.presentationGeometryRevision;
      return this.#change;
    }

    destroy(): void {
      if (this.#frame === null) return;
      cancelAnimationFrame(this.#frame);
      this.#frame = null;
    }
  }

  const [getViewportOverlayContext, setViewportOverlayContext] =
    createStableContext<ViewportOverlayContext>('editor-ffi.ViewportOverlayContext');

  export { getViewportOverlayContext };
</script>

<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';
  import type { Snippet } from 'svelte';

  type Props = {
    children?: Snippet;
  };

  let { children }: Props = $props();

  const { editor } = getEditorContext();
  const overlay = setViewportOverlayContext(new ViewportOverlayContext(editor));

  $effect(() => {
    if (!editor) return;

    const scrollContainer = editor.scrollContainerEl;
    const extensionAreaEl = editor.extensionAreaEl;
    const visualViewport = window.visualViewport;
    const passive = { passive: true } as AddEventListenerOptions;
    const passiveCapture = { passive: true, capture: true } as AddEventListenerOptions;
    const resizeObserver = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(overlay.requestSync);

    scrollContainer?.addEventListener('scroll', overlay.requestSync, passive);
    window.addEventListener('scroll', overlay.requestSync, passiveCapture);
    window.addEventListener('resize', overlay.requestSync, passive);
    visualViewport?.addEventListener('scroll', overlay.requestSync, passive);
    visualViewport?.addEventListener('resize', overlay.requestSync, passive);
    if (scrollContainer) resizeObserver?.observe(scrollContainer);
    if (extensionAreaEl) resizeObserver?.observe(extensionAreaEl);

    overlay.requestSync();

    return () => {
      scrollContainer?.removeEventListener('scroll', overlay.requestSync, passive);
      window.removeEventListener('scroll', overlay.requestSync, passiveCapture);
      window.removeEventListener('resize', overlay.requestSync, passive);
      visualViewport?.removeEventListener('scroll', overlay.requestSync, passive);
      visualViewport?.removeEventListener('resize', overlay.requestSync, passive);
      resizeObserver?.disconnect();
      overlay.destroy();
    };
  });
</script>

<div class={css({ display: 'contents' })}>
  {@render children?.()}
</div>
