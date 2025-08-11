import { Extension } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { dropPoint } from '@tiptap/pm/transform';
import { mmToPx } from '../../utils';
import type { Editor } from '@tiptap/core';
import type { EditorState } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';
import type { PageLayout } from './page';

type DropCursorOptions = {
  color?: string | false;
  width?: number;
  class?: string;
};

export const DropCursor = Extension.create({
  name: 'drop_cursor',

  addProseMirrorPlugins() {
    const editor = this.editor;
    return [
      new Plugin({
        view(editorView) {
          return new DropCursorView(
            editorView,
            {
              class: 'ProseMirror-dropcursor',
              color: false,
              width: 4,
            },
            editor,
          );
        },
      }),
    ];
  },
});

class DropCursorView {
  width: number;
  color: string | undefined;
  class: string | undefined;
  cursorPos: number | null = null;
  element: HTMLElement | null = null;
  timeout = -1;
  handlers: { name: string; handler: (event: Event) => void }[];
  editorView: EditorView;
  editor: Editor;
  lastClientPos: { x: number; y: number } | null = null;

  constructor(editorView: EditorView, options: DropCursorOptions, editor: Editor) {
    this.editorView = editorView;
    this.editor = editor;
    this.width = options.width ?? 1;
    this.color = options.color === false ? undefined : options.color || 'black';
    this.class = options.class;

    this.handlers = ['dragover', 'dragend', 'drop', 'dragleave'].map((name) => {
      const handler = (e: Event) => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (this as any)[name](e);
      };
      editorView.dom.addEventListener(name, handler);
      return { name, handler };
    });
  }

  destroy() {
    this.handlers.forEach(({ name, handler }) => this.editorView.dom.removeEventListener(name, handler));
    clearTimeout(this.timeout);
    this.setCursor(null);
  }

  update(editorView: EditorView, prevState: EditorState) {
    if (this.cursorPos != null && prevState.doc != editorView.state.doc) {
      if (this.cursorPos > editorView.state.doc.content.size) this.setCursor(null);
      else this.updateOverlay();
    }
  }

  setCursor(pos: number | null) {
    if (pos === this.cursorPos && !this.lastClientPos) return;

    this.cursorPos = pos;
    if (pos == null) {
      if (this.element && this.element.parentNode) {
        this.element.remove();
        this.element = null;
      }
      this.lastClientPos = null;
    } else {
      this.updateOverlay();
    }
  }

  getPageLayout(): PageLayout | undefined {
    return this.editor.storage.page?.layout;
  }

  findPageGaps(): NodeListOf<Element> {
    // data-page-gap 속성으로 페이지 갭 찾기
    return this.editorView.dom.querySelectorAll('[data-page-gap="true"]');
  }

  updateOverlay() {
    if (this.cursorPos == null) return;

    const $pos = this.editorView.state.doc.resolve(this.cursorPos);
    const isBlock = !$pos.parent.inlineContent;
    let rect;
    const editorDOM = this.editorView.dom;
    const editorRect = editorDOM.getBoundingClientRect();
    const scaleX = editorDOM.offsetWidth ? editorRect.width / editorDOM.offsetWidth : 1;
    const scaleY = editorDOM.offsetHeight ? editorRect.height / editorDOM.offsetHeight : 1;

    if (isBlock) {
      const before = $pos.nodeBefore;
      const after = $pos.nodeAfter;
      if (before || after) {
        const node = this.editorView.nodeDOM(this.cursorPos - (before ? before.nodeSize : 0));
        if (node) {
          const nodeRect = (node as HTMLElement).getBoundingClientRect();
          let top = before ? nodeRect.bottom : nodeRect.top;
          if (before && after) {
            const afterNode = this.editorView.nodeDOM(this.cursorPos) as HTMLElement;
            if (afterNode) {
              top = (top + afterNode.getBoundingClientRect().top) / 2;
            }
          }
          const halfWidth = (this.width / 2) * scaleY;
          rect = { left: nodeRect.left, right: nodeRect.right, top: top - halfWidth, bottom: top + halfWidth };
        }
      }
    }

    if (!rect) {
      const coords = this.editorView.coordsAtPos(this.cursorPos);
      const halfWidth = (this.width / 2) * scaleX;
      rect = { left: coords.left - halfWidth, right: coords.left + halfWidth, top: coords.top, bottom: coords.bottom };
    }

    // 페이지 갭 체크 및 위치 조정
    if (this.lastClientPos) {
      const pageLayout = this.getPageLayout();
      if (pageLayout) {
        const pageGaps = this.findPageGaps();
        const marginPx = mmToPx(pageLayout.margin);

        for (const gap of pageGaps) {
          const gapRect = (gap as HTMLElement).getBoundingClientRect();
          const expandedTop = gapRect.top - marginPx;
          const expandedBottom = gapRect.bottom + marginPx;

          const cursorCenter = (rect.top + rect.bottom) / 2;
          if (cursorCenter >= expandedTop && cursorCenter <= expandedBottom) {
            const gapCenter = gapRect.top + gapRect.height / 2;
            const aboveCenter = this.lastClientPos.y < gapCenter;

            if (aboveCenter) {
              const newTop = gapRect.top - marginPx;
              const height = rect.bottom - rect.top;
              rect.top = newTop - height / 2;
              rect.bottom = newTop + height / 2;
            } else {
              const newTop = gapRect.bottom + marginPx;
              const height = rect.bottom - rect.top;
              rect.top = newTop - height / 2;
              rect.bottom = newTop + height / 2;
            }
            break;
          }
        }
      }
    }

    const parent = this.editorView.dom.offsetParent as HTMLElement;
    if (!this.element) {
      this.element = document.createElement('div');
      if (this.class) this.element.className = this.class;
      this.element.style.cssText = 'position: absolute; z-index: 50; pointer-events: none;';
      if (this.color) {
        this.element.style.backgroundColor = this.color;
      }
      parent.append(this.element);
    }

    this.element.classList.toggle('prosemirror-dropcursor-block', isBlock);
    this.element.classList.toggle('prosemirror-dropcursor-inline', !isBlock);

    let parentLeft;
    let parentTop;
    if (!parent || (parent == document.body && getComputedStyle(parent).position == 'static')) {
      parentLeft = -window.pageXOffset;
      parentTop = -window.pageYOffset;
    } else {
      const parentRect = parent.getBoundingClientRect();
      const parentScaleX = parentRect.width / parent.offsetWidth;
      const parentScaleY = parentRect.height / parent.offsetHeight;
      parentLeft = parentRect.left - parent.scrollLeft * parentScaleX;
      parentTop = parentRect.top - parent.scrollTop * parentScaleY;
    }

    this.element.style.left = (rect.left - parentLeft) / scaleX + 'px';
    this.element.style.top = (rect.top - parentTop) / scaleY + 'px';
    this.element.style.width = (rect.right - rect.left) / scaleX + 'px';
    this.element.style.height = (rect.bottom - rect.top) / scaleY + 'px';
  }

  scheduleRemoval(timeout: number) {
    clearTimeout(this.timeout);
    this.timeout = window.setTimeout(() => this.setCursor(null), timeout);
  }

  dragover(event: DragEvent) {
    if (!this.editorView.editable) return;

    if (this.lastClientPos && this.lastClientPos.x === event.clientX && this.lastClientPos.y === event.clientY) {
      return;
    }

    const pos = this.editorView.posAtCoords({ left: event.clientX, top: event.clientY });

    const node = pos && pos.inside >= 0 && this.editorView.state.doc.nodeAt(pos.inside);
    const disableDropCursor = node && node.type.spec.disableDropCursor;
    const disabled = typeof disableDropCursor == 'function' ? disableDropCursor(this.editorView, pos, event) : disableDropCursor;

    if (pos && !disabled) {
      let target = pos.pos;
      if (this.editorView.dragging && this.editorView.dragging.slice) {
        const point = dropPoint(this.editorView.state.doc, target, this.editorView.dragging.slice);
        if (point != null) target = point;
      }

      this.lastClientPos = { x: event.clientX, y: event.clientY };
      this.setCursor(target);
      this.scheduleRemoval(5000);
    }
  }

  dragend() {
    this.lastClientPos = null;
    this.scheduleRemoval(20);
  }

  drop() {
    this.lastClientPos = null;
    this.scheduleRemoval(20);
  }

  dragleave(event: DragEvent) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    if (!this.editorView.dom.contains((event as any).relatedTarget)) {
      this.lastClientPos = null;
      this.setCursor(null);
    }
  }
}
