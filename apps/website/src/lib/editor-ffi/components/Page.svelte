<script lang="ts" module>
  import { css } from '@typie/styled-system/css';

  const canvasClass = css({ position: 'absolute', top: '0', left: '0', width: 'full', imageRendering: 'pixelated' });
</script>

<script lang="ts">
  import { CROP_MARKER_SIZE } from '../constants';
  import { getEditorContext } from '../editor.svelte';
  import { createSurfaceDriver } from '../surface-driver';
  import { probeAttach, probeDetach, probeEvent } from '../surface-probe';
  import { shouldKeepEmbedsWhileHidden, visibleExternalElements } from './external-element-visibility';
  import ExternalElement from './ExternalElement.svelte';
  import LinkOverlay from './LinkOverlay.svelte';
  import TableOverlay from './TableOverlay.svelte';
  import type { SurfaceDriverEffects } from '../surface-driver';

  type Props = {
    page: number;
    width: number;
    height: number;
    backingHeight: number;
  };

  let { page, width, height, backingHeight }: Props = $props();

  const ctx = getEditorContext();
  const { editor } = ctx;

  // Reactive mirror of `isVisible` used only by the overlay queries below, so
  // off-screen pages never build their fragments. Kept separate from the plain
  // `isVisible` so the imperative render effects are untouched.
  let overlaysVisible = $state(false);
  // Embed iframes (e.g. a playing YouTube video) lose their state when unmounted,
  // so pages holding embeds keep them mounted while scrolled off-screen.
  let keepEmbedsWhileHidden = $state(false);

  const scaleFactor = $derived(ctx.editor?.scaleFactor ?? 1);
  const cssWidth = $derived(Math.round(width * scaleFactor) / scaleFactor);
  const cssHeight = $derived(Math.round(height * scaleFactor) / scaleFactor);
  const cssBackingHeight = $derived(Math.round(backingHeight * scaleFactor) / scaleFactor);
  const layoutMode = $derived(ctx.editor?.rootAttrs?.layout_mode);
  const isPaginated = $derived(layoutMode?.type === 'paginated');
  const displayZoom = $derived(isPaginated ? (ctx.editor?.displayZoom ?? 1) : 1);
  const slotWidth = $derived(Math.round(width * displayZoom * scaleFactor) / scaleFactor);
  const slotHeight = $derived(Math.round(height * displayZoom * scaleFactor) / scaleFactor);
  const showCropMarker = $derived(layoutMode?.type === 'paginated' && !(ctx.editor?.readOnly ?? false));
  // Passive overlays must advance with the canvas, so these depend on publishedRevision.
  const externalElements = $derived.by(() => {
    void ctx.editor?.publishedRevision;
    const editor = ctx.editor;
    return editor ? visibleExternalElements(overlaysVisible, keepEmbedsWhileHidden, () => editor.pageExternalElements(page)) : [];
  });
  const tableOverlays = $derived.by(() => {
    void ctx.editor?.publishedRevision;
    return isPaginated && overlaysVisible && ctx.editor ? ctx.editor.pageTableOverlays(page) : [];
  });
  const linkRects = $derived.by(() => {
    void ctx.editor?.publishedRevision;
    return overlaysVisible && ctx.editor ? ctx.editor.pageLinkRects(page) : [];
  });
</script>

