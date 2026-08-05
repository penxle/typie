import { getAppContext } from '@typie/ui/context';
import { untrack } from 'svelte';
import { CONTINUOUS_VIEW_PADDING } from './constants';
import { selectionHeadRect } from './geometry';
import {
  resolveInstantRevealPreparationViewports,
  resolveKeepVisibleBottomPadding,
  resolveNearestScrollTop,
  resolveTypewriterBottomPadding,
  resolveTypewriterScrollTop,
} from './scroll';
import type { PageRect } from '@typie/editor-ffi/browser';
import type { Editor, EditorContext, EditorSnapshot } from './editor.svelte';
import type { EditorRequest } from './editor-update';
import type { VerticalSpan } from './required-surface-pages';
import type { EditorVisibleArea, RevealTargetSpan, ScrollContainerMetrics } from './scroll';

export type EditorScrollRevealMode = 'nearest' | 'typewriter';

export type EditorScrollIntoViewTarget = { type: 'current_selection_head' } | { type: 'tracked_item'; id: string };

export type EditorScrollIntoViewOptions = {
  target: EditorScrollIntoViewTarget;
  mode?: EditorScrollRevealMode;
  behavior?: ScrollBehavior;
};

export type EditorScrollIntentResult = { type: 'unresolved' } | { type: 'no_scroll' } | { type: 'scroll_to'; y: number };

type TypewriterPreferences = {
  enabled: boolean;
  position: number | undefined;
};

export type EditorBringIntoViewRequest = Required<EditorScrollIntoViewOptions> & {
  targetRevision: number | null;
  presentation: Promise<void>;
  completePresentation: () => void;
};

const DEFAULT_VISIBLE_AREA: EditorVisibleArea = {
  topInset: 0,
  bottomInset: 0,
};

function sameVisibleArea(a: EditorVisibleArea, b: EditorVisibleArea): boolean {
  return a.topInset === b.topInset && a.bottomInset === b.bottomInset;
}

function sanitizeInset(value: number): number {
  return Number.isFinite(value) ? Math.max(0, value) : 0;
}

function sanitizeVisibleArea(visibleArea: EditorVisibleArea): EditorVisibleArea {
  return {
    topInset: sanitizeInset(visibleArea.topInset),
    bottomInset: sanitizeInset(visibleArea.bottomInset),
  };
}

function sanitizeTypewriterPosition(position: number | undefined): number {
  return typeof position === 'number' && Number.isFinite(position) ? Math.max(0, Math.min(1, position)) : 0.5;
}

function createBringIntoViewRequest({ target, mode = 'nearest', behavior }: EditorScrollIntoViewOptions): EditorBringIntoViewRequest {
  const { promise: presentation, resolve } = Promise.withResolvers<undefined>();
  return {
    target,
    mode,
    behavior: behavior ?? (target.type === 'tracked_item' ? 'smooth' : 'instant'),
    targetRevision: null,
    presentation,
    completePresentation: () => resolve(undefined),
  };
}

export function setupEditorScroll(ctx: EditorContext): void {
  const app = getAppContext();

  $effect(() => {
    const editor = ctx.editor;
    if (!editor) {
      ctx.scroll = undefined;
      return;
    }

    // app is absent in the public viewer (no AppContext provider); fall back to typewriter off.
    const scope = new EditorScrollScope(editor, () => {
      const preference = app?.preference.current;
      return {
        enabled: preference?.typewriterEnabled ?? false,
        position: preference?.typewriterPosition,
      };
    });
    ctx.scroll = scope;
    editor.registerScrollIntoView((options, request) => scope.scrollIntoView(options, request));

    return () => {
      editor.registerScrollIntoView(null);
      scope.destroy();
      if (ctx.scroll === scope) {
        ctx.scroll = undefined;
      }
    };
  });

  $effect(() => {
    const editor = ctx.editor;
    const scroll = ctx.scroll;
    if (editor?.terminal) scroll?.destroy();
  });
}

