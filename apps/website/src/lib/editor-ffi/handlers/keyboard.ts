import type { EditorEventHandler } from '../types';

export const handleKeyDown: EditorEventHandler<HTMLInputElement, KeyboardEvent> = (editor, e) => {
  if (e.key === 'ArrowRight') {
    e.preventDefault();
    editor.enqueue({
      type: 'intent',
      value: { type: 'navigation', value: { type: 'move', value: { movement: { type: 'grapheme', value: 'forward' }, extend: false } } },
    });
  } else if (e.key === 'ArrowLeft') {
    e.preventDefault();
    editor.enqueue({
      type: 'intent',
      value: { type: 'navigation', value: { type: 'move', value: { movement: { type: 'grapheme', value: 'backward' }, extend: false } } },
    });
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    editor.enqueue({
      type: 'intent',
      value: {
        type: 'navigation',
        value: { type: 'move', value: { movement: { type: 'line', value: ['backward', 'vertical'] }, extend: false } },
      },
    });
  } else if (e.key === 'ArrowDown') {
    e.preventDefault();
    editor.enqueue({
      type: 'intent',
      value: {
        type: 'navigation',
        value: { type: 'move', value: { movement: { type: 'line', value: ['forward', 'vertical'] }, extend: false } },
      },
    });
  } else if (e.key === 'Enter') {
    e.preventDefault();
    e.stopPropagation();
    editor.enqueue({ type: 'key', value: { key: 'enter' } });
  }
};
