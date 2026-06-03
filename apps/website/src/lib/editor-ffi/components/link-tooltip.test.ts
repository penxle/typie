import { describe, expect, it } from 'vitest';
import { resolveSelectionTarget } from './link-tooltip';

describe('resolveSelectionTarget', () => {
  it('shows a tooltip for a range selection when the selected link href is uniform', () => {
    const target = resolveSelectionTarget({
      linkRects: [
        {
          node_id: 't1',
          href: 'https://a.com',
          page_idx: 0,
          rects: [{ x: 10, y: 20, width: 30, height: 10 }],
        },
      ],
      modifierStateLink: {
        type: 'uniform',
        value: { href: 'https://a.com' },
      },
      selection: {
        anchor: { node_id: 't1', offset: 0 },
        head: { node_id: 't1', offset: 5 },
      },
      selectionHeadRect: {
        page_idx: 0,
        rect: { x: 12, y: 20, width: 1, height: 10 },
      },
    });

    expect(target?.link.node_id).toBe('t1');
    expect(target?.link.href).toBe('https://a.com');
  });

  it('falls back to href matching when paragraph selection endpoints are container positions', () => {
    const target = resolveSelectionTarget({
      linkRects: [
        {
          node_id: 't1',
          href: 'https://a.com',
          page_idx: 0,
          rects: [{ x: 10, y: 20, width: 30, height: 10 }],
        },
      ],
      modifierStateLink: {
        type: 'uniform',
        value: { href: 'https://a.com' },
      },
      selection: {
        anchor: { node_id: 'p1', offset: 0 },
        head: { node_id: 'p1', offset: 2 },
      },
      selectionHeadRect: {
        page_idx: 0,
        rect: { x: 12, y: 20, width: 1, height: 10 },
      },
    });

    expect(target?.link.node_id).toBe('t1');
  });

  it('anchors to the link first rect regardless of where the selection head landed', () => {
    const target = resolveSelectionTarget({
      linkRects: [
        {
          node_id: 't1',
          href: 'https://a.com',
          page_idx: 0,
          rects: [
            { x: 10, y: 20, width: 30, height: 10 },
            { x: 0, y: 40, width: 50, height: 10 },
          ],
        },
      ],
      modifierStateLink: {
        type: 'uniform',
        value: { href: 'https://a.com' },
      },
      selection: {
        anchor: { node_id: 't1', offset: 0 },
        head: { node_id: 't1', offset: 8 },
      },
      // Head landed on the second visual line of the link.
      selectionHeadRect: {
        page_idx: 0,
        rect: { x: 20, y: 40, width: 1, height: 10 },
      },
    });

    expect(target?.anchorRect).toEqual({ x: 10, y: 20, width: 30, height: 10 });
  });
});
