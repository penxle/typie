import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import PublicViewerContextTestHost from './public-viewer-context-test-host.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

let mounted: Record<string, unknown> | undefined;

afterEach(async () => {
  if (mounted) await unmount(mounted);
  mounted = undefined;
  document.body.replaceChildren();
});

describe('public viewer editor context', () => {
  it('initializes editor UI without an AppContext provider', () => {
    mounted = mount(PublicViewerContextTestHost, { target: document.body });

    expect(document.querySelector('[data-public-viewer-context-ready]')).not.toBeNull();
  });
});
