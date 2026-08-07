import { mount, tick, unmount } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import SurfaceHostOwnershipTestRoot from './surface-host-ownership-test-root.svelte';
import type { Editor } from './editor.svelte';
import type { EditorSurfaceHost } from './editor-surface-host.svelte';

vi.mock('@sentry/sveltekit', () => ({ captureException: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));

vi.hoisted(() => {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
});

const PAGE_COUNT = 3;

class RecordingSurfaceHost {
  readonly #log: string[];
  readonly id: string;
  destroyed = false;

  constructor(id: string, log: string[]) {
    this.id = id;
    this.#log = log;
  }

  registerPageContainer(page: number): () => void {
    this.#log.push(`${this.id}:page${page}${this.destroyed ? ':after-destroy' : ''}`);
    return vi.fn();
  }

  destroy(): void {
    this.destroyed = true;
  }
}

// Editor는 실제로도 클래스 인스턴스다. 평범한 객체로 만들면 $state 프록시에 편입되어
// Page.svelte의 `editor.pageEls[page] = el`이 ownership 경고를 낸다.
class FakeEditor {
  readonly pageSizes = Array.from({ length: PAGE_COUNT }, () => ({ width: 320, height: 220 }));
  readonly scaleFactor = 1;
  readonly displayZoom = 1;
  readonly readOnly = false;
  readonly modifierHeld = false;
  readonly rootAttrs = { layout_mode: { type: 'continuous', max_width: 800 } };
  readonly published = undefined;
  readonly publishedRevision = 0;
  readonly externalElements = [];
  readonly pageEls: Record<number, HTMLElement | undefined> = {};

  pageExternalElements(): [] {
    return [];
  }

  pageTableOverlays(): [] {
    return [];
  }

  pageLinkRects(): [] {
    return [];
  }
}

const createEditor = (): Editor => new FakeEditor() as unknown as Editor;

describe('surface host ownership', () => {
  it('never exposes another view instance host to page attachments across an editor swap', async () => {
    const log: string[] = [];
    const errors: unknown[] = [];
    let sequence = 0;

    const props = $state({
      editor: createEditor(),
      createHost: () => new RecordingSurfaceHost(`H${++sequence}`, log) as unknown as EditorSurfaceHost,
      onerror: (error: unknown) => {
        errors.push(error);
      },
    });

    const app = mount(SurfaceHostOwnershipTestRoot, { target: document.body, props });
    await tick();

    expect(errors).toEqual([]);
    expect(log).toEqual(['H1:page0', 'H1:page1', 'H1:page2']);

    log.length = 0;
    props.editor = createEditor();
    await tick();
    await tick();

    // 옛 인스턴스가 파괴되기 전에 새 인스턴스가 먼저 렌더된다. host를 공유 상태에 두면
    // 여기서 새 Page들이 옛 host를, 이어서 undefined를 읽는다.
    expect(errors).toEqual([]);
    expect(log).toEqual(['H2:page0', 'H2:page1', 'H2:page2']);

    await unmount(app);
  });
});
