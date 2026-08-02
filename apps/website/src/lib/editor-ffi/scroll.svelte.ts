import { getAppContext } from '@typie/ui/context';
import { tick, untrack } from 'svelte';
import { CONTINUOUS_VIEW_PADDING } from './constants';
import { pageRectsToClientRect, selectionHeadRect } from './geometry';
import {
  resolveKeepVisibleBottomPadding,
  resolveNearestScrollTop,
  resolveTypewriterBottomPadding,
  resolveTypewriterScrollTop,
} from './scroll';
import type { PageRect } from '@typie/editor-ffi/browser';
import type { Editor, EditorContext, EditorSnapshot } from './editor.svelte';
import type { EditorVisibleArea } from './scroll';

export type EditorScrollRevealMode = 'nearest' | 'typewriter';

export type EditorScrollIntoViewTarget = { type: 'current_selection_head' } | { type: 'tracked_item'; id: string };

export type EditorScrollIntoViewOptions = {
  target: EditorScrollIntoViewTarget;
  mode?: EditorScrollRevealMode;
  behavior?: ScrollBehavior;
};

type TypewriterPreferences = {
  enabled: boolean;
  position: number | undefined;
};

type PendingScrollRequest = Required<EditorScrollIntoViewOptions>;

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
    editor.registerScrollIntoView((options) => scope.scrollIntoView(options));

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
    if (!editor || !scroll) return;

    // Scroll targets are visible geometry and must use the canvas-matching publication.
    void editor.publishedRevision;
    void editor.viewport.height;

    untrack(() => scroll.scheduleCommit());
  });
}

export class EditorScrollScope {
  #pendingRequest: PendingScrollRequest | null = null;
  #keepVisibleTarget = $state<EditorScrollIntoViewTarget | null>(null);
  #commitQueued = false;
  #destroyed = false;
  readonly #editor: Editor;
  readonly #typewriterPreferences: () => TypewriterPreferences;

  visibleArea = $state<EditorVisibleArea>(DEFAULT_VISIBLE_AREA);

  bottomPadding = $derived.by(() => {
    void this.#editor.viewport.height;
    const snapshot = this.#editor.published?.snapshot;
    const rect = selectionHeadRect(snapshot);
    const needsKeepVisiblePadding = rect !== undefined || this.#hasResolvedKeepVisibleTarget(snapshot);
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

    return Math.max(minimumPadding, this.#typewriterBottomPaddingForRect(rect));
  });

  constructor(editor: Editor, typewriterPreferences: () => TypewriterPreferences) {
    this.#editor = editor;
    this.#typewriterPreferences = typewriterPreferences;
  }

  async #commit(): Promise<void> {
    try {
      if (this.#destroyed || this.#editor.destroyed) return;

      const request = this.#pendingRequest;
      this.#pendingRequest = null;
      if (!request) return;

      // A synchronous update may request scrolling from inside its request builder.
      // Wait until that request's applied revision has a matching visible publication.
      await tick();
      if (this.#destroyed || this.#editor.destroyed || this.#pendingRequest) return;
      const requiredRevision = this.#editor.appliedRevision;
      const publication = await this.#editor.awaitPublishedRevision(requiredRevision);
      if (publication.type !== 'published') return;
      if (this.#destroyed || this.#editor.destroyed || this.#pendingRequest) return;

      const snapshot = this.#editor.published?.snapshot;
      if (!snapshot || snapshot.revision < requiredRevision) return;
      const rects = this.#resolveTargetRects(request.target, snapshot);
      if (!rects) return;

      const mode = request.mode === 'typewriter' && this.#typewriterPreferences().enabled ? 'typewriter' : 'nearest';
      void this.bottomPadding;
      await tick();
      if (this.#destroyed || this.#editor.destroyed || this.#pendingRequest) return;

      this.#applyCommit({
        rects,
        mode,
        behavior: request.behavior,
      });
    } catch {
      // Publication cancellation, disposal, and unavailable targets make this
      // best-effort visual request obsolete.
    } finally {
      this.#commitQueued = false;
      if (!this.#destroyed && !this.#editor.destroyed && this.#pendingRequest) {
        this.scheduleCommit();
      }
    }
  }

  #resolveTargetRects(target: EditorScrollIntoViewTarget, snapshot: EditorSnapshot | undefined): PageRect[] | null {
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

  #applyCommit({ rects, mode, behavior }: { rects: PageRect[]; mode: EditorScrollRevealMode; behavior: ScrollBehavior }): void {
    const viewport = this.#editor.scrollViewport;
    if (!viewport) return;

    const viewportRect = viewport.getRect();
    const targetRect = pageRectsToClientRect(this.#editor, rects);
    if (!targetRect) return;

    const scrollTop = viewport.getScrollTop();
    const metrics = {
      scrollTop,
      clientHeight: viewportRect.bottom - viewportRect.top,
      scrollHeight: viewport.getScrollHeight(),
      targetTop: targetRect.top - viewportRect.top + scrollTop,
      targetBottom: targetRect.bottom - viewportRect.top + scrollTop,
      visibleArea: this.visibleArea,
    };
    const nextTop =
      mode === 'typewriter'
        ? resolveTypewriterScrollTop({ ...metrics, position: sanitizeTypewriterPosition(this.#typewriterPreferences().position) })
        : resolveNearestScrollTop(metrics);

    if (nextTop !== null) {
      viewport.scrollTo({ top: nextTop, behavior });
    }
  }

  #hasResolvedKeepVisibleTarget(snapshot: EditorSnapshot | undefined): boolean {
    const target = this.#keepVisibleTarget;
    return target !== null && this.#resolveTargetRects(target, snapshot) !== null;
  }

  #typewriterBottomPaddingForRect(rect: PageRect): number {
    const viewport = this.#editor.scrollViewport;
    if (!viewport) {
      return 0;
    }

    const viewportRect = viewport.getRect();
    const zoom = this.#editor.safeDisplayZoom();
    const layoutMode = this.#editor.rootAttrs?.layout_mode;
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
    this.#pendingRequest = null;
  }

  setVisibleArea(visibleArea: EditorVisibleArea): void {
    const next = sanitizeVisibleArea(visibleArea);
    if (sameVisibleArea(this.visibleArea, next)) {
      return;
    }
    this.visibleArea = next;
  }

  setBottomInset(bottomInset: number): void {
    untrack(() => {
      this.setVisibleArea({
        topInset: this.visibleArea.topInset,
        bottomInset,
      });
    });
  }

  scrollIntoView({ target, mode = 'nearest', behavior }: EditorScrollIntoViewOptions): void {
    if (this.#destroyed) {
      return;
    }

    this.#pendingRequest = {
      target,
      mode,
      behavior: behavior ?? (target.type === 'tracked_item' ? 'smooth' : 'instant'),
    };
    this.#keepVisibleTarget = target;
    this.scheduleCommit();
  }

  scheduleCommit(): void {
    if (this.#destroyed || this.#editor.hasQueuedTick || this.#commitQueued || !this.#pendingRequest) return;
    this.#commitQueued = true;

    void this.#commit();
  }
}
