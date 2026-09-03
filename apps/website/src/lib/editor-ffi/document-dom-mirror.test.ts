import { afterEach, describe, expect, it, vi } from 'vitest';
import { createDocumentDomMirror, startDocumentDomProjection } from './document-dom-mirror';
import type { DocumentDomProjection, Modifier, ModifierType, PlainDoc, PlainNodeEntry } from '@typie/editor-ffi/browser';

const entry = (
  node: PlainNodeEntry['node'],
  children: PlainNodeEntry[] = [],
  modifiers: Partial<Record<ModifierType, Modifier>> = {},
): PlainNodeEntry => ({
  node,
  children,
  modifiers: modifiers as Record<ModifierType, Modifier>,
});

const text = (value: string, modifiers: Partial<Record<ModifierType, Modifier>> = {}) =>
  entry({ type: 'text', text: value }, [], modifiers);

const doc = (...children: PlainNodeEntry[]): PlainDoc => ({
  root: entry({ type: 'root', layout_mode: { type: 'continuous', max_width: 640 } }, [entry({ type: 'paragraph' }, children)]),
});

const inlineChildren = (plain: PlainDoc): PlainNodeEntry[] => {
  const [paragraph] = plain.root.children;
  if (!paragraph) throw new Error('expected paragraph');
  return paragraph.children;
};

const escapeHtml = (value: string) => value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');

const defaultInlineHtml = (source: PlainDoc): string =>
  inlineChildren(source)
    .map((child, index) => {
      const path = `0.${index}`;
      switch (child.node.type) {
        case 'text': {
          return `<span data-typie-text="${path}">${escapeHtml(child.node.text)}</span>`;
        }
        case 'hard_break': {
          return `<br data-typie-node="${path}" data-typie-boundary>`;
        }
        case 'page_break': {
          return `<span data-typie-node="${path}" data-typie-boundary></span>`;
        }
        default: {
          throw new Error(`unsupported test fixture node: ${child.node.type}`);
        }
      }
    })
    .join('');

const projection = (source: PlainDoc, inlineHtml = defaultInlineHtml(source)): DocumentDomProjection => ({
  source,
  html: `<article data-typie-node="r"><p data-typie-node="0" data-typie-text-container>${inlineHtml}</p></article>`,
});

const createMirror = (source: PlainDoc, inlineHtml?: string) => createDocumentDomMirror(projection(source, inlineHtml));

function at<T>(values: T[], index: number): T {
  const value = values[index];
  if (value === undefined) throw new Error(`expected value at ${index}`);
  return value;
}

const flushFrame = (frames: Map<number, FrameRequestCallback>) => {
  const callback = frames.values().next().value;
  if (!callback) throw new Error('expected animation frame');
  callback(0);
  frames.clear();
};

