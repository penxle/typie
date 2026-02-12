import { IS_MAC } from './constants';
import type { Direction, Message } from './types';

const nav = (direction: Direction, extend: boolean): Message => ({
  type: 'navigate',
  direction,
  extend,
});

export const getActionFromKeyEvent = (e: KeyboardEvent): Message | null => {
  const wordModifier = IS_MAC ? e.altKey : !IS_MAC && e.ctrlKey;

  switch (e.key) {
    case 'ArrowLeft': {
      if (IS_MAC && e.metaKey) {
        return nav('lineStart', e.shiftKey);
      } else if (wordModifier) {
        return nav('wordLeft', e.shiftKey);
      } else {
        return nav('left', e.shiftKey);
      }
    }
    case 'ArrowRight': {
      if (IS_MAC && e.metaKey) {
        return nav('lineEnd', e.shiftKey);
      } else if (wordModifier) {
        return nav('wordRight', e.shiftKey);
      } else {
        return nav('right', e.shiftKey);
      }
    }
    case 'ArrowUp': {
      if (IS_MAC && e.metaKey) {
        return nav('documentStart', e.shiftKey);
      } else if (e.altKey) {
        return nav('sentenceUp', e.shiftKey);
      } else {
        return nav('up', e.shiftKey);
      }
    }
    case 'ArrowDown': {
      if (IS_MAC && e.metaKey) {
        return nav('documentEnd', e.shiftKey);
      } else if (e.altKey) {
        return nav('sentenceDown', e.shiftKey);
      } else {
        return nav('down', e.shiftKey);
      }
    }
    case 'Home': {
      if (!IS_MAC && e.ctrlKey) {
        return nav('documentStart', e.shiftKey);
      } else {
        return nav('lineStart', e.shiftKey);
      }
    }
    case 'End': {
      if (!IS_MAC && e.ctrlKey) {
        return nav('documentEnd', e.shiftKey);
      } else {
        return nav('lineEnd', e.shiftKey);
      }
    }
    case 'PageUp': {
      return nav('pageUp', e.shiftKey);
    }
    case 'PageDown': {
      return nav('pageDown', e.shiftKey);
    }
    case 'Backspace': {
      if (IS_MAC && e.metaKey) {
        return { type: 'deleteToLineStart' };
      } else if (wordModifier) {
        return { type: 'deleteWordBackward' };
      } else {
        return { type: 'deleteBackward' };
      }
    }
    case 'Delete': {
      if (wordModifier) {
        return { type: 'deleteWordForward' };
      } else {
        return { type: 'deleteForward' };
      }
    }
    case 'Enter': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        return { type: 'insertPageBreak' };
      } else if (e.shiftKey) {
        return { type: 'insertHardBreak' };
      } else {
        return { type: 'insertNewline' };
      }
    }
    case 'a':
    case 'A': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        return { type: 'selectAll' };
      }
      break;
    }
    case 'b':
    case 'B': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        return { type: 'toggleBold' };
      }
      break;
    }
    case 'i':
    case 'I': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        return { type: 'toggleStyle', style: { type: 'italic' } };
      }
      break;
    }
    case 'u':
    case 'U': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        return { type: 'toggleStyle', style: { type: 'underline' } };
      }
      break;
    }
    case 's':
    case 'S': {
      if ((e.shiftKey && IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        return { type: 'toggleStyle', style: { type: 'strikethrough' } };
      }
      break;
    }
    case 'z':
    case 'Z': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        if (e.shiftKey) {
          return { type: 'redo' };
        } else {
          return { type: 'undo' };
        }
      }
      break;
    }
    case '\\': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        return { type: 'clearFormatting' };
      }
      break;
    }
    case 'Tab': {
      if (e.shiftKey) {
        return { type: 'outdent' };
      } else {
        return { type: 'indent' };
      }
    }
    case 'Escape': {
      return { type: 'escape' };
    }
  }

  return null;
};
