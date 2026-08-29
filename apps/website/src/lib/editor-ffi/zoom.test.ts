import { describe, expect, it } from 'vitest';
import {
  clampDocumentLayoutZoom,
  computeDocumentFitWidthZoom,
  computeDocumentZoomBounds,
  resolveContinuousLayoutViewportWidth,
  resolveContinuousViewPadding,
  resolveDirectDocumentZoom,
  resolveDocumentZoomIndicator,
  resolveDocumentZoomLandmark,
} from './zoom';

describe('continuous document zoom policy', () => {
  const layout = { type: 'continuous', maxWidth: 600 } as const;

  it.each([
    [1.5, 500],
    [1, 500],
    [0.8, 625],
  ])('maps committed zoom %s to logical viewport width %s', (committedZoom, expected) => {
    expect(resolveContinuousLayoutViewportWidth({ viewportWidth: 500, committedZoom })).toBe(expected);
  });

  it('uses max width plus continuous padding for bounds, fit and snapping', () => {
    expect(computeDocumentZoomBounds(layout)).toEqual({ min: 0.15625, max: 2 });
    expect(computeDocumentFitWidthZoom(layout, 500)).toBe(0.78125);
    expect(computeDocumentFitWidthZoom(layout, 800)).toBe(1.25);
    expect(clampDocumentLayoutZoom({ zoom: 0.79, layout, viewportWidth: 500 })).toBe(0.78125);
  });

  it('falls back safely for invalid viewport and zoom input', () => {
    expect(resolveContinuousLayoutViewportWidth({ viewportWidth: NaN, committedZoom: NaN })).toBe(1);
    expect(clampDocumentLayoutZoom({ zoom: NaN, layout, viewportWidth: NaN })).toBe(0.15625);
  });

  it('scales the engine owned continuous padding with visual zoom', () => {
    expect(resolveContinuousViewPadding(1.5)).toBe(30);
    expect(resolveContinuousViewPadding(0.5)).toBe(10);
    expect(resolveContinuousViewPadding(NaN)).toBe(20);
  });

  it('keeps snapping out of direct input and rubber bands past hard bounds', () => {
    expect(resolveDirectDocumentZoom(0.79, layout)).toBe(0.79);
    expect(resolveDirectDocumentZoom(2.2, layout)).toBeGreaterThan(2);
    expect(resolveDirectDocumentZoom(2.2, layout)).toBeLessThan(2.2);
  });

  it('reports only the normal range to the zoom indicator', () => {
    expect(resolveDocumentZoomIndicator(2.08, layout)).toBe(2);
    expect(resolveDocumentZoomIndicator(0.1, layout)).toBe(computeDocumentZoomBounds(layout).min);
  });
});

describe('document zoom landmarks', () => {
  const layout = { type: 'paginated', pageWidth: 1000 } as const;

  it.each([
    [0.1, 500, 'minimum'],
    [0.5, 500, 'fit-width'],
    [0.75, 500, null],
    [1, 500, 'unit'],
    [2, 500, 'maximum'],
  ] as const)('resolves zoom %s in viewport %s as %s', (zoom, viewportWidth, expected) => {
    expect(resolveDocumentZoomLandmark({ zoom, layout, viewportWidth })).toBe(expected);
  });

  it('prefers unit over fit-width and fit-width over a bound', () => {
    expect(resolveDocumentZoomLandmark({ zoom: 1, layout, viewportWidth: 1000 })).toBe('unit');
    expect(resolveDocumentZoomLandmark({ zoom: 0.1, layout, viewportWidth: 100 })).toBe('fit-width');
  });

  it('does not call a clamped fit-width target a fit landmark', () => {
    expect(resolveDocumentZoomLandmark({ zoom: 0.1, layout, viewportWidth: 50 })).toBe('minimum');
    expect(resolveDocumentZoomLandmark({ zoom: 2, layout, viewportWidth: 2500 })).toBe('maximum');
  });

  it.each([
    [NaN, 500, layout],
    [1, 0, layout],
    [1, NaN, layout],
    [1, 500, { type: 'paginated', pageWidth: NaN } as const],
  ])('does not name invalid zoom or layout input', (zoom, viewportWidth, invalidLayout) => {
    expect(resolveDocumentZoomLandmark({ zoom, layout: invalidLayout, viewportWidth })).toBeNull();
  });
});
