import type { Message, Movement } from '@typie/editor-ffi/browser';
import type { Editor } from '../editor.svelte';
import type { EditorEventHandler } from '../types';

type KeyBindingModifier = 'shift' | 'mod' | 'ctrl' | 'alt';

type KeyBinding = {
  key: string | string[];
  modifiers?: KeyBindingModifier[];
  predicate?: (e: KeyboardEvent) => boolean;
  action: (editor: Editor, e: KeyboardEvent) => void;
};

const isMac = navigator.platform.toUpperCase().includes('MAC');

const word: KeyBindingModifier[] = isMac ? ['alt'] : ['ctrl'];
const wordShift: KeyBindingModifier[] = isMac ? ['shift', 'alt'] : ['shift', 'ctrl'];
const macOnly = () => isMac;

const bindings: KeyBinding[] = [
  { key: 'ArrowLeft', action: (ed) => ed.enqueue(move({ type: 'grapheme', direction: 'backward' }, false)) },
  { key: 'ArrowLeft', modifiers: ['shift'], action: (ed) => ed.enqueue(move({ type: 'grapheme', direction: 'backward' }, true)) },
  { key: 'ArrowLeft', modifiers: word, action: (ed) => ed.enqueue(move({ type: 'word', direction: 'backward' }, false)) },
  { key: 'ArrowLeft', modifiers: wordShift, action: (ed) => ed.enqueue(move({ type: 'word', direction: 'backward' }, true)) },
  {
    key: 'ArrowLeft',
    modifiers: ['mod'],
    predicate: macOnly,
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'backward', axis: 'horizontal' }, false)),
  },
  {
    key: 'ArrowLeft',
    modifiers: ['shift', 'mod'],
    predicate: macOnly,
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'backward', axis: 'horizontal' }, true)),
  },

  { key: 'ArrowRight', action: (ed) => ed.enqueue(move({ type: 'grapheme', direction: 'forward' }, false)) },
  { key: 'ArrowRight', modifiers: ['shift'], action: (ed) => ed.enqueue(move({ type: 'grapheme', direction: 'forward' }, true)) },
  { key: 'ArrowRight', modifiers: word, action: (ed) => ed.enqueue(move({ type: 'word', direction: 'forward' }, false)) },
  { key: 'ArrowRight', modifiers: wordShift, action: (ed) => ed.enqueue(move({ type: 'word', direction: 'forward' }, true)) },
  {
    key: 'ArrowRight',
    modifiers: ['mod'],
    predicate: macOnly,
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'forward', axis: 'horizontal' }, false)),
  },
  {
    key: 'ArrowRight',
    modifiers: ['shift', 'mod'],
    predicate: macOnly,
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'forward', axis: 'horizontal' }, true)),
  },

  { key: 'ArrowUp', action: (ed) => ed.enqueue(move({ type: 'line', direction: 'backward', axis: 'vertical' }, false)) },
  {
    key: 'ArrowUp',
    modifiers: ['shift'],
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'backward', axis: 'vertical' }, true)),
  },
  {
    key: 'ArrowUp',
    modifiers: ['mod'],
    predicate: macOnly,
    action: (ed) => ed.enqueue(move({ type: 'document', direction: 'backward' }, false)),
  },
  {
    key: 'ArrowUp',
    modifiers: ['shift', 'mod'],
    predicate: macOnly,
    action: (ed) => ed.enqueue(move({ type: 'document', direction: 'backward' }, true)),
  },

  { key: 'ArrowDown', action: (ed) => ed.enqueue(move({ type: 'line', direction: 'forward', axis: 'vertical' }, false)) },
  {
    key: 'ArrowDown',
    modifiers: ['shift'],
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'forward', axis: 'vertical' }, true)),
  },
  {
    key: 'ArrowDown',
    modifiers: ['mod'],
    predicate: macOnly,
    action: (ed) => ed.enqueue(move({ type: 'document', direction: 'forward' }, false)),
  },
  {
    key: 'ArrowDown',
    modifiers: ['shift', 'mod'],
    predicate: macOnly,
    action: (ed) => ed.enqueue(move({ type: 'document', direction: 'forward' }, true)),
  },

  { key: 'Home', action: (ed) => ed.enqueue(move({ type: 'line', direction: 'backward', axis: 'horizontal' }, false)) },
  {
    key: 'Home',
    modifiers: ['shift'],
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'backward', axis: 'horizontal' }, true)),
  },
  { key: 'Home', modifiers: ['mod'], action: (ed) => ed.enqueue(move({ type: 'document', direction: 'backward' }, false)) },
  {
    key: 'Home',
    modifiers: ['shift', 'mod'],
    action: (ed) => ed.enqueue(move({ type: 'document', direction: 'backward' }, true)),
  },

  { key: 'End', action: (ed) => ed.enqueue(move({ type: 'line', direction: 'forward', axis: 'horizontal' }, false)) },
  {
    key: 'End',
    modifiers: ['shift'],
    action: (ed) => ed.enqueue(move({ type: 'line', direction: 'forward', axis: 'horizontal' }, true)),
  },
  { key: 'End', modifiers: ['mod'], action: (ed) => ed.enqueue(move({ type: 'document', direction: 'forward' }, false)) },
  {
    key: 'End',
    modifiers: ['shift', 'mod'],
    action: (ed) => ed.enqueue(move({ type: 'document', direction: 'forward' }, true)),
  },

  { key: 'PageUp', action: (ed) => ed.enqueue(move({ type: 'page', direction: 'backward' }, false)) },
  { key: 'PageUp', modifiers: ['shift'], action: (ed) => ed.enqueue(move({ type: 'page', direction: 'backward' }, true)) },

  { key: 'PageDown', action: (ed) => ed.enqueue(move({ type: 'page', direction: 'forward' }, false)) },
  { key: 'PageDown', modifiers: ['shift'], action: (ed) => ed.enqueue(move({ type: 'page', direction: 'forward' }, true)) },

  { key: 'Enter', action: (ed) => ed.enqueue({ type: 'key', event: { key: 'enter' } }) },
  {
    key: 'Enter',
    modifiers: ['shift'],
    action: (ed) => ed.enqueue({ type: 'key', event: { key: 'enter', modifiers: { shift: true } } }),
  },
  {
    key: 'Enter',
    modifiers: ['mod'],
    action: (ed) => ed.enqueue({ type: 'insertion', op: { type: 'break', kind: 'page' } }),
  },

  { key: 'Tab', action: (ed) => ed.enqueue({ type: 'key', event: { key: 'tab' } }) },
  {
    key: 'Tab',
    modifiers: ['shift'],
    action: (ed) => ed.enqueue({ type: 'key', event: { key: 'tab', modifiers: { shift: true } } }),
  },

  { key: 'Escape', action: (ed) => ed.enqueue({ type: 'key', event: { key: 'escape' } }) },

  { key: 'Backspace', action: (ed) => ed.enqueue({ type: 'key', event: { key: 'backspace' } }) },
  { key: 'Backspace', modifiers: word, action: (ed) => ed.enqueue(del({ type: 'word', direction: 'backward' })) },
  {
    key: 'Backspace',
    modifiers: ['mod'],
    predicate: macOnly,
    action: (ed) => ed.enqueue(del({ type: 'line', direction: 'backward', axis: 'horizontal' })),
  },

  { key: 'Delete', modifiers: word, action: (ed) => ed.enqueue(del({ type: 'word', direction: 'forward' })) },
  {
    key: 'Delete',
    modifiers: ['mod'],
    predicate: macOnly,
    action: (ed) => ed.enqueue(del({ type: 'line', direction: 'forward', axis: 'horizontal' })),
  },

  {
    key: 'a',
    modifiers: ['mod'],
    action: (ed) => ed.enqueue({ type: 'selection', op: { type: 'expand', unit: 'all' } }),
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

  { key: 'z', modifiers: ['mod'], action: (ed) => ed.enqueue({ type: 'history', op: { type: 'undo' } }) },
  { key: 'z', modifiers: ['mod', 'shift'], action: (ed) => ed.enqueue({ type: 'history', op: { type: 'redo' } }) },
  {
    key: 'y',
    modifiers: ['mod'],
    predicate: () => !isMac,
    action: (ed) => ed.enqueue({ type: 'history', op: { type: 'redo' } }),
  },

  { key: ['q', 'ㅂ'], modifiers: ['ctrl'], predicate: macOnly, action: (ed) => ed.inspect('state') },
  { key: ['w', 'ㅈ'], modifiers: ['ctrl'], predicate: macOnly, action: (ed) => ed.inspect('state-as-macro') },
];

const move = (movement: Movement, extend: boolean): Message => ({
  type: 'navigation',
  op: { type: 'move', movement, extend },
});

const del = (movement: Movement): Message => ({
  type: 'deletion',
  op: { type: 'move', movement },
});

const matchBinding = (binding: KeyBinding, e: KeyboardEvent): boolean => {
  if (Array.isArray(binding.key) ? !binding.key.includes(e.key) : binding.key !== e.key) return false;

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
