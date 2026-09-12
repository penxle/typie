import * as Sentry from '@sentry/sveltekit';
import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import DocumentDomMirror from './DocumentDomMirror.svelte';
import type { Editor } from '$lib/editor-ffi/editor.svelte';

vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));

let component: ReturnType<typeof mount> | undefined;
let consoleError: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  vi.mocked(Sentry.captureException).mockReset();
  consoleError = vi.spyOn(console, 'error').mockImplementation(() => null);
});

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  consoleError.mockRestore();
  document.body.replaceChildren();
});

describe('DocumentDomMirror error reporting', () => {
  it('reports initialization failures without attaching document content', async () => {
    const failure = new Error('invalid document projection');
    const editor = {
      documentDomProjection: () => {
        throw failure;
      },
    } as unknown as Editor;

    component = mount(DocumentDomMirror, { target: document.body, props: { editor } });
    await tick();

    expect(consoleError).toHaveBeenCalledWith(failure);
    expect(Sentry.captureException).toHaveBeenCalledOnce();
    expect(Sentry.captureException).toHaveBeenCalledWith(failure, {
      tags: {
        feature: 'document-dom-mirror',
        phase: 'initialization',
      },
    });
  });
});
