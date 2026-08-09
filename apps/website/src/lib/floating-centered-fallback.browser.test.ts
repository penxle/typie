import { computePosition, flip, hide, inline, offset, shift } from '@floating-ui/dom';
import { createCenterWhenReferenceDoesNotFitMiddleware } from '@typie/ui/actions';
import { afterEach, describe, expect, it } from 'vitest';

const GAP = 6;
const PADDING = 8;

const mounted: HTMLElement[] = [];

afterEach(() => {
  for (const element of mounted) element.remove();
  mounted.length = 0;
});

describe('createCenterWhenReferenceDoesNotFitMiddleware', () => {
  it('centers from the complete multi-rect anchor after inline placement resets', async () => {
    const popoverHeight = Math.floor(window.innerHeight / 3);
    const floating = document.createElement('div');
    Object.assign(floating.style, { position: 'absolute', width: '280px', height: `${popoverHeight}px` });
    document.body.append(floating);
    mounted.push(floating);

    const edgeSpace = Math.floor(popoverHeight / 3);
    const first = new DOMRect(100, edgeSpace, 100, 20);
    const last = new DOMRect(100, window.innerHeight - edgeSpace - 20, 200, 20);
    const reference = {
      getBoundingClientRect: () => new DOMRect(100, first.top, 200, last.bottom - first.top),
      getClientRects: () => [first, last],
    };
    const requiredSideSpace = popoverHeight + GAP;

    expect(first.top - 8).toBeLessThan(requiredSideSpace);
    expect(window.innerHeight - 8 - last.bottom).toBeLessThan(requiredSideSpace);
    expect(last.top - 8).toBeGreaterThanOrEqual(requiredSideSpace);
    const centeredFallback = createCenterWhenReferenceDoesNotFitMiddleware({ gap: GAP, overflow: { padding: PADDING } });

    const position = await computePosition(reference, floating, {
      placement: 'right-start',
      middleware: [
        offset(GAP),
        centeredFallback.captureReferenceBounds,
        inline(),
        flip(),
        shift({ padding: PADDING }),
        centeredFallback.centerWhenNeitherSideFits,
        hide(),
      ],
    });

    expect(position.x).toBeCloseTo((window.innerWidth - 280) / 2);
    expect(position.y).toBeCloseTo((window.innerHeight - popoverHeight) / 2);
  });

  it('centers within the configured clipping boundary', async () => {
    const boundary = document.createElement('div');
    Object.assign(boundary.style, {
      position: 'fixed',
      left: '40px',
      top: '40px',
      width: '300px',
      height: '500px',
      overflow: 'hidden',
    });
    document.body.append(boundary);
    mounted.push(boundary);
    const boundaryRect = boundary.getBoundingClientRect();

    const floating = document.createElement('div');
    Object.assign(floating.style, { position: 'absolute', width: '280px', height: '240px' });
    document.body.append(floating);
    mounted.push(floating);

    const first = new DOMRect(100, 100, 100, 20);
    const last = new DOMRect(100, 460, 200, 20);
    const reference = {
      getBoundingClientRect: () => new DOMRect(100, first.top, 200, last.bottom - first.top),
      getClientRects: () => [first, last],
    };
    const centeredFallback = createCenterWhenReferenceDoesNotFitMiddleware({ gap: 4, overflow: { boundary, padding: PADDING } });
    const position = await computePosition(reference, floating, {
      placement: 'right-start',
      middleware: [
        offset(4),
        centeredFallback.captureReferenceBounds,
        inline(),
        flip({ boundary, padding: PADDING }),
        shift({ boundary, padding: PADDING }),
        centeredFallback.centerWhenNeitherSideFits,
        hide({ boundary }),
      ],
    });

    expect(position.x).toBeCloseTo(boundaryRect.left + (boundaryRect.width - 280) / 2);
    expect(position.y).toBeCloseTo(boundaryRect.top + (boundaryRect.height - 240) / 2);
  });
});
