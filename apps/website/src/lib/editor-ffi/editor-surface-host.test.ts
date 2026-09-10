import { describe, expect, it, vi } from 'vitest';
import { EditorSurfaceHost } from './editor-surface-host.svelte';
import type { Editor, PublishedBundle } from './editor.svelte';

describe('EditorSurfaceHost', () => {
  it('reactivates a retained published producer when its page becomes required again', () => {
    let attachedSurface: HTMLElement | undefined;
    let targetAttached = false;
    const attachSurface = vi.fn((_page: number, surface: HTMLElement) => {
      attachedSurface = surface;
      targetAttached = true;
      return 'cpu';
    });
    const detachSurface = vi.fn(() => {
      targetAttached = false;
    });
    const editor = {
      terminal: false,
      appliedSnapshot: {
        pageSizes: [{ width: 320, height: 480 }],
        pageBackingSizes: [],
      },
      surfacePageRequirements: new Set([0]),
      scaleFactor: 1,
      published: undefined,
      activateVisualHost: () => vi.fn(),
      publishedSurfaceElement: () => attachedSurface,
      surfaceConfigMatches: () => targetAttached,
      attachSurface,
      detachSurface,
      invalidateSurface: vi.fn(),
      surfaceReplacementFailed: vi.fn(),
    } as unknown as Editor;
    const host = new EditorSurfaceHost(editor, vi.fn());

    host.reconcile(new Set([0]));
    const firstSurface = attachedSurface;
    expect(firstSurface).toBeDefined();
    host.syncPublished({ frames: new Map([[0, { surface: firstSurface }]]) } as unknown as PublishedBundle);

    host.reconcile(new Set());
    expect(detachSurface).toHaveBeenCalledOnce();

    host.reconcile(new Set([0]));
    expect(attachSurface).toHaveBeenCalledTimes(2);

    host.destroy();
  });
});
