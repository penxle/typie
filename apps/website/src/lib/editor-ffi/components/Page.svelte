<script lang="ts" module>
  import { css } from '@typie/styled-system/css';
  import { GlCanvasRecycler } from '../gl-canvas-recycler';
  import { GL_POOL_BUDGET, glContextPool, setBackendChangeHandler } from '../gl-context-pool';
  import { createTrailingDebounce, IO_REBUILD_DEBOUNCE_MS, shouldRebuildForResize } from '../io-rebuild';
  import { statsEnabled, surfaceStats } from '../surface-stats';
  import type { LeaseToken, SurfaceBackend } from '../gl-context-pool';

  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  const poolRoutes = new Set<(editorKey: object, page: number, backend: SurfaceBackend, acquireHint?: LeaseToken) => void>();
  setBackendChangeHandler((editorKey, page, backend, acquireHint) => {
    for (const route of poolRoutes) route(editorKey, page, backend, acquireHint);
  });

  // 기본값 = GL. 'cpu'만 강제 CPU(killswitch).
  const killSwitch = typeof localStorage !== 'undefined' && localStorage.getItem('typie:page-surface') === 'cpu';

  // 후보 수정 플래그(기본 미설정 = 현행 동작 그대로). 'io'=관찰 연속성, 'recycle'=GL 캔버스 재사용.
  const surfaceFix = typeof localStorage === 'undefined' ? '' : (localStorage.getItem('typie:surface-fix') ?? '');
  const surfaceFixSet = new Set(
    surfaceFix
      .split(',')
      .map((token) => token.trim())
      .filter(Boolean),
  );
  const ioFix = surfaceFixSet.has('io');
  const recycleFix = surfaceFixSet.has('recycle');

  const canvasClass = css({ position: 'absolute', top: '0', left: '0', width: 'full', imageRendering: 'pixelated' });

  // 'recycle': 파킹된 GL 캔버스(고정 webgl2 컨텍스트 유지)를 GL_POOL_BUDGET개까지 담는 모듈 전역 LRU.
  // 파킹 중 force-loss를 감지해 즉시 축출하는 리스너를 붙이고, 재사용/축출 시 그 리스너를 뗀다.
  const pooledLostCleanup = new WeakMap<HTMLCanvasElement, () => void>();

  const detachPooledLostListener = (canvas: HTMLCanvasElement): void => {
    const off = pooledLostCleanup.get(canvas);
    off?.();
    pooledLostCleanup.delete(canvas);
  };

  const glCanvasRecycler = new GlCanvasRecycler<HTMLCanvasElement>(GL_POOL_BUDGET, {
    isLost: (canvas) => canvas.getContext('webgl2')?.isContextLost() ?? true,
    dispose: (canvas) => {
      // 오버플로 축출: pooled 리스너를 먼저 떼고(loseContext 재진입 축출 방지) 컨텍스트를 처분한다.
      detachPooledLostListener(canvas);
      const gl = canvas.getContext('webgl2');
      const wasLost = !gl || gl.isContextLost();
      if (gl && !wasLost) gl.getExtension('WEBGL_lose_context')?.loseContext();
      surfaceStats.glDispose(canvas, wasLost);
      surfaceStats.recycleEvict();
      canvas.remove();
    },
  });

  // 활성 Page 인스턴스 수(모든 에디터 합). 0이 되면(마지막 에디터의 View unmount) 재활용 풀을
  // flush해 파킹된 live GL 컨텍스트가 탭 수명 내내 남지 않게 한다. 여러 에디터가 공존하면 한
  // 에디터가 사라져도 남은 에디터가 파킹 캔버스를 재사용할 수 있으므로 마지막 0에서만 flush한다.
  let activePageCount = 0;

  // attach가 실제 webgl2 컨텍스트를 잡은 캔버스 — promote 시 삽입 후 re-blit 대상 판별용.
  const glLiveCanvases = new WeakSet<HTMLCanvasElement>();

  const parkGlCanvas = (canvas: HTMLCanvasElement): void => {
    const onLostWhilePooled = () => {
      detachPooledLostListener(canvas);
      glCanvasRecycler.drop(canvas);
      surfaceStats.glDispose(canvas, true);
      surfaceStats.recycleEvict();
      canvas.remove();
    };
    canvas.addEventListener('webglcontextlost', onLostWhilePooled);
    pooledLostCleanup.set(canvas, () => canvas.removeEventListener('webglcontextlost', onLostWhilePooled));
    glCanvasRecycler.park(canvas);
  };
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
  import type { ManagerEffects, PoolPort } from '../page-surface-manager';

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
    return overlaysVisible && ctx.editor ? ctx.editor.pageTableOverlays(page) : [];
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

        activePageCount += 1;

        let manager: ReturnType<typeof createPageSurfaceManager<HTMLCanvasElement>>;
        let isVisible = false;
        let dirty = false;
        let needsResize = false;

        const scheduleDeadCheck = () => {
          requestAnimationFrame(() => {
            if (editor.surfaceBackend(page) === 'gl-dead') manager.remountFromLoss();
          });
        };

        const paint = () => {
          if (manager.isAttached()) {
            editor.requestSurfaceRender(page);
            dirty = false;
            scheduleDeadCheck();
          } else {
            dirty = true;
          }
        };

        // Flushes dirty/needsResize left over from edits while unattached, once (re)attached.
        const flushIfAttached = () => {
          if (!manager.isAttached()) return;
          if (needsResize) {
            editor.requestSurfaceResize(page, width, backingHeight);
            needsResize = false;
            dirty = false;
          }
          if (dirty) {
            editor.requestSurfaceRender(page);
            dirty = false;
          }
        };

        // eslint-disable-next-line @typescript-eslint/no-empty-function -- shared no-op for the killswitch's cpu-only stub pool
        const noop = () => {};
        const pool: PoolPort = killSwitch
          ? {
              updateDemand: () => 'cpu',
              acquireLease: () => ({ backend: 'cpu' }),
              ackAttached: noop,
              cancelReservation: noop,
              beginRelease: noop,
              ackReleased: noop,
              notePresent: noop,
              noteGlFailure: noop,
              noteBudgetFallback: noop,
              backendOf: () => 'cpu',
              leave: noop,
              forget: noop,
            }
          : {
              updateDemand: (zone) => glContextPool.updatePageDemand(editor, page, zone),
              acquireLease: (requested) => {
                const lease = glContextPool.acquireCanvasLease(editor, page, requested);
                if (requested === 'gl') {
                  if (lease.backend === 'gl') surfaceStats.acquireGl();
                  else surfaceStats.acquireCpuFallback();
                }
                return lease;
              },
              ackAttached: (token, actual) => glContextPool.ackAttached(token, actual),
              cancelReservation: (token, reason) => glContextPool.cancelReservation(token, reason),
              beginRelease: (token) => glContextPool.beginRelease(token),
              ackReleased: (token) => glContextPool.ackReleased(token),
              notePresent: (token) => glContextPool.notePresent(editor, page, token),
              noteGlFailure: (incident) => glContextPool.noteGlFailure(editor, page, incident),
              noteBudgetFallback: () => glContextPool.noteBudgetFallback(editor, page),
              backendOf: () => glContextPool.backendOf(editor, page),
              leave: () => glContextPool.leave(editor, page),
              forget: () => glContextPool.forget(editor, page),
            };

        const effects: ManagerEffects<HTMLCanvasElement> = {
          createCanvas: (backend) => {
            // 'recycle': GL 경로면 LRU에서 재사용 캔버스(고정 컨텍스트 유지)를 꺼낸다 — attach가
            // 새 크기로 리사이즈하므로 크기 불문. 없으면 새로 만든다.
            if (recycleFix && backend === 'gl') {
              const reused = glCanvasRecycler.acquire();
              if (reused) {
                detachPooledLostListener(reused);
                reused.className = canvasClass;
                reused.dataset.pageCanvas = String(page);
                surfaceStats.recycleHit();
                return reused;
              }
              surfaceStats.recycleMiss();
            }
            const canvas = document.createElement('canvas');
            canvas.className = canvasClass;
            canvas.dataset.pageCanvas = String(page);
            return canvas;
          },
          styleCanvas: (canvas) => {
            canvas.style.height = `${cssBackingHeight}px`;
          },
          attach: (canvas, backend) => {
            const actual = editor.attachSurfaceWithBackend(page, canvas, width, backingHeight, backend);
            probeAttach(editor, page, canvas);
            const polled = editor.surfaceBackend(page);
            const resolved = polled === 'gl-dead' || polled === 'cpu-oversized' ? polled : actual;
            // gl 요청이 실제 webgl2 컨텍스트를 잡았을 때만 create 계상(canvas 신원 dedup으로 재활용 재-attach 제외).
            if (backend === 'gl' && (resolved === 'gl' || resolved === 'gl-dead')) surfaceStats.glCreate(canvas);
            if (resolved === 'gl') glLiveCanvases.add(canvas);
            return resolved;
          },
          detach: () => {
            probeDetach(editor, page);
            editor.detachSurface(page);
          },
          requestRender: () => editor.requestSurfaceRender(page),
          isSuspended: () => document.visibilityState !== 'visible',
          onPresented: (listener) => editor.onSurfacePresented(page, listener),
          addContextListeners: (canvas, isCurrent) => {
            const onWebglContextLost = (event: Event) => {
              event.preventDefault();
              probeEvent(`webglcontextlost page=${page}`);
              // 우리가 처분한 캔버스는 dispose 전에 이 리스너가 제거되므로, 여기 도달하는 로스는
              // 전부 우리가 유발하지 않은 것이다(force-loss 감지).
              surfaceStats.unexpectedLost(page);
              if (isCurrent()) manager.onContextLost();
            };
            const onWebglContextRestored = () => {
              probeEvent(`webglcontextrestored page=${page}`);
              if (isCurrent()) manager.onContextRestored();
            };
            const onContextRestored2d = () => {
              probeEvent(`contextrestored page=${page}`);
              if (isCurrent()) {
                editor.invalidateSurface(page);
                paint();
              }
            };
            canvas.addEventListener('webglcontextlost', onWebglContextLost);
            canvas.addEventListener('webglcontextrestored', onWebglContextRestored);
            canvas.addEventListener('contextrestored', onContextRestored2d);
            return () => {
              canvas.removeEventListener('webglcontextlost', onWebglContextLost);
              canvas.removeEventListener('webglcontextrestored', onWebglContextRestored);
              canvas.removeEventListener('contextrestored', onContextRestored2d);
            };
          },
          disposeGlContext: (canvas) => {
            const gl = canvas.getContext('webgl2');
            const wasLost = !gl || gl.isContextLost();
            // 'recycle': 아직 살아있는 컨텍스트는 loseContext 대신 LRU에 파킹한다(고정 컨텍스트 유지).
            // 뒤이어 매니저의 removeNode가 DOM에서 뗀다. 로스된 컨텍스트는 파킹하지 않는다.
            if (recycleFix && !wasLost) {
              parkGlCanvas(canvas);
              return;
            }
            // 기본: 이미 로스된 컨텍스트에 loseContext를 또 호출하면 INVALID_OPERATION 스팸이 난다
            // (force-loss storm 중 특히 심함) — 살아있을 때만 명시적으로 해제한다.
            if (gl && !wasLost) gl.getExtension('WEBGL_lose_context')?.loseContext();
            surfaceStats.glDispose(canvas, wasLost);
          },
          releaseCpuBacking: (canvas) => {
            canvas.width = 0;
            canvas.height = 0;
          },
          promote: (next) => {
            wrapper.append(next);
            // GL 캔버스의 present(finishSwap 커밋)는 DOM 삽입 전에 일어난다. iOS WebKit은 삽입 후
            // 새 present가 없으면 compositor가 잡을 프레임이 없어 빈 화면이 될 수 있다(특히 재삽입
            // 캔버스). 삽입 뒤 레이어가 생성될 시간을 두고 re-blit 1회로 표시를 보장한다.
            if (glLiveCanvases.has(next)) {
              requestAnimationFrame(() => {
                requestAnimationFrame(() => {
                  if (next.isConnected) editor.refreshSurface(page);
                });
              });
            }
          },
          removeNode: (canvas) => {
            canvas.remove();
          },
          schedule: (fn, ms) => {
            const id = setTimeout(fn, ms);
            return () => clearTimeout(id);
          },
          defer: (fn) => queueMicrotask(fn),
          pool,
          // stats 활성 시에만 note를 단다 — 비활성이면 undefined라 매니저의 note?.() 호출이 즉시 단락된다.
          note: statsEnabled ? (event) => surfaceStats.managerEvent(page, event) : undefined,
        };

        manager = createPageSurfaceManager(effects);

        // blank 샘플러/IO 계측 상태(attach 클로저 스코프 — 아래 $effect가 갱신, 등록이 읽는다).
        let ioSeeded = 0;
        let ioLastBuildAt = 0;
        let ioLastReconcileAt = 0;

        const unregisterStats = surfaceStats.registerPage({
          page,
          wrapper,
          isAttached: () => manager.isAttached(),
          debug: () => manager.debug(),
          poolBackend: () => pool.backendOf(),
          ioSnapshot: () => ({
            seeded: ioSeeded,
            msSinceLastBuild: ioLastBuildAt === 0 ? -1 : Date.now() - ioLastBuildAt,
            msSinceLastReconcile: ioLastReconcileAt === 0 ? -1 : Date.now() - ioLastReconcileAt,
          }),
        });

        const route = (editorKey: object, routedPage: number, backend: SurfaceBackend, acquireHint?: LeaseToken): void => {
          if (editorKey !== editor || routedPage !== page) return;
          manager.onPoolBackend(backend, acquireHint);
          flushIfAttached();
        };
        poolRoutes.add(route);

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
          const state = { inAcquire: false, inRelease: false, isVisible: false };

          type ObserverSet = { observers: IntersectionObserver[]; seeded: number; buildAt: number; seededReported: boolean };
          // 'io' 스와프-온-시드: 재빌드 시 구 세트(activeSet)를 계속 연결·reconcile한 채로 두고,
          // 신 세트(stagingSet)가 seeded>=3에 도달하면 그때 구 세트를 끊는다 — 관찰 공백 0.
          // io off(기본)에서는 stagingSet을 쓰지 않고 즉시 교체해 현행 동작을 그대로 유지한다.
          let activeSet: ObserverSet | null = null;
          let stagingSet: ObserverSet | null = null;
          let lastBuiltH = 0;

          const disconnectSet = (set: ObserverSet | null) => {
            if (!set) return;
            for (const observer of set.observers) observer.disconnect();
            set.observers = [];
          };

          const reconcileNow = () => {
            if (disposed) return;
            ioLastReconcileAt = Date.now();
            surfaceStats.reconcile();
            manager.reconcile({ ...state });
            flushIfAttached();
          };

          const commitStaging = () => {
            if (!stagingSet) return;
            disconnectSet(activeSet);
            activeSet = stagingSet;
            stagingSet = null;
            ioSeeded = activeSet.seeded;
          };

          const makeSet = (cause: string): ObserverSet => {
            const set: ObserverSet = { observers: [], seeded: 0, buildAt: Date.now(), seededReported: false };
            const h = root === null ? window.innerHeight : root.clientHeight;
            lastBuiltH = h;
            ioLastBuildAt = set.buildAt;
            surfaceStats.build(cause);
            const mk = (margin: string, apply: (hit: boolean) => void) => {
              let seededSelf = false;
              const observer = new IntersectionObserver(
                (entries) => {
                  if (disposed) return;
                  if (set !== activeSet && set !== stagingSet) return;
                  apply(entries.at(-1)?.isIntersecting ?? false);
                  if (!seededSelf) {
                    seededSelf = true;
                    set.seeded += 1;
                    if (set === activeSet) ioSeeded = set.seeded;
                  }
                  if (set.seeded >= 3) {
                    if (!set.seededReported) {
                      set.seededReported = true;
                      surfaceStats.seededGap(Date.now() - set.buildAt);
                    }
                    if (set === stagingSet) commitStaging();
                    reconcileNow();
                  }
                },
                { root, rootMargin: margin, threshold: 0 },
              );
              observer.observe(wrapper);
              set.observers.push(observer);
            };
            mk('0px', (hit) => {
              isVisible = hit;
              if (overlaysVisible && !isVisible) {
                keepEmbedsWhileHidden = shouldKeepEmbedsWhileHidden(externalElements);
              }
              overlaysVisible = isVisible;
              state.isVisible = hit;
            });
            mk(`${Math.round(h)}px 0px`, (hit) => (state.inAcquire = hit));
            mk(`${Math.round(1.5 * h)}px 0px`, (hit) => (state.inRelease = hit));
            return set;
          };

          const startBuild = (cause: string) => {
            const set = makeSet(cause);
            if (ioFix && activeSet) {
              disconnectSet(stagingSet);
              stagingSet = set;
            } else {
              disconnectSet(activeSet);
              disconnectSet(stagingSet);
              stagingSet = null;
              activeSet = set;
              ioSeeded = 0;
            }
          };

          const rebuildDebounce = ioFix
            ? createTrailingDebounce((fn, ms) => {
                const id = setTimeout(fn, ms);
                return () => clearTimeout(id);
              }, IO_REBUILD_DEBOUNCE_MS)
            : null;

          const buildHeight = () => (root === null ? window.innerHeight : root.clientHeight);

          const requestRebuild = (cause: string) => {
            if (ioFix) {
              // 작은 높이 델타(툴바 collapse ≈10%)는 재빌드하지 않는다 — 회전/분할화면(큰 델타)만 재빌드.
              if ((cause === 'window-resize' || cause === 'visualviewport-resize') && !shouldRebuildForResize(lastBuiltH, buildHeight())) {
                return;
              }
              rebuildDebounce?.call(() => {
                if (!disposed) startBuild(cause);
              });
            } else {
              startBuild(cause);
            }
          };

          startBuild('initial');
          let resize: ResizeObserver | null = null;
          if (root !== null) {
            resize = new ResizeObserver(() => requestRebuild('root-resize'));
            resize.observe(root);
          }
          const onWindowResize = () => requestRebuild('window-resize');
          const onVvResize = () => {
            surfaceStats.visualViewportResize(window.innerHeight, window.visualViewport?.height ?? 0);
            requestRebuild('visualviewport-resize');
          };
          if (root === null) {
            window.addEventListener('resize', onWindowResize);
            window.visualViewport?.addEventListener('resize', onVvResize);
          }

          return () => {
            disposed = true;
            rebuildDebounce?.cancel();
            resize?.disconnect();
            if (root === null) {
              window.removeEventListener('resize', onWindowResize);
              window.visualViewport?.removeEventListener('resize', onVvResize);
            }
            disconnectSet(activeSet);
            disconnectSet(stagingSet);
          };
        });

        $effect.pre(() => {
          void editor.surfaceScaleFactor;
          void width;
          void backingHeight;
          manager.restyle();
          if (manager.isAttached()) {
            editor.requestSurfaceResize(page, width, backingHeight);
            dirty = false;
            needsResize = false;
            scheduleDeadCheck();
          } else {
            needsResize = true;
            dirty = true;
          }
        });

        return () => {
          unregisterStats();
          poolRoutes.delete(route);
          document.removeEventListener('visibilitychange', onVisible);
          window.removeEventListener('pageshow', onPageShow);
          offRender();
          manager.destroy();
          activePageCount -= 1;
          // 마지막 에디터의 마지막 Page까지 사라졌다 — 파킹된 GL 컨텍스트를 전부 해제한다.
          if (activePageCount === 0) glCanvasRecycler.flush();
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
