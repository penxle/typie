import { describe, expect, it, vi } from 'vitest';
import { handleKeyDown } from './keyboard';
import type { Message } from '@typie/editor-ffi/browser';
import type { Editor } from '../editor.svelte';

const createEditor = (messages: Message[]) =>
  ({
    enqueue: vi.fn((message: Message) => {
      messages.push(message);
    }),
    scrollIntoView: vi.fn(),
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
