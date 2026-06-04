import { describe, expect, it } from 'vitest';
import { createPaginatedLayout, getMaxMargin, mmToPx, resizePageUnit } from '$lib/editor/utils';
import type { PageLayout } from '$lib/editor/utils';

const toV2Layout = (layout: PageLayout) => ({
  type: 'paginated' as const,
  page_width: layout.pageWidth,
  page_height: layout.pageHeight,
  page_margin_top: layout.pageMarginTop,
  page_margin_bottom: layout.pageMarginBottom,
  page_margin_left: layout.pageMarginLeft,
  page_margin_right: layout.pageMarginRight,
});

const fromV2Layout = (layout: ReturnType<typeof toV2Layout>): PageLayout => ({
  pageWidth: layout.page_width,
  pageHeight: layout.page_height,
  pageMarginTop: layout.page_margin_top,
  pageMarginBottom: layout.page_margin_bottom,
  pageMarginLeft: layout.page_margin_left,
  pageMarginRight: layout.page_margin_right,
});

describe('paginated layout helpers', () => {
  it('clamps left/right margins after a width change and stays equivalent through v2 shape conversion', () => {
    const layout: PageLayout = {
      ...createPaginatedLayout('a4'),
      pageMarginLeft: mmToPx(90),
      pageMarginRight: mmToPx(70),
    };

    const actual = resizePageUnit(layout, 'width', 100);
    const expectedBase = { ...layout, pageWidth: mmToPx(100) };
    const expected = {
      ...expectedBase,
      pageMarginLeft: Math.min(expectedBase.pageMarginLeft, getMaxMargin('left', expectedBase)),
      pageMarginRight: Math.min(expectedBase.pageMarginRight, getMaxMargin('right', expectedBase)),
    };

    expect(actual).toEqual(expected);
    expect(fromV2Layout(toV2Layout(actual))).toEqual(expected);
  });

  it('clamps top/bottom margins after a height change and stays equivalent through v2 shape conversion', () => {
    const layout: PageLayout = {
      ...createPaginatedLayout('a4'),
      pageMarginTop: mmToPx(120),
      pageMarginBottom: mmToPx(90),
    };

    const actual = resizePageUnit(layout, 'height', 120);
    const expectedBase = { ...layout, pageHeight: mmToPx(120) };
    const expected = {
      ...expectedBase,
      pageMarginTop: Math.min(expectedBase.pageMarginTop, getMaxMargin('top', expectedBase)),
      pageMarginBottom: Math.min(expectedBase.pageMarginBottom, getMaxMargin('bottom', expectedBase)),
    };

    expect(actual).toEqual(expected);
    expect(fromV2Layout(toV2Layout(actual))).toEqual(expected);
  });

  it('clamps margins to the 0..maxMargin range for oversized and negative inputs', () => {
    const layout = createPaginatedLayout('a4');

    const oversized = resizePageUnit(layout, 'left', 500);
    expect(oversized.pageMarginLeft).toBe(getMaxMargin('left', layout));

    const negative = resizePageUnit(layout, 'top', -10);
    expect(negative.pageMarginTop).toBe(0);
  });
});
