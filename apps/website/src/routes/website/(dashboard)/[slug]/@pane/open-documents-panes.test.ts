import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import OpenDocumentsPanesTestRoot from './open-documents-panes-test-root.svelte';
import type { OpenDocumentRegistry } from '$lib/prism/open-documents.svelte';

const localStorageStub = vi.hoisted(() => {
  const storage = new Map<string, string>();
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      get length() {
        return storage.size;
      },
      clear: () => storage.clear(),
      getItem: (key: string) => storage.get(key) ?? null,
      key: (index: number) => [...storage.keys()][index] ?? null,
      removeItem: (key: string) => storage.delete(key),
      setItem: (key: string, value: string) => storage.set(key, value),
    } satisfies Storage,
  });
  return storage;
});

vi.mock('@typie/ui/context', () => ({
  getAppContext: () => ({ preference: { current: { zenModeEnabled: false } } }),
  tryAppContext: vi.fn(),
}));

vi.mock('./EntityPane.svelte', async () => {
  const module = await import('./open-documents-panes-test-entity-pane.svelte');
  return { default: module.default };
});

type TestRoot = {
  refreshPane: () => void;
  replacePane: () => void;
};

beforeEach(() => {
  localStorageStub.clear();
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe = vi.fn();
      disconnect = vi.fn();
    },
  );
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue(DOMRect.fromRect({ width: 800, height: 600 }));
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  document.body.replaceChildren();
});

describe('Panes open-document readiness', () => {
  it('교체된 페인의 캐시된 문서를 첫 분류에서 받아들인다', async () => {
    let registry: OpenDocumentRegistry | undefined;
    const component = mount(OpenDocumentsPanesTestRoot, {
      target: document.body,
      props: { onRegistry: (value) => (registry = value) },
    }) as TestRoot;

    try {
      await tick();
      expect(registry?.snapshot().documents.map((document) => document.documentId)).toEqual(['D1']);

      component.replacePane();
      await tick();

      expect(registry?.snapshot().documents.map((document) => document.documentId)).toEqual(['D2']);
    } finally {
      await unmount(component);
    }
  });

  it('같은 페인의 데이터가 갱신되어도 열린 문서 분류가 반복되지 않는다', async () => {
    let registry: OpenDocumentRegistry | undefined;
    const component = mount(OpenDocumentsPanesTestRoot, {
      target: document.body,
      props: { onRegistry: (value) => (registry = value) },
    }) as TestRoot;

    try {
      await tick();

      component.refreshPane();
      await tick();

      expect(registry?.snapshot().documents.map((document) => document.documentId)).toEqual(['D1']);
    } finally {
      await unmount(component);
    }
  });
});
