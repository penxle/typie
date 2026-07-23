<script lang="ts" module>
  import { css } from '@typie/styled-system/css';

  const canvasClass = css({ position: 'absolute', top: '0', left: '0', width: 'full', imageRendering: 'pixelated' });
</script>

<script lang="ts">
  import { CROP_MARKER_SIZE } from '../constants';
  import { getEditorContext } from '../editor.svelte';
  import { createPageSurfaceManager } from '../page-surface-manager';
  import { probeAttach, probeDetach, probeEvent } from '../surface-probe';
  import { shouldKeepEmbedsWhileHidden, visibleExternalElements } from './external-element-visibility';
  import ExternalElement from './ExternalElement.svelte';
  import LinkOverlay from './LinkOverlay.svelte';
  import TableOverlay from './TableOverlay.svelte';
  import type { ManagerEffects } from '../page-surface-manager';

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
  // Per-visible-page queries: only on-screen pages build their fragment, turning
  // the old whole-document O(pages · N) recompute (every keystroke) into O(N) for
  // the few visible pages.
  const externalElements = $derived.by(() => {
    void ctx.editor?.tickRevision;
    const editor = ctx.editor;
    return editor ? visibleExternalElements(overlaysVisible, keepEmbedsWhileHidden, () => editor.pageExternalElements(page)) : [];
  });
  const tableOverlays = $derived.by(() => {
    void ctx.editor?.tickRevision;
    return isPaginated && overlaysVisible && ctx.editor ? ctx.editor.pageTableOverlays(page) : [];
  });
  const linkRects = $derived.by(() => {
    void ctx.editor?.tickRevision;
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

        let manager: ReturnType<typeof createPageSurfaceManager<HTMLCanvasElement>>;
        let isVisible = false;
        let dirty = false;
        let needsResize = false;
        let syncSeeded = false;

        const paint = () => {
          if (manager.isAttached()) {
            editor.requestSurfaceRender(page);
            dirty = false;
          } else {
            dirty = true;
          }
        };

        // Flushes dirty/needsResize left over from edits while unattached, once (re)attached.
        const flushIfAttached = () => {
          if (!manager.isAttached()) return;
          if (needsResize) {
            editor.resizeSurfaceNow(page, width, backingHeight);
            needsResize = false;
            dirty = false;
          }
          if (dirty) {
            editor.requestSurfaceRender(page);
            dirty = false;
          }
        };

        const effects: ManagerEffects<HTMLCanvasElement> = {
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
            editor.attachSurface(page, canvas, width, backingHeight);
            probeAttach(editor, page, canvas);
            return editor.surfaceBackend(page) === 'cpu-oversized' ? 'cpu-oversized' : 'cpu';
          },
          detach: () => {
            probeDetach(editor, page);
            editor.detachSurface(page);
          },
          requestRender: () => editor.requestSurfaceRender(page),
          isSuspended: () => document.visibilityState !== 'visible',
          onPresented: (listener) => editor.onSurfacePresented(page, listener),
          addContextListeners: (canvas, isCurrent) => {
            // 2D 캔버스도 GPU 리셋 시 백킹을 잃고 'contextrestored'를 발화한다 — 복귀 시
            // present state를 무효화하고 다시 그린다(CPU 유일 경로의 복구).
            const onContextRestored2d = () => {
              probeEvent(`contextrestored page=${page}`);
              if (isCurrent()) {
                editor.invalidateSurface(page);
                paint();
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
            wrapper.append(next);
          },
          removeNode: (canvas) => {
            canvas.remove();
          },
          schedule: (fn, ms) => {
            const id = setTimeout(fn, ms);
            return () => clearTimeout(id);
          },
        };

        manager = createPageSurfaceManager(effects);

        // 가시성 복귀 시 resume: 숨김 중 로스로 detach된 표면(failedParked)을 치유하고, 정체된
        // pending은 렌더를 재촉한다. 에디터 레벨 recoverSurfaces는 attached 표면만 갱신하므로
        // detached 표면 복구는 이 손이 담당한다(둘은 독립적).
        const onVisible = () => {
          if (document.visibilityState === 'visible') manager.resume();
        };
        const onPageShow = () => manager.resume();
        document.addEventListener('visibilitychange', onVisible);
        window.addEventListener('pageshow', onPageShow);

        const offRender = editor.on('render_invalidated', paint);

        $effect(() => {
          const root = editor.scrollRootEl;
          if (root === undefined) return;

          let disposed = false;
          let observers: IntersectionObserver[] = [];
          let seeded = 0;
          const state = { inAcquire: false, inRelease: false, isVisible: false };

          let buildEpoch = 0;
          const build = () => {
            const epoch = ++buildEpoch;
            for (const observer of observers) observer.disconnect();
            observers = [];
            seeded = 0;
            const h = root === null ? window.innerHeight : root.clientHeight;
            const mk = (margin: string, apply: (hit: boolean) => void, seed: boolean) => {
              let seededSelf = false;
              const observer = new IntersectionObserver(
                (entries) => {
                  if (epoch !== buildEpoch) return;
                  apply(entries.at(-1)?.isIntersecting ?? false);
                  if (seed && !seededSelf) {
                    seededSelf = true;
                    seeded += 1;
                  }
                  if (seeded >= 3 && !disposed) {
                    manager.reconcile({ ...state });
                    flushIfAttached();
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
                state.isVisible = hit;
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
            const rect = wrapper.getBoundingClientRect();
            const rootRect = root === null ? { top: 0, bottom: window.innerHeight } : root.getBoundingClientRect();
            const viewportHeight = rootRect.bottom - rootRect.top;
            const inAcquire = rect.bottom > rootRect.top - viewportHeight && rect.top < rootRect.bottom + viewportHeight;
            if (inAcquire) {
              state.inAcquire = true;
              state.inRelease = true;
              state.isVisible = rect.bottom > rootRect.top && rect.top < rootRect.bottom;
              isVisible = state.isVisible;
              overlaysVisible = state.isVisible;
              manager.reconcile({ ...state });
              flushIfAttached();
              editor.flush();
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
            resize?.disconnect();
            if (root === null) {
              window.removeEventListener('resize', rebuild);
              window.visualViewport?.removeEventListener('resize', rebuild);
            }
            for (const observer of observers) observer.disconnect();
          };
        });

        $effect.pre(() => {
          void editor.surfaceScaleFactor;
          void width;
          void backingHeight;
          manager.restyle();
          if (manager.isAttached()) {
            editor.resizeSurfaceNow(page, width, backingHeight);
            dirty = false;
            needsResize = false;
          } else {
            needsResize = true;
            dirty = true;
          }
        });

        return () => {
          document.removeEventListener('visibilitychange', onVisible);
          window.removeEventListener('pageshow', onPageShow);
          offRender();
          manager.destroy();
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
