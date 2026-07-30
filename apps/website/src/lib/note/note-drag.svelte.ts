import { token } from '@typie/styled-system/tokens';
import { thresholdDrag } from '@typie/ui/actions';
import { pushEscapeHandler } from '@typie/ui/utils';
import type { ActionReturn } from 'svelte/action';
import type { NoteReorderDirection } from '$lib/note-reorder';
import type { NoteListDragPosition } from './NoteList.svelte';

export type NoteDragOptions = {
  disabled?: boolean;
  dragging: boolean;
  onDragStart: (pointer: { clientX: number; clientY: number }) => boolean;
  onDragMove: (position: NoteListDragPosition) => void;
  onDragEnd: () => void;
  onDragCancel: () => void;
  onPress?: () => void;
};

type NoteDragSession = {
  element: HTMLElement;
  rect: DOMRect;
  offsetX: number;
  offsetY: number;
  ghost: HTMLElement | null;
  cursorStyle: HTMLStyleElement | null;
  removeEscapeHandler: (() => void) | null;
  previousCenterY: number;
  direction: NoteReorderDirection;
};

const DRAG_THRESHOLD = 5;
const DRAG_DIRECTION_EPSILON = 0.5;

const cleanup = (session: NoteDragSession) => {
  session.ghost?.remove();
  session.ghost = null;
  session.cursorStyle?.remove();
  session.cursorStyle = null;
  session.removeEscapeHandler?.();
  session.removeEscapeHandler = null;
};

const updateGhost = (session: NoteDragSession, event: PointerEvent, onDragMove: NoteDragOptions['onDragMove']) => {
  if (!session.ghost) return;

  const top = event.clientY - session.offsetY;
  const centerY = top + session.rect.height / 2;
  const centerDeltaY = centerY - session.previousCenterY;

  session.ghost.style.left = `${event.clientX - session.offsetX}px`;
  session.ghost.style.top = `${top}px`;
  session.previousCenterY = centerY;

  if (Math.abs(centerDeltaY) > DRAG_DIRECTION_EPSILON) {
    session.direction = centerDeltaY < 0 ? -1 : 1;
  }

  onDragMove({
    clientX: event.clientX,
    clientY: event.clientY,
    direction: session.direction,
    ghost: {
      top,
      bottom: top + session.rect.height,
    },
  });
};

const createGhost = (session: NoteDragSession, event: PointerEvent) => {
  const ghost = document.createElement('div');
  const cloned = session.element.cloneNode(true) as HTMLElement;
  (cloned as unknown as { inert: boolean }).inert = true;
  cloned.setAttribute('aria-hidden', 'true');
  cloned.style.pointerEvents = 'none';
  cloned.style.transform = 'rotate(1.5deg) scale(1.05)';
  cloned.style.opacity = '0.8';
  cloned.style.width = '100%';
  cloned.style.height = '100%';
  ghost.append(cloned);

  ghost.style.position = 'fixed';
  ghost.style.pointerEvents = 'none';
  ghost.style.zIndex = token('zIndex.ghost');
  ghost.style.width = `${session.rect.width}px`;
  ghost.style.height = `${session.rect.height}px`;
  ghost.style.left = `${event.clientX - session.offsetX}px`;
  ghost.style.top = `${event.clientY - session.offsetY}px`;
  document.body.append(ghost);
  session.ghost = ghost;

  const cursorStyle = document.createElement('style');
  cursorStyle.textContent = '* { cursor: grabbing !important; }';
  document.head.append(cursorStyle);
  session.cursorStyle = cursorStyle;
};

export const noteDrag = (element: HTMLElement, initialOptions: NoteDragOptions): ActionReturn<NoteDragOptions> => {
  let options = initialOptions;
  const drag = thresholdDrag<NoteDragSession>(
    element,
    {
      threshold: DRAG_THRESHOLD,
      start: (event) => {
        if (options.disabled || !event.isPrimary || event.button !== 0) return null;
        const target = event.target as HTMLElement;
        if (target.closest('button, textarea, a')) return null;

        event.preventDefault();
        const rect = element.getBoundingClientRect();
        return {
          element,
          rect,
          offsetX: event.clientX - rect.left,
          offsetY: event.clientY - rect.top,
          ghost: null,
          cursorStyle: null,
          removeEscapeHandler: null,
          previousCenterY: rect.top + rect.height / 2,
          direction: 0,
        };
      },
      activate: (session, _startEvent, event, controls) => {
        if (!options.onDragStart({ clientX: event.clientX, clientY: event.clientY })) return false;

        createGhost(session, event);
        session.removeEscapeHandler = pushEscapeHandler(() => {
          controls.cancel();
          return true;
        });
        return true;
      },
      move: (session, event) => updateGhost(session, event, options.onDragMove),
      press: (session) => {
        cleanup(session);
        options.onPress?.();
      },
      end: (session, event) => {
        updateGhost(session, event, options.onDragMove);
        cleanup(session);
        options.onDragEnd();
      },
      cancel: (session, _reason, wasActive) => {
        cleanup(session);
        if (wasActive) options.onDragCancel();
      },
    },
    document.documentElement,
  );

  return {
    update(nextOptions) {
      options = nextOptions;
      if (options.disabled || (!options.dragging && drag.state().active)) {
        drag.cancel();
      }
    },
    destroy: drag.destroy,
  };
};
