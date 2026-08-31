import { pushEscapeHandler } from '@typie/ui/utils';
import { describe, expect, it, vi } from 'vitest';
import { handleKeyDown } from './keyboard';
import type { Message } from '@typie/editor-ffi/browser';
import type { Editor } from '../editor.svelte';

const createEditor = (messages: Message[], escapeKeyHandler: (() => boolean) | null = null) =>
  ({
    enqueue: vi.fn((message: Message) => {
      messages.push(message);
    }),
    escapeKeyHandler,
    scrollIntoView: vi.fn(),
    updateNow: vi.fn((build: () => void) => {
      build();
      return null;
    }),
  }) as unknown as Editor;

type TestKeyboardEvent = KeyboardEvent & { currentTarget: HTMLTextAreaElement };

const composingEvent = (
  key: string,
  modifiers: { shift?: boolean; mod?: boolean; ctrl?: boolean; alt?: boolean } = {},
): TestKeyboardEvent => {
  const isMac = navigator.platform.toUpperCase().includes('MAC');
  return {
    key,
    isComposing: true,
    shiftKey: modifiers.shift ?? false,
    altKey: modifiers.alt ?? false,
    ctrlKey: modifiers.ctrl ?? (modifiers.mod === true && !isMac),
    metaKey: modifiers.mod === true && isMac,
    currentTarget: document.createElement('textarea'),
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  } as unknown as TestKeyboardEvent;
};

describe('handleKeyDown', () => {
  it('admits a hardware-key mutation and its reveal in one synchronous update', () => {
    const messages: Message[] = [];
    const editor = createEditor(messages);
    const event = { ...composingEvent('Enter'), isComposing: false } as TestKeyboardEvent;

    editor.updateNow(() => handleKeyDown(editor, event));

    expect(editor.updateNow).toHaveBeenCalledOnce();
    expect(messages).toEqual([{ type: 'key', event: { key: 'enter' } }]);
    expect(editor.scrollIntoView).toHaveBeenCalledWith({ target: { type: 'current_selection_head' }, policy: 'typewriter' });
  });

  it('leaves composing navigation to the native post-composition keydown', () => {
    const messages: Message[] = [];
    const editor = createEditor(messages);
    const composing = composingEvent('ArrowLeft');

    expect(handleKeyDown(editor, composing)).toBeUndefined();
    expect(messages).toEqual([]);
    expect(composing.preventDefault).not.toHaveBeenCalled();
    expect(composing.stopPropagation).not.toHaveBeenCalled();

    const postComposition = { ...composing, isComposing: false } as TestKeyboardEvent;
    handleKeyDown(editor, postComposition);

    expect(messages).toEqual([
      {
        type: 'navigation',
        op: {
          type: 'move',
          movement: { type: 'grapheme', direction: 'backward' },
          extend: false,
        },
      },
    ]);
    expect(postComposition.preventDefault).toHaveBeenCalledOnce();
    expect(postComposition.stopPropagation).toHaveBeenCalledOnce();
  });

  it('defers and consumes a composing editor shortcut', () => {
    const messages: Message[] = [];
    const editor = createEditor(messages);
    const event = composingEvent('b', { mod: true });

    const pending = handleKeyDown(editor, event);

    expect(pending).toBeTypeOf('function');
    expect(messages).toEqual([]);
    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(event.stopPropagation).toHaveBeenCalledOnce();

    messages.push({ type: 'text_input', ops: [{ type: 'commit_as_is' }] });
    pending?.();

    expect(messages).toEqual([
      { type: 'text_input', ops: [{ type: 'commit_as_is' }] },
      { type: 'modifier', op: { type: 'toggle', modifier_type: 'bold' } },
    ]);
  });

  it.each(['Tab', 'Escape'])('leaves composing %s to native text input', (key) => {
    const messages: Message[] = [];
    const editor = createEditor(messages);
    const event = composingEvent(key);

    expect(handleKeyDown(editor, event)).toBeUndefined();
    expect(messages).toEqual([]);
    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(event.stopPropagation).not.toHaveBeenCalled();
  });

  it('leaves composing Escape before the document Escape handler', () => {
    const messages: Message[] = [];
    const escapeKeyHandler = vi.fn(() => true);
    const editor = createEditor(messages, escapeKeyHandler);
    const event = composingEvent('Escape');

    handleKeyDown(editor, event);

    expect(escapeKeyHandler).not.toHaveBeenCalled();
    expect(messages).toEqual([]);
  });

  it('lets the document consume Escape before the core binding', () => {
    const messages: Message[] = [];
    const escapeKeyHandler = vi.fn(() => true);
    const editor = createEditor(messages, escapeKeyHandler);
    const event = { ...composingEvent('Escape'), isComposing: false } as TestKeyboardEvent;

    handleKeyDown(editor, event);

    expect(escapeKeyHandler).toHaveBeenCalledOnce();
    expect(messages).toEqual([]);
    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(event.stopPropagation).toHaveBeenCalledOnce();
  });

  it('lets the topmost transient UI consume Escape before the document handler', () => {
    const messages: Message[] = [];
    const transientHandler = vi.fn(() => true);
    const removeTransientHandler = pushEscapeHandler(transientHandler);
    const escapeKeyHandler = vi.fn(() => true);
    const editor = createEditor(messages, escapeKeyHandler);
    const event = { ...composingEvent('Escape'), isComposing: false } as TestKeyboardEvent;

    try {
      handleKeyDown(editor, event);

      expect(transientHandler).toHaveBeenCalledOnce();
      expect(escapeKeyHandler).not.toHaveBeenCalled();
      expect(messages).toEqual([]);
    } finally {
      removeTransientHandler();
    }
  });

  it('runs the core Escape binding when the document declines it', () => {
    const messages: Message[] = [];
    const escapeKeyHandler = vi.fn(() => false);
    const editor = createEditor(messages, escapeKeyHandler);
    const event = { ...composingEvent('Escape'), isComposing: false } as TestKeyboardEvent;

    handleKeyDown(editor, event);

    expect(escapeKeyHandler).toHaveBeenCalledOnce();
    expect(messages).toEqual([{ type: 'key', event: { key: 'escape' } }]);
    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(event.stopPropagation).toHaveBeenCalledOnce();
  });

  it('defers Mod+Enter until after the active composition is committed', () => {
    const messages: Message[] = [];
    const editor = createEditor(messages);
    const event = composingEvent('Enter', { mod: true });

    const pending = handleKeyDown(editor, event);

    expect(pending).toBeTypeOf('function');
    expect(messages).toEqual([]);
    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(event.stopPropagation).toHaveBeenCalledOnce();

    messages.push({ type: 'text_input', ops: [{ type: 'commit_as_is' }] });
    pending?.();

    expect(messages).toEqual([
      { type: 'text_input', ops: [{ type: 'commit_as_is' }] },
      { type: 'insertion', op: { type: 'break', kind: 'page' } },
    ]);
  });
});
