export type VerticalSpan = {
  top: number;
  bottom: number;
};

export type SurfacePageSpan = {
  page: number;
  top: number;
  bottom: number;
};

type RequiredSurfacePagesInput = {
  pages: readonly SurfacePageSpan[];
  currentViewport: VerticalSpan | null;
  activePages: ReadonlySet<number>;
  preparationViewports: readonly VerticalSpan[];
};

export const requiredSurfacePages = ({
  pages,
  currentViewport,
  activePages,
  preparationViewports,
}: RequiredSurfacePagesInput): Set<number> => {
  if (pages.length === 0) return new Set();

  const required = new Set<number>();
  const addViewport = (viewport: VerticalSpan): void => {
    if (!hasFinitePositiveHeight(viewport)) return;
    addIntersecting(required, pages, expandBy(viewport, height(viewport)));
    for (const page of intersecting(pages, expandBy(viewport, height(viewport) * 1.5))) {
      if (activePages.has(page)) required.add(page);
    }
  };
  if (currentViewport !== null) addViewport(currentViewport);
  for (const viewport of preparationViewports) addViewport(viewport);

  return required;
};

const intersecting = (pages: readonly SurfacePageSpan[], viewport: VerticalSpan): Set<number> => {
  const result = new Set<number>();
  addIntersecting(result, pages, viewport);
  return result;
};

const addIntersecting = (result: Set<number>, pages: readonly SurfacePageSpan[], viewport: VerticalSpan): void => {
  for (const page of pages) {
    if (hasFinitePositiveHeight(page) && intersects(page, viewport)) result.add(page.page);
  }
};

const expandBy = (span: VerticalSpan, distance: number): VerticalSpan => ({
  top: span.top - distance,
  bottom: span.bottom + distance,
});

const height = (span: VerticalSpan): number => span.bottom - span.top;

const hasFinitePositiveHeight = (span: VerticalSpan): boolean =>
  Number.isFinite(span.top) && Number.isFinite(span.bottom) && span.bottom > span.top;

// Page and viewport spans are half-open: [top, bottom).
const intersects = (page: SurfacePageSpan, viewport: VerticalSpan): boolean => page.top < viewport.bottom && page.bottom > viewport.top;
