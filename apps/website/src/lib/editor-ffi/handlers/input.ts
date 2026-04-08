import type { Message } from '@typie/editor-ffi/browser';
import type { EditorEventHandler } from '../types';

let commitPending = false;

export const clearCommitPending = () => {
  commitPending = false;
};

export const setCommitPending = () => {
  commitPending = true;
  setTimeout(clearCommitPending, 0);
};

const messageForInputType = (inputType: string, data: string | null): Message | undefined => {
  switch (inputType) {
    case 'insertText': {
      return data == null ? undefined : { type: 'intent', intent: { type: 'insertion', intent: { type: 'text', text: data } } };
    }

    case 'insertCompositionText': {
      return data == null
        ? undefined
        : { type: 'intent', intent: { type: 'composition', intent: { type: 'update', text: data, replace_length: undefined } } };
    }

    case 'deleteContentBackward': {
      return { type: 'key', event: { key: 'backspace' } };
    }

    case 'deleteContentForward': {
      return { type: 'key', event: { key: 'delete' } };
    }

    case 'insertLineBreak':
    case 'insertParagraph': {
      return { type: 'key', event: { key: 'enter' } };
    }

    default: {
      return undefined;
    }
  }
};

export const handleBeforeInput: EditorEventHandler<HTMLInputElement, InputEvent> = (editor, e) => {
  if (commitPending) {
    commitPending = false;
    if (e.inputType === 'insertText' || e.inputType === 'insertCompositionText') {
      e.preventDefault();
      return;
    }
  }

  const message = messageForInputType(e.inputType, e.data);
  if (message) {
    e.preventDefault();
    editor.enqueue(message);
  }
};

export const handleCompositionStart: EditorEventHandler<HTMLInputElement, CompositionEvent> = () => {
  clearCommitPending();
};

export const handleCompositionEnd: EditorEventHandler<HTMLInputElement, CompositionEvent> = (editor) => {
  editor.enqueue({ type: 'intent', intent: { type: 'composition', intent: { type: 'commit_as_is' } } });
  setCommitPending();
};
