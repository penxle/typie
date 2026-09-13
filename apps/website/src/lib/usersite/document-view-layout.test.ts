import { describe, expect, it } from 'vitest';
import { documentViewColumnStyle, parseDocumentViewLayoutMode, resolveDocumentViewColumn } from './document-view-layout';

describe('parseDocumentViewLayoutMode', () => {
  it('accepts a continuous layout', () => {
    expect(parseDocumentViewLayoutMode({ type: 'continuous', maxWidth: 800 })).toEqual({ type: 'continuous', maxWidth: 800 });
  });

  it('accepts a paginated layout', () => {
    const paginated = {
      type: 'paginated',
      pageWidth: 794,
      pageHeight: 1123,
      pageMarginTop: 60,
      pageMarginBottom: 60,
      pageMarginLeft: 60,
      pageMarginRight: 60,
    };
    expect(parseDocumentViewLayoutMode(paginated)).toEqual(paginated);
  });

  it('falls back to the engine default for anything else', () => {
    const fallback = { type: 'continuous', maxWidth: 600 };
    expect(parseDocumentViewLayoutMode(null)).toEqual(fallback);
    expect(parseDocumentViewLayoutMode(undefined)).toEqual(fallback);
    expect(parseDocumentViewLayoutMode({ type: 'continuous', maxWidth: 0 })).toEqual(fallback);
    expect(parseDocumentViewLayoutMode({ type: 'paginated', pageWidth: 794 })).toEqual(fallback);
    expect(parseDocumentViewLayoutMode({ type: 'other' })).toEqual(fallback);
  });
});

describe('resolveDocumentViewColumn', () => {
  it('matches the continuous track: min(max_width, viewport - padding) plus the view padding', () => {
    expect(resolveDocumentViewColumn({ type: 'continuous', maxWidth: 600 })).toEqual({
      paginated: false,
      width: 'min(640px, 100%)',
      minWidth: '300px',
      paddingLeft: '20px',
      paddingRight: '20px',
    });
  });

  it('matches the paginated track: page width capped at the viewport, margins scaled by the fit zoom', () => {
    expect(
      resolveDocumentViewColumn({
        type: 'paginated',
        pageWidth: 794,
        pageHeight: 1123,
        pageMarginTop: 96,
        pageMarginBottom: 96,
        pageMarginLeft: 60,
        pageMarginRight: 40,
      }),
    ).toEqual({
      paginated: true,
      width: 'min(794px, 100%)',
      minWidth: '0px',
      paddingLeft: 'min(60px, calc(100% * 60 / 794))',
      paddingRight: 'min(40px, calc(100% * 40 / 794))',
    });
  });

  it('serializes the column as an inline style', () => {
    expect(documentViewColumnStyle(resolveDocumentViewColumn({ type: 'continuous', maxWidth: 600 }))).toBe(
      'width:min(640px, 100%);min-width:300px;padding-left:20px;padding-right:20px',
    );
  });
});