describe('document DOM mirror conversion', () => {
  it('projects translated run text while preserving source-owned semantic modifiers', () => {
    const source = doc(
      text('Important', {
        bold: { type: 'bold' },
        link: { type: 'link', href: 'https://example.com/original' },
        text_color: { type: 'text_color', value: '#c2410c' },
      }),
      text(' Tokyo', { ruby: { type: 'ruby', text: 'とうきょう' } }),
    );

    const mirror = createMirror(source);
    const runs = [...mirror.element.querySelectorAll<HTMLElement>('[data-typie-text]')];

    runs[0].textContent = '중요한';
    runs[1].textContent = ' 도쿄';

    const projected = inlineChildren(mirror.project().doc);
    const sourceChildren = inlineChildren(source);

    expect(projected.map((child) => (child.node.type === 'text' ? child.node.text : ''))).toEqual(['중요한', ' 도쿄']);
    expect(at(projected, 0).modifiers).toEqual(at(sourceChildren, 0).modifiers);
    expect(at(projected, 1).modifiers).toEqual(at(sourceChildren, 1).modifiers);
  });

  it('preserves frozen inline boundary nodes between translated runs', () => {
    const source = doc(text('Before'), entry({ type: 'hard_break' }), text('After'));
    const mirror = createMirror(source);
    const runs = [...mirror.element.querySelectorAll<HTMLElement>('[data-typie-text]')];

    runs[0].textContent = '앞';
    runs[1].textContent = '뒤';

    expect(inlineChildren(mirror.project().doc).map((child) => child.node)).toEqual([
      { type: 'text', text: '앞' },
      { type: 'hard_break' },
      { type: 'text', text: '뒤' },
    ]);
  });

  it('projects translated text from a paragraph that ends with a page break', () => {
    const source = doc(text('Before'), entry({ type: 'page_break' }));
    const mirror = createMirror(source);
    const run = mirror.element.querySelector<HTMLElement>('[data-typie-text]');
    if (!run) throw new Error('expected text run');

    run.textContent = '앞';

    expect(inlineChildren(mirror.project().doc).map((child) => child.node)).toEqual([{ type: 'text', text: '앞' }, { type: 'page_break' }]);
  });

  it('uses original identity before attributes and source values before translated attributes', () => {
    const source = doc(
      text('Link', {
        link: { type: 'link', href: 'https://example.com/original' },
        text_color: { type: 'text_color', value: '#c2410c' },
      }),
      text(' Plain'),
    );
    const mirror = createMirror(
      source,
      '<a href="https://example.com/original"><span data-typie-text="0.0">Link</span></a><span data-typie-text="0.1"> Plain</span>',
    );
    const runs = [...mirror.element.querySelectorAll<HTMLElement>('[data-typie-text]')];

    runs[0].dataset.typieText = runs[1].dataset.typieText;
    runs[0].textContent = '링크';
    runs[0].style.color = 'rgb(0, 0, 0)';
    mirror.element.querySelector('a')?.setAttribute('href', 'https://example.com/translated');

    const projected = at(inlineChildren(mirror.project().doc), 0);

    expect(projected.node).toEqual({ type: 'text', text: '링크' });
    expect(projected.modifiers).toEqual(at(inlineChildren(source), 0).modifiers);
  });

  it('recovers replaced and unknown-wrapper text without trusting translated markup', () => {
    const source = doc(text('Known', { italic: { type: 'italic' } }), text(' Tail'));
    const mirror = createMirror(source);
    const runs = [...mirror.element.querySelectorAll<HTMLElement>('[data-typie-text]')];
    const replacement = runs[0].cloneNode(false) as HTMLElement;
    const unknown = document.createElement('mark');

    replacement.textContent = '알려진';
    runs[0].replaceWith(replacement);
    unknown.textContent = ' 이동';
    runs[1].replaceWith(unknown);

    const projected = inlineChildren(mirror.project().doc);

    expect(projected.map((child) => child.node)).toEqual([
      { type: 'text', text: '알려진' },
      { type: 'text', text: ' 이동' },
    ]);
    expect(at(projected, 0).modifiers).toEqual(at(inlineChildren(source), 0).modifiers);
    expect(at(projected, 1).modifiers).toEqual({});
  });

  it('resolves a browser-replaced inline container through its source path', () => {
    const mirror = createMirror(doc(text('Original', { bold: { type: 'bold' } })));
    const paragraph = mirror.element.querySelector('p');
    if (!paragraph) throw new Error('expected paragraph');
    const replacement = paragraph.cloneNode(true) as HTMLElement;
    const run = replacement.querySelector<HTMLElement>('[data-typie-text]');
    if (!run) throw new Error('expected translated run');

    run.textContent = '교체된 문단';
    paragraph.replaceWith(replacement);

    expect(inlineChildren(mirror.project().doc).map((child) => child.node)).toEqual([{ type: 'text', text: '교체된 문단' }]);
  });

  it('excludes ruby annotations and merges adjacent translated runs with equal modifiers', () => {
    const source = doc(text('Tokyo', { ruby: { type: 'ruby', text: 'とうきょう' } }), text(' is'), text(' large'));
    const mirror = createMirror(
      source,
      '<ruby><span data-typie-text="0.0">Tokyo</span><rt translate="no">とうきょう</rt></ruby><span data-typie-text="0.1"> is</span><span data-typie-text="0.2"> large</span>',
    );
    const runs = [...mirror.element.querySelectorAll<HTMLElement>('[data-typie-text]')];

    runs[0].textContent = '도쿄';
    runs[1].textContent = '는';
    runs[2].textContent = ' 크다';
    const annotation = mirror.element.querySelector('rt');
    if (!annotation) throw new Error('expected ruby annotation');
    annotation.textContent = '번역되면 안 됨';

    expect(inlineChildren(mirror.project().doc).map((child) => child.node)).toEqual([
      { type: 'text', text: '도쿄' },
      { type: 'text', text: '는 크다' },
    ]);
  });

  it('changes the full signature when formatting boundaries change without changing prose', () => {
    const source = doc(text('A', { bold: { type: 'bold' } }), text('B', { italic: { type: 'italic' } }));
    const mirror = createMirror(source);
    const runs = [...mirror.element.querySelectorAll<HTMLElement>('[data-typie-text]')];

    runs[0].textContent = 'AB';
    runs[1].textContent = '';

    const projected = mirror.project();

    expect(inlineChildren(projected.doc).map((child) => child.node)).toEqual([{ type: 'text', text: 'AB' }]);
    expect(projected.signature).not.toBe(mirror.sourceSignature);
    expect(mirror.project().signature).toBe(projected.signature);
  });
});

