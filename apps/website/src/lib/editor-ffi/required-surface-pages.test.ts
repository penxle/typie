import { describe, expect, it } from 'vitest';
import { requiredSurfacePages } from './required-surface-pages';
import { resolveInstantRevealPreparationViewports, resolveNearestScrollTop, resolveTypewriterScrollTop } from './scroll';
import type { SurfacePageSpan, VerticalSpan } from './required-surface-pages';

const pages = (...boundaries: number[]): SurfacePageSpan[] =>
  boundaries.slice(0, -1).map((top, page) => ({ page, top, bottom: boundaries[page + 1] }));

const required = ({
  pages,
  currentViewport = null,
  activePages = new Set(),
  preparationViewports = [],
}: {
  pages: SurfacePageSpan[];
  currentViewport?: VerticalSpan | null;
  activePages?: Set<number>;
  preparationViewports?: VerticalSpan[];
}): number[] => [...requiredSurfacePages({ pages, currentViewport, activePages, preparationViewports })].toSorted((a, b) => a - b);

describe('requiredSurfacePages', () => {
  it('empty-document', () => {
    expect(
      required({
        pages: [],
        currentViewport: { top: 0, bottom: 100 },
        activePages: new Set([0]),
        preparationViewports: [{ top: 0, bottom: 100 }],
      }),
    ).toEqual([]);
  });

  it('ignores invalid geometry', () => {
    const pageSpans = pages(0, 100, 200);
    expect(
      required({
        pages: pageSpans,
        currentViewport: { top: NaN, bottom: 100 },
        activePages: new Set([1]),
        preparationViewports: [{ top: 300, bottom: 200 }],
      }),
    ).toEqual([]);
  });

  it('current-viewport-acquire-one-height', () => {
    expect(required({ pages: pages(0, 100, 200, 300, 400, 500), currentViewport: { top: 200, bottom: 300 } })).toEqual([1, 2, 3]);
  });

  it('active-target-release-one-and-a-half-heights', () => {
    const pageSpans = [
      { page: 0, top: 0, bottom: 50 },
      { page: 1, top: 50, bottom: 100 },
      { page: 2, top: 100, bottom: 200 },
      { page: 3, top: 200, bottom: 300 },
      { page: 4, top: 300, bottom: 400 },
      { page: 5, top: 400, bottom: 450 },
      { page: 6, top: 450, bottom: 500 },
    ];

    expect(
      required({
        pages: pageSpans,
        currentViewport: { top: 200, bottom: 300 },
        activePages: new Set([0, 1, 5, 6]),
      }),
    ).toEqual([1, 2, 3, 4, 5]);
  });

  it('preparation viewport uses the same acquire range as the destination viewport', () => {
    const pageSpans = [
      { page: 0, top: 0, bottom: 100 },
      { page: 1, top: 120, bottom: 220 },
      { page: 2, top: 240, bottom: 340 },
    ];

    expect(required({ pages: pageSpans, preparationViewports: [{ top: 100, bottom: 120 }] })).toEqual([0, 1]);
    expect(required({ pages: pageSpans, preparationViewports: [{ top: 99, bottom: 120 }] })).toEqual([0, 1]);
    expect(required({ pages: pageSpans, preparationViewports: [{ top: 100, bottom: 121 }] })).toEqual([0, 1]);
  });

  it('zoomed-origin-and-gaps', () => {
    const alreadyDerivedPageSpans = [
      { page: 0, top: 50, bottom: 250 },
      { page: 1, top: 290, bottom: 490 },
    ];

    expect(required({ pages: alreadyDerivedPageSpans, preparationViewports: [{ top: 245, bottom: 295 }] })).toEqual([0, 1]);
  });

  it('out-of-range-active-pages', () => {
    expect(
      required({
        pages: pages(0, 100, 200, 300),
        currentViewport: { top: 0, bottom: 100 },
        activePages: new Set([-1, 2, 3, 100]),
      }),
    ).toEqual([0, 1, 2]);
  });

  it('one-to-three-append', () => {
    const currentViewport = { top: 100, bottom: 200 };

    expect(required({ pages: pages(0, 100), currentViewport, activePages: new Set([0]) })).toEqual([0]);
    expect(required({ pages: pages(0, 100, 200, 300), currentViewport, activePages: new Set([0]) })).toEqual([0, 1, 2]);
  });

  it('four-to-one-shrink', () => {
    expect(
      required({
        pages: pages(0, 100),
        currentViewport: { top: 100, bottom: 200 },
        activePages: new Set([0, 1, 2, 3]),
      }),
    ).toEqual([0]);
  });

  it('long-document-bounded-count', () => {
    const pageSpans = Array.from({ length: 100 }, (_, page) => ({ page, top: page * 100, bottom: (page + 1) * 100 }));

    expect(required({ pages: pageSpans, currentViewport: { top: 5000, bottom: 5100 } })).toEqual([49, 50, 51]);
  });

  it('disjoint-preparation-viewports-do-not-fill-gaps', () => {
    const pageSpans = Array.from({ length: 10 }, (_, page) => ({ page, top: page * 100, bottom: (page + 1) * 100 }));

    expect(
      required({
        pages: pageSpans,
        preparationViewports: [
          { top: 210, bottom: 220 },
          { top: 810, bottom: 820 },
        ],
      }),
    ).toEqual([2, 8]);
  });

  it('instant preparation covers the first destination viewport requirements', () => {
    const pageSpans = pages(0, 100, 200, 300, 400, 500, 600);
    const destinationViewport = { top: 300, bottom: 400 };
    const prepared = required({
      pages: pageSpans,
      currentViewport: { top: 0, bottom: 100 },
      activePages: new Set([0, 1]),
      preparationViewports: [destinationViewport],
    });
    const firstDestinationRequirements = required({
      pages: pageSpans,
      currentViewport: destinationViewport,
      activePages: new Set(prepared),
    });

    expect(prepared).toEqual(expect.arrayContaining(firstDestinationRequirements));
  });

  it('instant preparation contains every destination supported by the production scroll resolvers', () => {
    const metrics = {
      clientHeight: 400,
      scrollHeight: 1400,
      targetTop: 500,
      targetBottom: 650,
      visibleArea: { topInset: 10, bottomInset: 20 },
    };
    const preparation = resolveInstantRevealPreparationViewports({
      mode: 'nearest',
      scrollTop: 350,
      ...metrics,
    });
    const exactDestinations = [0, 350, 800].map((scrollTop) => resolveNearestScrollTop({ scrollTop, ...metrics }) ?? scrollTop);

    for (const scrollTop of exactDestinations) {
      expect(preparation).toContainEqual({ top: scrollTop, bottom: scrollTop + metrics.clientHeight });
    }
  });

  it('instant preparation covers a target taller than the visible range', () => {
    const metrics = {
      clientHeight: 400,
      scrollHeight: 2000,
      targetTop: 600,
      targetBottom: 1100,
      visibleArea: { topInset: 20, bottomInset: 30 },
    };
    const preparation = resolveInstantRevealPreparationViewports({ mode: 'nearest', scrollTop: 0, ...metrics });

    for (const scrollTop of [0, 600, 1500]) {
      const destination = resolveNearestScrollTop({ scrollTop, ...metrics });
      expect(destination).not.toBeNull();
      expect(preparation).toContainEqual({ top: destination, bottom: Number(destination) + metrics.clientHeight });
    }
  });

  it('instant preparation uses production clamp centered fallback and typewriter alignment', () => {
    expect(
      resolveInstantRevealPreparationViewports({
        mode: 'nearest',
        scrollTop: 200,
        clientHeight: 400,
        scrollHeight: 650,
        targetTop: 500,
        targetBottom: 650,
        visibleArea: { topInset: 10, bottomInset: 20 },
      }),
    ).toEqual([{ top: 250, bottom: 650 }]);

    const centeredMetrics = {
      scrollTop: 0,
      clientHeight: 180,
      scrollHeight: 1180,
      targetTop: 500,
      targetBottom: 520,
      visibleArea: { topInset: 70, bottomInset: 70 },
    };
    const centeredDestination = resolveNearestScrollTop(centeredMetrics);
    expect(centeredDestination).not.toBeNull();
    expect(resolveInstantRevealPreparationViewports({ mode: 'nearest', ...centeredMetrics })).toEqual([
      { top: centeredDestination, bottom: Number(centeredDestination) + centeredMetrics.clientHeight },
    ]);

    const typewriterMetrics = {
      scrollTop: 0,
      clientHeight: 400,
      scrollHeight: 1900,
      targetTop: 800,
      targetBottom: 820,
      position: 0.5,
    };
    const typewriterDestination = resolveTypewriterScrollTop(typewriterMetrics);
    expect(resolveInstantRevealPreparationViewports({ mode: 'typewriter', ...typewriterMetrics })).toEqual([
      { top: typewriterDestination, bottom: Number(typewriterDestination) + typewriterMetrics.clientHeight },
    ]);
  });

  it('bounded instant preparation covers each destination cohort without filling disjoint gaps', () => {
    const pageSpans = Array.from({ length: 30 }, (_, page) => ({ page, top: page * 100, bottom: (page + 1) * 100 }));
    const preparationViewports = [
      { targetTop: 300, targetBottom: 320 },
      { targetTop: 2300, targetBottom: 2320 },
    ].flatMap((target) =>
      resolveInstantRevealPreparationViewports({
        mode: 'typewriter',
        scrollTop: 0,
        clientHeight: 100,
        scrollHeight: 3000,
        position: 0.5,
        ...target,
      }),
    );
    const prepared = required({
      pages: pageSpans,
      currentViewport: { top: 0, bottom: 100 },
      preparationViewports,
    });

    for (const destination of preparationViewports) {
      const destinationRequired = required({ pages: pageSpans, currentViewport: destination });
      expect(prepared).toEqual(expect.arrayContaining(destinationRequired));
    }
    expect(prepared).not.toContain(12);
  });

  it('offscreen-selection-is-not-implicit-demand', () => {
    const pageSpans = Array.from({ length: 10 }, (_, page) => ({ page, top: page * 100, bottom: (page + 1) * 100 }));

    // There is intentionally no selection input that could add the offscreen page.
    const result = required({ pages: pageSpans, currentViewport: { top: 0, bottom: 100 } });
    expect(result).toEqual([0, 1]);
    expect(result).not.toContain(8);
  });
});