export class EditorScrollScope {
  #pendingRequest: EditorBringIntoViewRequest | null = null;
  #keepVisibleTarget = $state<EditorScrollIntoViewTarget | null>(null);
  #destroyed = false;
  readonly #editor: Editor;
  readonly #typewriterPreferences: () => TypewriterPreferences;

  visibleArea = $state<EditorVisibleArea>(DEFAULT_VISIBLE_AREA);

  bottomPadding = $derived.by(() => {
    void this.#editor.viewport.height;
    return this.bottomPaddingFor(this.#editor.published?.snapshot);
  });

  constructor(editor: Editor, typewriterPreferences: () => TypewriterPreferences) {
    this.#editor = editor;
    this.#typewriterPreferences = typewriterPreferences;
  }

  get pendingRequest(): EditorBringIntoViewRequest | null {
    return this.#pendingRequest;
  }

  bottomPaddingFor(snapshot: EditorSnapshot | undefined): number {
    const rect = selectionHeadRect(snapshot);
    const needsKeepVisiblePadding = rect !== null || this.#hasResolvedKeepVisibleTarget(snapshot);
    const minimumPadding = needsKeepVisiblePadding
      ? resolveKeepVisibleBottomPadding({ visibleArea: this.visibleArea })
      : this.visibleArea.bottomInset;
    if (!rect) {
      return minimumPadding;
    }

    const prefs = this.#typewriterPreferences();
    if (!prefs.enabled) {
      return minimumPadding;
    }

    return Math.max(minimumPadding, this.#typewriterBottomPaddingForRect(rect, snapshot));
  }

  resolveTargetRects(target: EditorScrollIntoViewTarget, snapshot: EditorSnapshot | undefined): PageRect[] | null {
    if (!snapshot) return null;
    switch (target.type) {
      case 'current_selection_head': {
        const rect = selectionHeadRect(snapshot);
        return rect ? [rect] : null;
      }
      case 'tracked_item': {
        const range = snapshot.trackedRanges.find((item) => item.id === target.id);
        return range && range.rects.length > 0 ? range.rects : null;
      }
    }
  }

  applyPending(request: EditorBringIntoViewRequest, snapshot: EditorSnapshot, result: EditorScrollIntentResult): boolean {
    if (this.#destroyed || this.#editor.destroyed || this.activateForRevision(snapshot.revision) !== request) return false;
    switch (result.type) {
      case 'unresolved': {
        return false;
      }
      case 'no_scroll': {
        break;
      }
      case 'scroll_to': {
        const viewport = this.#editor.scrollViewport;
        if (!viewport) return false;
        viewport.scrollTo({ top: result.y, behavior: request.behavior });
        break;
      }
    }
    return this.markPresented(snapshot.revision, request);
  }

  declare(options: EditorScrollIntoViewOptions): EditorBringIntoViewRequest {
    const request = createBringIntoViewRequest(options);
    this.#pendingRequest?.completePresentation();
    this.#pendingRequest = request;
    this.#keepVisibleTarget = request.target;
    this.#editor.requestPublication();
    return request;
  }

  bind(request: EditorBringIntoViewRequest, revision: number): boolean {
    if (this.#pendingRequest !== request) return false;
    if (request.targetRevision !== null) return request.targetRevision === revision;
    request.targetRevision = revision;
    return this.#pendingRequest === request;
  }

  activateForRevision(revision: number): EditorBringIntoViewRequest | null {
    const request = this.#pendingRequest;
    if (!request || request.targetRevision === null) return null;
    return revision >= request.targetRevision ? request : null;
  }

  discard(request: EditorBringIntoViewRequest): void {
    if (this.#pendingRequest !== request) return;
    this.#pendingRequest = null;
    request.completePresentation();
    this.#editor.requestPublication();
  }

  discardFailedForRevision(revision: number): void {
    const request = this.activateForRevision(revision);
    if (request) this.discard(request);
  }

  cancel(): void {
    const request = this.#pendingRequest;
    if (request) this.discard(request);
  }

  markPresented(revision: number, request: EditorBringIntoViewRequest): boolean {
    if (this.activateForRevision(revision) !== request) return false;
    this.#pendingRequest = null;
    request.completePresentation();
    this.#editor.requestPublication();
    return true;
  }

  resolveScrollTop(request: EditorBringIntoViewRequest, metrics: ScrollContainerMetrics & RevealTargetSpan): number | null {
    const mode = request.mode === 'typewriter' && this.#typewriterPreferences().enabled ? 'typewriter' : 'nearest';
    return mode === 'typewriter'
      ? resolveTypewriterScrollTop({
          ...metrics,
          visibleArea: this.visibleArea,
          position: sanitizeTypewriterPosition(this.#typewriterPreferences().position),
        })
      : resolveNearestScrollTop({ ...metrics, visibleArea: this.visibleArea });
  }

  resolvePreparationViewports(request: EditorBringIntoViewRequest, metrics: ScrollContainerMetrics & RevealTargetSpan): VerticalSpan[] {
    const mode = request.mode === 'typewriter' && this.#typewriterPreferences().enabled ? 'typewriter' : 'nearest';
    return resolveInstantRevealPreparationViewports({
      ...metrics,
      mode,
      visibleArea: this.visibleArea,
      position: sanitizeTypewriterPosition(this.#typewriterPreferences().position),
    });
  }

  // eslint-disable-next-line unicorn/consistent-class-member-order -- public scroll contract is grouped before private padding details
  #hasResolvedKeepVisibleTarget(snapshot: EditorSnapshot | undefined): boolean {
    const target = this.#keepVisibleTarget;
    return target !== null && this.resolveTargetRects(target, snapshot) !== null;
  }

  #typewriterBottomPaddingForRect(rect: PageRect, snapshot: EditorSnapshot | undefined): number {
    const viewport = this.#editor.scrollViewport;
    if (!viewport) {
      return 0;
    }

    const viewportRect = viewport.getRect();
    const zoom = this.#editor.safeDisplayZoom();
    const layoutMode = snapshot?.rootAttrs?.layout_mode;
    const trailingBottomMargin = layoutMode?.type === 'paginated' ? layoutMode.page_margin_bottom * zoom : CONTINUOUS_VIEW_PADDING;
    return resolveTypewriterBottomPadding({
      clientHeight: viewportRect.bottom - viewportRect.top,
      targetHeight: rect.rect.height * zoom,
      visibleArea: this.visibleArea,
      position: sanitizeTypewriterPosition(this.#typewriterPreferences().position),
      trailingBottomMargin,
    });
  }

  destroy(): void {
    this.#destroyed = true;
    this.#pendingRequest?.completePresentation();
    this.#pendingRequest = null;
  }

  setVisibleArea(visibleArea: EditorVisibleArea): void {
    const next = sanitizeVisibleArea(visibleArea);
    if (sameVisibleArea(this.visibleArea, next)) {
      return;
    }
    this.visibleArea = next;
    this.#editor.requestPublication();
  }

  setBottomInset(bottomInset: number): void {
    untrack(() => {
      this.setVisibleArea({
        topInset: this.visibleArea.topInset,
        bottomInset,
      });
    });
  }

  scrollIntoView(
    { target, mode = 'nearest', behavior }: EditorScrollIntoViewOptions,
    admission?: EditorRequest,
  ): Promise<void> | undefined {
    if (this.#destroyed) {
      return;
    }

    const request = this.declare({ target, mode, behavior });
    if (admission) {
      admission.beforePublish(
        (update) => {
          if (!this.bind(request, update.revision)) this.discard(request);
        },
        () => this.discard(request),
      );
    } else {
      this.bind(request, this.#editor.appliedSnapshot.revision);
    }
    return request.presentation;
  }
}
