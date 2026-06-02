import { getAppContext } from '@typie/ui/context';
import { tick, untrack } from 'svelte';
import { PAGE_GAP } from './constants';
import {
  resolveDistanceToPagesBottom,
  resolveKeepVisibleBottomPadding,
  resolveNearestScrollTop,
  resolveTypewriterBottomPadding,
  resolveTypewriterScrollTop,
} from './scroll';
import type { PageRect } from '@typie/editor-ffi/browser';
import type { Editor, EditorContext } from './editor.svelte';
import type { EditorVisibleArea, RevealTargetSpan } from './scroll';

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

    const scope = new EditorScrollScope(editor, () => ({
      enabled: app.preference.current.typewriterEnabled,
      position: app.preference.current.typewriterPosition,
    }));
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

    void editor.tickRevision;
    void editor.viewport.height;

    untrack(() => scroll.scheduleCommit());
  });
}

export class EditorScrollScope {
  visibleArea = $state<EditorVisibleArea>(DEFAULT_VISIBLE_AREA);

  #pendingRequest: PendingScrollRequest | null = null;
  #keepVisibleTarget = $state<EditorScrollIntoViewTarget | null>(null);
  #commitQueued = false;
  #destroyed = false;
  readonly #editor: Editor;
  readonly #typewriterPreferences: () => TypewriterPreferences;

  bottomPadding = $derived.by(() => {
    void this.#editor.viewport.height;
    const keepVisiblePadding = this.#keepVisibleBottomPadding();
    const rect = this.#editor.selectionHeadRect();
    if (!rect) {
      return keepVisiblePadding;
    }

    const prefs = this.#typewriterPreferences();
    if (!prefs.enabled) {
      return keepVisiblePadding;
    }

    return Math.max(keepVisiblePadding, this.#typewriterBottomPaddingForRect(rect));
  });

  constructor(editor: Editor, typewriterPreferences: () => TypewriterPreferences) {
    this.#editor = editor;
    this.#typewriterPreferences = typewriterPreferences;
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

  async #commit(): Promise<void> {
    try {
      if (this.#destroyed || this.#editor.destroyed) return;

      const request = this.#pendingRequest;
      this.#pendingRequest = null;
      if (!request) return;

      const rect = this.#resolveTargetRect(request.target);
      if (!rect) return;

      const mode = request.mode === 'typewriter' && this.#typewriterPreferences().enabled ? 'typewriter' : 'nearest';
      void this.bottomPadding;
      await tick();

      this.#applyCommit({
        rect,
        mode,
        behavior: request.behavior,
      });
    } finally {
      this.#commitQueued = false;
      if (!this.#destroyed && !this.#editor.destroyed && this.#pendingRequest) {
        this.scheduleCommit();
      }
    }
  }

  #resolveTargetRect(target: EditorScrollIntoViewTarget): PageRect | null {
    switch (target.type) {
      case 'current_selection_head': {
        return this.#editor.selectionHeadRect();
      }
      case 'tracked_item': {
        return this.#editor.trackedItemRect(target.id);
      }
    }
  }

  #applyCommit({ rect, mode, behavior }: { rect: PageRect; mode: EditorScrollRevealMode; behavior: ScrollBehavior }): void {
    const container = this.#editor.scrollContainerEl;
    if (!container) return;

    const span = this.#pageRectToScrollSpan(rect);
    if (!span) return;

    const metrics = {
      scrollTop: container.scrollTop,
      clientHeight: container.clientHeight,
      scrollHeight: container.scrollHeight,
      ...span,
      visibleArea: this.visibleArea,
    };
    const nextTop =
      mode === 'typewriter'
        ? resolveTypewriterScrollTop({ ...metrics, position: sanitizeTypewriterPosition(this.#typewriterPreferences().position) })
        : resolveNearestScrollTop(metrics);

    if (nextTop !== null) {
      container.scrollTo({ top: nextTop, behavior });
    }
  }

  #pageRectToScrollSpan({ page_idx, rect }: PageRect): RevealTargetSpan | null {
    const pageEl = this.#editor.pageEls[page_idx];
    const container = this.#editor.scrollContainerEl;
    if (!pageEl || !container) return null;

    const zoom = this.#editor.safeDisplayZoom();
    const pageRect = pageEl.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    const targetTop = pageRect.top - containerRect.top + container.scrollTop + rect.y * zoom;
    const targetBottom = targetTop + rect.height * zoom;
    return { targetTop, targetBottom };
  }

  #distanceToPagesBottom({ page_idx }: PageRect, targetY: number): number | null {
    return resolveDistanceToPagesBottom({
      pageSizes: this.#editor.pageSizes,
      pageIdx: page_idx,
      targetY,
      displayZoom: this.#editor.safeDisplayZoom(),
      pageGap: this.#editor.rootAttrs?.layout_mode.type === 'paginated' ? PAGE_GAP : 0,
    });
  }

  #keepVisibleBottomPadding(): number {
    const target = this.#keepVisibleTarget;
    if (!target) {
      return 0;
    }

    const rect = this.#resolveTargetRect(target);
    const distanceToPagesBottom = rect ? this.#distanceToPagesBottom(rect, rect.rect.y + rect.rect.height) : null;
    if (distanceToPagesBottom === null) {
      return 0;
    }

    return resolveKeepVisibleBottomPadding({
      distanceToContentBottom: distanceToPagesBottom,
      visibleArea: this.visibleArea,
    });
  }

  #typewriterBottomPaddingForRect(rect: PageRect): number {
    const container = this.#editor.scrollContainerEl;
    const distanceToPagesBottom = this.#distanceToPagesBottom(rect, rect.rect.y);
    if (!container || distanceToPagesBottom === null) {
      return 0;
    }

    const zoom = this.#editor.safeDisplayZoom();
    return resolveTypewriterBottomPadding({
      clientHeight: container.clientHeight,
      targetHeight: rect.rect.height * zoom,
      distanceToContentBottom: distanceToPagesBottom,
      visibleArea: this.visibleArea,
      position: sanitizeTypewriterPosition(this.#typewriterPreferences().position),
    });
  }
}