<div style:width={`${slotWidth}px`} style:height={`${slotHeight}px`} class={css({ position: 'relative', flexShrink: '0' })}>
  <div
    style:width={`${cssWidth}px`}
    style:height={`${cssHeight}px`}
    style:transform={isPaginated && displayZoom !== 1 ? `scale(${displayZoom})` : undefined}
    style:transform-origin={isPaginated && displayZoom !== 1 ? 'top left' : undefined}
    style:will-change={isPaginated && displayZoom !== 1 ? 'transform' : undefined}
    class={css({
      position: 'relative',
      isolation: 'isolate',
      ...(isPaginated && {
        backgroundColor: 'surface.default',
        boxShadow: '[0_2px_8px_rgba(0,0,0,0.1)]',
        ringWidth: '1px',
        ringColor: 'black/5',
      }),
    })}
    {@attach (el) => {
      if (!editor) {
        return;
      }

      editor.pageEls[page] = el;

      return () => {
        editor.pageEls[page] = undefined;
      };
    }}
  >
    <div
      class={css({ position: 'absolute', inset: '0', overflow: 'hidden' })}
      {@attach (wrapper) => {
        if (!editor) return;

        let driver: ReturnType<typeof createSurfaceDriver<HTMLCanvasElement>>;
        let isVisible = false;
        let syncSeeded = false;

        const effects: SurfaceDriverEffects<HTMLCanvasElement> = {
          createCanvas: () => {
            const canvas = document.createElement('canvas');
            canvas.className = canvasClass;
            canvas.dataset.pageCanvas = String(page);
            return canvas;
          },
          styleCanvas: (canvas) => {
            canvas.style.height = `${cssBackingHeight}px`;
          },
          attach: (canvas) => {
            const backend = editor.attachSurface(page, canvas, width, backingHeight, () => driver.replace());
            probeAttach(editor, page, canvas);
            if (backend === 'cpu') return 'cpu';
            return backend === 'cpu-oversized' ? 'cpu-oversized' : 'none';
          },
          detach: () => {
            probeDetach(editor, page);
            editor.detachSurface(page);
          },
          recover: () => editor.invalidateSurface(page),
          addContextListeners: (canvas, isCurrent) => {
            // 2D 캔버스도 GPU 리셋 시 백킹을 잃고 'contextrestored'를 발화한다 — 복귀 시
            // present state를 무효화하고 다시 그린다(CPU 유일 경로의 복구).
            const onContextRestored2d = () => {
              probeEvent(`contextrestored page=${page}`);
              if (isCurrent()) {
                editor.invalidateSurface(page);
              }
            };
            canvas.addEventListener('contextrestored', onContextRestored2d);
            return () => {
              canvas.removeEventListener('contextrestored', onContextRestored2d);
            };
          },
          releaseCpuBacking: (canvas) => {
            canvas.width = 0;
            canvas.height = 0;
          },
          promote: (next) => {
            if (next.parentNode !== wrapper) wrapper.append(next);
          },
          removeNode: (canvas) => {
            canvas.remove();
          },
          replacementFailed: () => editor.surfaceReplacementFailed(page),
        };

        driver = createSurfaceDriver(effects);
        const stopPublishedSync = $effect.root(() => {
          $effect(() => {
            driver.syncPublished(editor.publishedSurfaceCanvas(page));
          });
        });

        // A detached target has no editor-owned requirement. Visibility recovery
        // only recreates the target; attachSurface establishes its fresh requirement.
        const onVisible = () => {
          if (document.visibilityState === 'visible') driver.resume();
        };
        const onPageShow = () => driver.resume();
        document.addEventListener('visibilitychange', onVisible);
        window.addEventListener('pageshow', onPageShow);

        $effect(() => {
          const root = editor.scrollRootEl;
          if (root === undefined) return;

          let disposed = false;
          let observers: IntersectionObserver[] = [];
          let seeded = 0;
          const state = { inAcquire: false, inRelease: false };
          const measureVisibility = () => {
            const rect = wrapper.getBoundingClientRect();
            const rootRect = root === null ? { top: 0, bottom: window.innerHeight } : root.getBoundingClientRect();
            const viewportHeight = rootRect.bottom - rootRect.top;
            state.inAcquire = rect.bottom > rootRect.top - viewportHeight && rect.top < rootRect.bottom + viewportHeight;
            state.inRelease = rect.bottom > rootRect.top - 1.5 * viewportHeight && rect.top < rootRect.bottom + 1.5 * viewportHeight;
            return { rect, rootRect };
          };
          const updateDriverActive = () => {
            if (editor.terminal) return;
            const withinActiveRange = driver.hasSurface() ? state.inRelease : state.inAcquire;
            driver.setActive(editor.preparingPage === page || withinActiveRange);
          };
          const stopHostSync = $effect.root(() => {
            let wasPreparing = false;
            $effect(() => {
              if (editor.terminal) return;
              const preparing = editor.preparingPage === page;
              if (preparing === wasPreparing) return;
              wasPreparing = preparing;
              if (!preparing) {
                // Publication and preparation settle in the same Host turn. Promote
                // the exact published canvas before normal visibility may park it.
                driver.syncPublished(editor.publishedSurfaceCanvas(page));
                measureVisibility();
              }
              updateDriverActive();
            });
          });

          let buildEpoch = 0;
          const build = () => {
            if (disposed) return;
            const epoch = ++buildEpoch;
            for (const observer of observers) observer.disconnect();
            observers = [];
            seeded = 0;
            const h = root === null ? window.innerHeight : root.clientHeight;
            const mk = (margin: string, apply: (hit: boolean) => void, seed: boolean) => {
              let seededSelf = false;
              const observer = new IntersectionObserver(
                (entries) => {
                  if (disposed || epoch !== buildEpoch) return;
                  apply(entries.at(-1)?.isIntersecting ?? false);
                  if (seed && !seededSelf) {
                    seededSelf = true;
                    seeded += 1;
                  }
                  if (seeded >= 3 && !disposed) {
                    updateDriverActive();
                  }
                },
                { root, rootMargin: margin, threshold: 0 },
              );
              observer.observe(wrapper);
              observers.push(observer);
            };
            mk(
              '0px',
              (hit) => {
                isVisible = hit;
                if (overlaysVisible && !isVisible) {
                  keepEmbedsWhileHidden = shouldKeepEmbedsWhileHidden(externalElements);
                }
                overlaysVisible = isVisible;
              },
              true,
            );
            mk(`${Math.round(h)}px 0px`, (hit) => (state.inAcquire = hit), true);
            mk(`${Math.round(1.5 * h)}px 0px`, (hit) => (state.inRelease = hit), true);
          };

          build();
          // A freshly created page must not wait for the first IntersectionObserver
          // delivery (it lands after a paint): seed visibility synchronously once so a
          // page born under the caret presents in its first frame.
          if (!syncSeeded) {
            syncSeeded = true;
            const { rect, rootRect } = measureVisibility();
            if (state.inAcquire) {
              state.inRelease = true;
              isVisible = rect.bottom > rootRect.top && rect.top < rootRect.bottom;
              overlaysVisible = isVisible;
              updateDriverActive();
            }
          }
          let resize: ResizeObserver | null = null;
          if (root !== null) {
            resize = new ResizeObserver(() => build());
            resize.observe(root);
          }
          const rebuild = () => build();
          if (root === null) {
            window.addEventListener('resize', rebuild);
            window.visualViewport?.addEventListener('resize', rebuild);
          }

          return () => {
            disposed = true;
            stopHostSync();
            resize?.disconnect();
            if (root === null) {
              window.removeEventListener('resize', rebuild);
              window.visualViewport?.removeEventListener('resize', rebuild);
            }
            for (const observer of observers) observer.disconnect();
          };
        });

        $effect.pre(() => {
          if (editor.terminal) {
            driver.syncPublished(editor.publishedSurfaceCanvas(page));
            driver.freeze();
            return;
          }
          void editor.surfaceScaleFactor;
          void width;
          void backingHeight;
          if (!driver.isAttached()) {
            driver.replace();
            return;
          }
          if (editor.surfaceConfigMatches(page, width, backingHeight)) driver.restyle();
          else driver.replace();
        });

        return () => {
          document.removeEventListener('visibilitychange', onVisible);
          window.removeEventListener('pageshow', onPageShow);
          stopPublishedSync();
          driver.destroy();
        };
      }}
    ></div>

    {#each externalElements as element (element.node)}
      <ExternalElement {element} />
    {/each}

    {#each tableOverlays as overlay (`${overlay.table_id}-${overlay.page_idx}-${overlay.rows[0]?.index ?? 0}`)}
      <TableOverlay {overlay} readOnly={ctx.editor?.readOnly ?? false} />
    {/each}

    <LinkOverlay links={linkRects} />

    {#if showCropMarker && layoutMode?.type === 'paginated'}
      {@const marginLeft = layoutMode.page_margin_left}
      {@const marginRight = layoutMode.page_margin_right}
      {@const marginTop = layoutMode.page_margin_top}
      {@const marginBottom = layoutMode.page_margin_bottom}
      <svg
        class={css({
          pointerEvents: 'none',
          position: 'absolute',
          inset: '0',
          height: 'full',
          width: 'full',
          overflow: 'visible',
          color: 'text.default',
          opacity: '15',
        })}
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d={`M ${marginLeft} ${marginTop - CROP_MARKER_SIZE} L ${marginLeft} ${marginTop} L ${marginLeft - CROP_MARKER_SIZE} ${marginTop} M ${width - marginRight} ${marginTop - CROP_MARKER_SIZE} L ${width - marginRight} ${marginTop} L ${width - marginRight + CROP_MARKER_SIZE} ${marginTop} M ${marginLeft} ${height - marginBottom + CROP_MARKER_SIZE} L ${marginLeft} ${height - marginBottom} L ${marginLeft - CROP_MARKER_SIZE} ${height - marginBottom} M ${width - marginRight} ${height - marginBottom + CROP_MARKER_SIZE} L ${width - marginRight} ${height - marginBottom} L ${width - marginRight + CROP_MARKER_SIZE} ${height - marginBottom}`}
          fill="none"
          stroke="currentColor"
        />
      </svg>
    {/if}
  </div>
</div>