describe('document DOM projection', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('batches mutations per frame and suppresses duplicate projections', async () => {
    const frames = new Map<number, FrameRequestCallback>();
    let nextFrame = 1;
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      const id = nextFrame++;
      frames.set(id, callback);
      return id;
    });
    vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));

    const mirror = createMirror(doc(text('First'), text(' Second')));
    const runs = [...mirror.element.querySelectorAll<HTMLElement>('[data-typie-text]')];
    const applied: PlainDoc[] = [];
    const stop = startDocumentDomProjection({
      mirror,
      apply: (projected) => {
        applied.push(projected);
      },
    });

    runs[0].textContent = '첫째';
    runs[1].textContent = ' 둘째';
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(frames.size).toBe(1);
    flushFrame(frames);
    expect(applied).toHaveLength(1);

    runs[0].textContent = '첫째';
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushFrame(frames);
    expect(applied).toHaveLength(1);

    runs[0].textContent = 'First';
    runs[1].textContent = ' Second';
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushFrame(frames);
    expect(applied).toHaveLength(2);
    expect(applied[1]).toEqual(doc(text('First Second')));

    stop();
    runs[0].textContent = '중단 뒤';
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(frames.size).toBe(0);
  });

  it('keeps the last successful signature when projection fails before apply', async () => {
    const frames: FrameRequestCallback[] = [];
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      frames.push(callback);
      return frames.length;
    });
    vi.stubGlobal('cancelAnimationFrame', vi.fn());

    const root = document.createElement('div');
    const apply = vi.fn();
    const reportError = vi.fn();
    const stop = startDocumentDomProjection({
      mirror: {
        element: root,
        sourceSignature: 'source',
        project: () => {
          throw new Error('invalid translated DOM');
        },
      },
      apply,
      reportError,
    });

    root.append('translation');
    await new Promise((resolve) => setTimeout(resolve, 0));
    const frame = frames.shift();
    if (!frame) throw new Error('expected animation frame');
    frame(0);

    expect(apply).not.toHaveBeenCalled();
    expect(reportError).toHaveBeenCalledOnce();

    root.append('more translation');
    await new Promise((resolve) => setTimeout(resolve, 0));
    const nextFrame = frames.shift();
    if (!nextFrame) throw new Error('expected another animation frame');
    nextFrame(0);

    expect(apply).not.toHaveBeenCalled();
    expect(reportError).toHaveBeenCalledOnce();
    stop();
  });
});
