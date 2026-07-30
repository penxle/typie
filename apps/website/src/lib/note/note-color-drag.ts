import { pointerCapture } from '@typie/ui/actions';

export type NoteColorDragOptions = {
  onchange: (color: string) => void;
};

export type NoteColorDragHandle = {
  update: (options: NoteColorDragOptions) => void;
  destroy: () => void;
};

type DragSession = {
  color: string;
};

export function createNoteColorDrag(container: HTMLElement, initialOptions: NoteColorDragOptions): NoteColorDragHandle {
  let options = initialOptions;

  const colorAt = (target: EventTarget | null): string | null => {
    if (!(target instanceof Element)) return null;

    const colorElement = target.closest<HTMLElement>('[data-note-color-value]');
    if (!colorElement || !container.contains(colorElement)) return null;
    return colorElement.dataset.noteColorValue ?? null;
  };

  const capture = pointerCapture<DragSession>(container, {
    start(event) {
      if (event.button !== 0 || !event.isPrimary) return null;

      const color = colorAt(event.target);
      if (color === null) return null;

      options.onchange(color);
      return { color };
    },
    move(session, event) {
      const color = colorAt(document.elementFromPoint(event.clientX, event.clientY));
      if (color === null || color === session.color) return;

      session.color = color;
      options.onchange(color);
    },
  });

  return {
    update(nextOptions) {
      options = nextOptions;
    },
    destroy() {
      capture.destroy();
    },
  };
}
