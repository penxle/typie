import stringify from 'fast-json-stable-stringify';
import type { DocumentDomProjection, Modifier, ModifierType, PlainDoc, PlainNodeEntry } from '@typie/editor-ffi/browser';

const NODE_ATTRIBUTE = 'data-typie-node';
const TEXT_CONTAINER_ATTRIBUTE = 'data-typie-text-container';
const TEXT_ATTRIBUTE = 'data-typie-text';
const BOUNDARY_ATTRIBUTE = 'data-typie-boundary';

export type DocumentDomMirror = {
  element: HTMLElement;
  sourceSignature: string;
  project(): { doc: PlainDoc; signature: string };
};

type DocumentDomProjectionOptions = {
  mirror: DocumentDomMirror;
  apply(doc: PlainDoc): void;
  reportError?(error: unknown): void;
};

export const startDocumentDomProjection = ({ mirror, apply, reportError = console.error }: DocumentDomProjectionOptions): (() => void) => {
  let appliedSignature = mirror.sourceSignature;
  let errorReported = false;
  let frame: number | undefined;

  const observer = new MutationObserver(() => {
    if (frame !== undefined) return;
    frame = requestAnimationFrame(() => {
      frame = undefined;
      try {
        const projected = mirror.project();
        if (projected.signature === appliedSignature) return;
        apply(projected.doc);
        appliedSignature = projected.signature;
      } catch (err) {
        if (!errorReported) {
          errorReported = true;
          reportError(err);
        }
      }
    });
  });

  observer.observe(mirror.element, { subtree: true, childList: true, characterData: true });

  return () => {
    observer.disconnect();
    if (frame !== undefined) cancelAnimationFrame(frame);
    frame = undefined;
  };
};

type RunRecord = {
  source: PlainNodeEntry;
};

function clonePlainValue<T>(value: T): T {
  return structuredClone(value);
}

const sameTextShape = (left: PlainNodeEntry, right: PlainNodeEntry) =>
  stringify(left.modifiers) === stringify(right.modifiers) && stringify(left.carry ?? []) === stringify(right.carry ?? []);

const textEntry = (value: string, source?: PlainNodeEntry): PlainNodeEntry => ({
  node: { type: 'text', text: value },
  modifiers: source ? clonePlainValue(source.modifiers) : ({} as Record<ModifierType, Modifier>),
  ...(source?.carry && { carry: clonePlainValue(source.carry) }),
  children: [],
});

const elementsWithAttribute = (root: HTMLElement, attribute: string): HTMLElement[] => {
  const elements = root.hasAttribute(attribute) ? [root] : [];
  elements.push(...root.querySelectorAll<HTMLElement>(`[${attribute}]`));
  return elements;
};

const parseProjectionHtml = (html: string): HTMLElement => {
  const template = document.createElement('template');
  template.innerHTML = html.trim();
  const element = template.content.firstElementChild;
  if (template.content.childElementCount !== 1 || !(element instanceof HTMLElement)) {
    throw new Error('document HTML projection must contain exactly one root element');
  }
  return element;
};

const entryAtPath = (source: PlainDoc, path: string): PlainNodeEntry => {
  if (path === 'r') return source.root;
  let entry = source.root;
  for (const segment of path.split('.')) {
    if (!/^\d+$/.test(segment)) throw new Error(`invalid document projection path: ${path}`);
    const child = entry.children[Number(segment)];
    if (!child) throw new Error(`document projection path is outside its source: ${path}`);
    entry = child;
  }
  return entry;
};

const pathOf = (element: Element, attribute: string): string => {
  const path = element.getAttribute(attribute);
  if (!path) throw new Error(`document projection element is missing ${attribute}`);
  return path;
};

