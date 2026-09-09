import type { Message } from '@typie/editor-ffi/browser';

export const isMutatingMessage = (message: Message): boolean => {
  switch (message.type) {
    case 'selection':
    case 'view':
    case 'navigation':
    case 'system': {
      return false;
    }
    case 'tracked_range': {
      return message.op.type === 'replace_text';
    }
    case 'key': {
      return message.event.key !== 'escape';
    }
    case 'text_input': {
      // Ending preedit removes IME state without changing the document.
      return message.ops.some((op) => op.type !== 'clear_composition');
    }
    case 'insertion':
    case 'deletion':
    case 'modifier':
    case 'node':
    case 'dnd':
    case 'history':
    case 'clipboard': {
      return true;
    }
    default: {
      return true;
    }
  }
};
