import type { Message, Movement } from '@typie/editor-ffi/browser';
import type { Editor } from '../editor.svelte';
import type { EditorEventHandler } from '../types';

type KeyBindingModifier = 'shift' | 'mod' | 'ctrl' | 'alt';

type KeyBinding = {
  key: string;
  modifiers?: KeyBindingModifier[];
  predicate?: (e: KeyboardEvent) => boolean;
  action: (editor: Editor, e: KeyboardEvent) => void;
};

const bindings: KeyBinding[] = [
  { key: 'ArrowLeft', action: (ed) => ed.enqueue(move({ type: 'grapheme', direction: 'backward' }, false)) },
  { key: 'ArrowLeft', modifiers: ['shift'], action: (ed) => ed.enqueue(move({ type: 'grapheme', direction: 'backward' }, true)) },
  { key: 'ArrowLeft', modifiers: ['alt'], action: (ed) => ed.enqueue(move({ type: 'word', direction: 'backward' }, false)) },
  { key: 'ArrowLeft', modifiers: ['shift', 'alt'], action: (ed) => ed.enqueue(move({ type: 'word', direction: 'backward' }, true)) },
  {
    key: 'ArrowLeft',
    modifiers: ['mod'],
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'backward', axis: 'horizontal' }, false)),
  },
  {
    key: 'ArrowLeft',
    modifiers: ['shift', 'mod'],
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'backward', axis: 'horizontal' }, true)),
  },

  { key: 'ArrowRight', action: (ed) => ed.enqueue(move({ type: 'grapheme', direction: 'forward' }, false)) },
  { key: 'ArrowRight', modifiers: ['shift'], action: (ed) => ed.enqueue(move({ type: 'grapheme', direction: 'forward' }, true)) },
  { key: 'ArrowRight', modifiers: ['alt'], action: (ed) => ed.enqueue(move({ type: 'word', direction: 'forward' }, false)) },
  { key: 'ArrowRight', modifiers: ['shift', 'alt'], action: (ed) => ed.enqueue(move({ type: 'word', direction: 'forward' }, true)) },
  {
    key: 'ArrowRight',
    modifiers: ['mod'],
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'forward', axis: 'horizontal' }, false)),
  },
  {
    key: 'ArrowRight',
    modifiers: ['shift', 'mod'],
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'forward', axis: 'horizontal' }, true)),
  },

  { key: 'ArrowUp', action: (ed) => ed.enqueue(move({ type: 'line', direction: 'backward', axis: 'vertical' }, false)) },
  {
    key: 'ArrowUp',
    modifiers: ['shift'],
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'backward', axis: 'vertical' }, true)),
  },
  { key: 'ArrowUp', modifiers: ['mod'], action: (ed) => ed.enqueue(move({ type: 'document', direction: 'backward' }, false)) },
  { key: 'ArrowUp', modifiers: ['shift', 'mod'], action: (ed) => ed.enqueue(move({ type: 'document', direction: 'backward' }, true)) },

  { key: 'ArrowDown', action: (ed) => ed.enqueue(move({ type: 'line', direction: 'forward', axis: 'vertical' }, false)) },
  {
    key: 'ArrowDown',
    modifiers: ['shift'],
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'forward', axis: 'vertical' }, true)),
  },
  { key: 'ArrowDown', modifiers: ['mod'], action: (ed) => ed.enqueue(move({ type: 'document', direction: 'forward' }, false)) },
  { key: 'ArrowDown', modifiers: ['shift', 'mod'], action: (ed) => ed.enqueue(move({ type: 'document', direction: 'forward' }, true)) },

  { key: 'Enter', action: (ed) => ed.enqueue({ type: 'key', event: { key: 'enter' } }) },
  { key: 'Backspace', action: (ed) => ed.enqueue({ type: 'key', event: { key: 'backspace' } }) },

  {
    key: 'a',
    modifiers: ['mod'],
    action: (ed) => ed.enqueue({ type: 'selection', op: { type: 'all' } }),
  },

  {
    key: 'b',
    modifiers: ['mod'],
    action: (ed) => ed.enqueue({ type: 'modifier', op: { type: 'toggle', modifier_type: 'bold' } }),
  },
  {
    key: 'i',
    modifiers: ['mod'],
    action: (ed) => ed.enqueue({ type: 'modifier', op: { type: 'toggle', modifier_type: 'italic' } }),
  },
  {
    key: 's',
    modifiers: ['mod', 'shift'],
    action: (ed) => ed.enqueue({ type: 'modifier', op: { type: 'toggle', modifier_type: 'strikethrough' } }),
  },
  {
    key: 'u',
    modifiers: ['mod', 'shift'],
    action: (ed) => ed.enqueue({ type: 'modifier', op: { type: 'toggle', modifier_type: 'underline' } }),
  },

  { key: 'q', modifiers: ['ctrl'], predicate: () => isMac, action: (ed) => ed.inspect('state') },
  { key: 'w', modifiers: ['ctrl'], predicate: () => isMac, action: (ed) => ed.inspect('state-as-macro') },
];

const isMac = navigator.platform.toUpperCase().includes('MAC');

const move = (movement: Movement, extend: boolean): Message => ({
  type: 'navigation',
  op: { type: 'move', movement, extend },
});

const matchBinding = (binding: KeyBinding, e: KeyboardEvent): boolean => {
  if (binding.key !== e.key) return false;

  const mods = binding.modifiers ?? [];
  const expectShift = mods.includes('shift');
  const expectAlt = mods.includes('alt');
  const expectCtrl = mods.includes('ctrl') || (!isMac && mods.includes('mod'));
  const expectMeta = isMac && mods.includes('mod');

  if (e.shiftKey !== expectShift) return false;
  if (e.altKey !== expectAlt) return false;
  if (e.ctrlKey !== expectCtrl) return false;
  if (e.metaKey !== expectMeta) return false;

  if (binding.predicate && !binding.predicate(e)) return false;

  return true;
};

export const handleKeyDown: EditorEventHandler<HTMLInputElement, KeyboardEvent> = (editor, e) => {
  if (e.isComposing) return;

  const binding = bindings.find((b) => matchBinding(b, e));
  if (binding) {
    e.preventDefault();
    e.stopPropagation();
    binding.action(editor, e);
  }
};