export const createDocumentDomMirror = ({ html, source }: DocumentDomProjection): DocumentDomMirror => {
  const element = parseProjectionHtml(html);
  const elementByEntry = new Map<PlainNodeEntry, { path: string; element: HTMLElement }>();
  const containerEntries = new WeakSet<PlainNodeEntry>();
  const runByElement = new WeakMap<Element, RunRecord>();
  const runByPath = new Map<string, RunRecord>();
  const boundaryByElement = new WeakMap<Element, PlainNodeEntry>();
  const boundaryByPath = new Map<string, PlainNodeEntry>();

  for (const entryElement of elementsWithAttribute(element, NODE_ATTRIBUTE)) {
    const path = pathOf(entryElement, NODE_ATTRIBUTE);
    const entry = entryAtPath(source, path);
    elementByEntry.set(entry, { path, element: entryElement });
    if (entryElement.hasAttribute(TEXT_CONTAINER_ATTRIBUTE)) containerEntries.add(entry);
  }

  for (const runElement of elementsWithAttribute(element, TEXT_ATTRIBUTE)) {
    const path = pathOf(runElement, TEXT_ATTRIBUTE);
    const sourceRun = entryAtPath(source, path);
    if (sourceRun.node.type !== 'text') throw new Error(`document projection run does not reference text: ${path}`);
    const record = { source: sourceRun };
    runByElement.set(runElement, record);
    runByPath.set(path, record);
  }

  for (const boundaryElement of elementsWithAttribute(element, BOUNDARY_ATTRIBUTE)) {
    const path = pathOf(boundaryElement, NODE_ATTRIBUTE);
    const boundary = entryAtPath(source, path);
    boundaryByElement.set(boundaryElement, boundary);
    boundaryByPath.set(path, boundary);
  }

  const resolveEntryElement = (entry: PlainNodeEntry, currentByPath: Map<string, HTMLElement>): HTMLElement | undefined => {
    const record = elementByEntry.get(entry);
    if (!record) return;
    if (record.element === element || element.contains(record.element)) return record.element;
    return currentByPath.get(record.path);
  };

  const collectCurrentRuns = (container: HTMLElement): PlainNodeEntry[] => {
    const entries: PlainNodeEntry[] = [];

    const append = (value: string, sourceRun?: PlainNodeEntry) => {
      if (value.length === 0) return;
      const next = textEntry(value, sourceRun);
      const previous = entries.at(-1);
      if (previous?.node.type === 'text' && sameTextShape(previous, next)) {
        previous.node.text += value;
      } else {
        entries.push(next);
      }
    };

    const visit = (node: Node, activeRun?: RunRecord) => {
      if (node instanceof Text) {
        append(node.data, activeRun?.source);
        return;
      }
      if (!(node instanceof Element) || node.tagName === 'RT') return;

      const boundaryPath = node.hasAttribute(BOUNDARY_ATTRIBUTE) ? node.getAttribute(NODE_ATTRIBUTE) : undefined;
      const boundary = boundaryByElement.get(node) ?? (boundaryPath ? boundaryByPath.get(boundaryPath) : undefined);
      if (boundary) {
        entries.push(clonePlainValue(boundary));
        return;
      }

      const runPath = node.getAttribute(TEXT_ATTRIBUTE);
      const run = runByElement.get(node) ?? (runPath ? runByPath.get(runPath) : undefined) ?? activeRun;
      for (const child of node.childNodes) visit(child, run);
    };

    for (const child of container.childNodes) visit(child);
    return entries;
  };

  const project = (): { doc: PlainDoc; signature: string } => {
    const currentByPath = new Map(
      elementsWithAttribute(element, NODE_ATTRIBUTE).map((current) => [pathOf(current, NODE_ATTRIBUTE), current]),
    );

    const projectEntry = (entry: PlainNodeEntry): PlainNodeEntry => {
      const projected: PlainNodeEntry = {
        node: clonePlainValue(entry.node),
        modifiers: clonePlainValue(entry.modifiers),
        ...(entry.carry && { carry: clonePlainValue(entry.carry) }),
        children: [],
      };

      const container = resolveEntryElement(entry, currentByPath);
      projected.children =
        container && containerEntries.has(entry) ? collectCurrentRuns(container) : entry.children.map((child) => projectEntry(child));
      return projected;
    };

    const doc = { root: projectEntry(source.root) };
    return { doc, signature: stringify(doc) };
  };

  return {
    element,
    sourceSignature: stringify(source),
    project,
  };
};
