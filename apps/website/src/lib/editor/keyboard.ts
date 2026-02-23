import { IS_MAC } from './constants';
import type { Editor } from './editor.svelte';

export const handleKeyEvent = (editor: Editor, e: KeyboardEvent): boolean => {
  const wordModifier = IS_MAC ? e.altKey : !IS_MAC && e.ctrlKey;

  switch (e.key) {
    case 'ArrowLeft': {
      if (IS_MAC && e.metaKey) {
        editor.dispatch({ type: 'navigate', direction: 'lineStart', extend: e.shiftKey });
      } else if (wordModifier) {
        editor.dispatch({ type: 'navigate', direction: 'wordLeft', extend: e.shiftKey });
      } else {
        editor.dispatch({ type: 'navigate', direction: 'left', extend: e.shiftKey });
      }

      editor.scrollIntoView({ mode: e.shiftKey ? 'auto' : 'typewriter' });
      return true;
    }
    case 'ArrowRight': {
      if (IS_MAC && e.metaKey) {
        editor.dispatch({ type: 'navigate', direction: 'lineEnd', extend: e.shiftKey });
      } else if (wordModifier) {
        editor.dispatch({ type: 'navigate', direction: 'wordRight', extend: e.shiftKey });
      } else {
        editor.dispatch({ type: 'navigate', direction: 'right', extend: e.shiftKey });
      }

      editor.scrollIntoView({ mode: e.shiftKey ? 'auto' : 'typewriter' });
      return true;
    }
    case 'ArrowUp': {
      if (IS_MAC && e.metaKey) {
        editor.dispatch({ type: 'navigate', direction: 'documentStart', extend: e.shiftKey });
      } else if (e.altKey) {
        editor.dispatch({ type: 'navigate', direction: 'sentenceUp', extend: e.shiftKey });
      } else {
        editor.dispatch({ type: 'navigate', direction: 'up', extend: e.shiftKey });
      }

      editor.scrollIntoView({ mode: e.shiftKey ? 'auto' : 'typewriter' });
      return true;
    }
    case 'ArrowDown': {
      if (IS_MAC && e.metaKey) {
        editor.dispatch({ type: 'navigate', direction: 'documentEnd', extend: e.shiftKey });
      } else if (e.altKey) {
        editor.dispatch({ type: 'navigate', direction: 'sentenceDown', extend: e.shiftKey });
      } else {
        editor.dispatch({ type: 'navigate', direction: 'down', extend: e.shiftKey });
      }

      editor.scrollIntoView({ mode: e.shiftKey ? 'auto' : 'typewriter' });
      return true;
    }
    case 'Home': {
      if (!IS_MAC && e.ctrlKey) {
        editor.dispatch({ type: 'navigate', direction: 'documentStart', extend: e.shiftKey });
      } else {
        editor.dispatch({ type: 'navigate', direction: 'lineStart', extend: e.shiftKey });
      }

      editor.scrollIntoView({ mode: e.shiftKey ? 'auto' : 'typewriter' });
      return true;
    }
    case 'End': {
      if (!IS_MAC && e.ctrlKey) {
        editor.dispatch({ type: 'navigate', direction: 'documentEnd', extend: e.shiftKey });
      } else {
        editor.dispatch({ type: 'navigate', direction: 'lineEnd', extend: e.shiftKey });
      }

      editor.scrollIntoView({ mode: e.shiftKey ? 'auto' : 'typewriter' });
      return true;
    }
    case 'PageUp': {
      editor.dispatch({ type: 'navigate', direction: 'pageUp', extend: e.shiftKey });
      editor.scrollIntoView({ mode: e.shiftKey ? 'auto' : 'typewriter' });
      return true;
    }
    case 'PageDown': {
      editor.dispatch({ type: 'navigate', direction: 'pageDown', extend: e.shiftKey });
      editor.scrollIntoView({ mode: e.shiftKey ? 'auto' : 'typewriter' });
      return true;
    }
    case 'Backspace': {
      if (IS_MAC && e.metaKey) {
        editor.dispatch({ type: 'deleteToLineStart' });
      } else if (wordModifier) {
        editor.dispatch({ type: 'deleteWordBackward' });
      } else {
        editor.dispatch({ type: 'deleteBackward' });
      }

      editor.scrollIntoView({ mode: 'typewriter' });
      return true;
    }
    case 'Delete': {
      if (wordModifier) {
        editor.dispatch({ type: 'deleteWordForward' });
      } else {
        editor.dispatch({ type: 'deleteForward' });
      }

      editor.scrollIntoView({ mode: 'typewriter' });
      return true;
    }
    case 'Enter': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        editor.dispatch({ type: 'insertPageBreak' });
      } else if (e.shiftKey) {
        editor.dispatch({ type: 'insertHardBreak' });
      } else {
        editor.dispatch({ type: 'insertNewline' });
      }

      editor.scrollIntoView({ mode: 'typewriter' });
      return true;
    }
    case 'a':
    case 'A': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        editor.markSelectAllShortcut();
        editor.dispatch({ type: 'selectAll' });
        return true;
      }
      break;
    }
    case 'b':
    case 'B': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        editor.dispatch({ type: 'toggleBold' }).scrollIntoView();
        return true;
      }
      break;
    }
    case 'i':
    case 'I': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        editor.dispatch({ type: 'toggleStyle', style: { type: 'italic' } }).scrollIntoView();
        return true;
      }
      break;
    }
    case 'u':
    case 'U': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        editor.dispatch({ type: 'toggleStyle', style: { type: 'underline' } }).scrollIntoView();
        return true;
      }
      break;
    }
    case 's':
    case 'S': {
      if ((e.shiftKey && IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        editor.dispatch({ type: 'toggleStyle', style: { type: 'strikethrough' } }).scrollIntoView();
        return true;
      }
      break;
    }
    case 'z':
    case 'Z': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        if (e.shiftKey) {
          editor.dispatch({ type: 'redo' }).scrollIntoView();
        } else {
          editor.dispatch({ type: 'undo' }).scrollIntoView();
        }
        return true;
      }
      break;
    }
    case '\\': {
      if ((IS_MAC && e.metaKey) || (!IS_MAC && e.ctrlKey)) {
        editor.dispatch({ type: 'clearFormatting' }).scrollIntoView();
        return true;
      }
      break;
    }
    case 'Tab': {
      if (e.shiftKey) {
        editor.dispatch({ type: 'outdent' });
      } else {
        editor.dispatch({ type: 'indent' });
      }

      editor.scrollIntoView({ mode: 'typewriter' });
      return true;
    }
    case 'Escape': {
      editor.dispatch({ type: 'escape' }).scrollIntoView();
      return true;
    }
  }

  return false;
};
