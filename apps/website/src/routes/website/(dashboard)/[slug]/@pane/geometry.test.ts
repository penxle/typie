import { describe, expect, it } from 'vitest';
import { resolveTopCornerPaneIds } from './geometry';
import type { Rect } from './types';

const panes = (entries: [string, Rect][]) => new Map(entries);

describe('resolveTopCornerPaneIds', () => {
  it('assigns both corners to a single pane', () => {
    expect(resolveTopCornerPaneIds(panes([['only', { left: 0, top: 0, width: 800, height: 600 }]]))).toEqual({
      topLeftPaneId: 'only',
      topRightPaneId: 'only',
    });
  });

  it('assigns horizontal corners from rendered positions instead of map order', () => {
    expect(
      resolveTopCornerPaneIds(
        panes([
          ['right', { left: 404, top: 0, width: 396, height: 600 }],
          ['left', { left: 0, top: 0, width: 400, height: 600 }],
        ]),
      ),
    ).toEqual({ topLeftPaneId: 'left', topRightPaneId: 'right' });
  });

  it('assigns both corners to the top pane in a vertical split', () => {
    expect(
      resolveTopCornerPaneIds(
        panes([
          ['bottom', { left: 0, top: 304, width: 800, height: 296 }],
          ['top', { left: 0, top: 0, width: 800, height: 300 }],
        ]),
      ),
    ).toEqual({ topLeftPaneId: 'top', topRightPaneId: 'top' });
  });

  it('selects the two panes that touch the upper corners in a nested split', () => {
    expect(
      resolveTopCornerPaneIds(
        panes([
          ['bottom-right', { left: 404, top: 304, width: 396, height: 296 }],
          ['top-right', { left: 404, top: 0, width: 396, height: 300 }],
          ['bottom-left', { left: 0, top: 304, width: 400, height: 296 }],
          ['top-left', { left: 0, top: 0, width: 400, height: 300 }],
        ]),
      ),
    ).toEqual({ topLeftPaneId: 'top-left', topRightPaneId: 'top-right' });
  });
});
