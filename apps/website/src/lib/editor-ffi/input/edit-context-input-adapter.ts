import { codePointLength, codePointSlice, flatOffsetToUtf16Index, utf16IndexToFlatOffset } from './ime-context';
import { normalizeLineBreakBeforeInput, textInputMessage } from './ime-normalizer';
import type { Message } from '@typie/editor-ffi/browser';
import type { ImeContext, ImeRange } from './ime-context';

export class EditContextInputAdapter {
  readonly #readContext: () => ImeContext | null;
  readonly #enqueue: (messages: Message[]) => void;
  #context: ImeContext | null = null;
  #composing = false;
  readonly editContext = new EditContext();

  constructor(readContext: () => ImeContext | null, enqueue: (messages: Message[]) => void) {
    this.#readContext = readContext;
    this.#enqueue = enqueue;
  }

  get composing(): boolean {
    return this.#composing;
  }

  flatOffset(index: number): number {
    return utf16IndexToFlatOffset(this.editContext.text, this.#context?.windowStart ?? 0, index);
  }

  syncFromEditor(): void {
    const incoming = this.#readContext();
    if (!incoming) return;

    if (this.#composing) {
      const local = this.#context;
      // Keep the native buffer and clause selection throughout composition, even
      // when the engine's surrounding-text window moves as the preedit grows.
      if (local?.composing && incoming.composing) {
        const text = codePointSlice(local.text, local.composing.start - local.windowStart, local.composing.end - local.windowStart);
        const nextText = codePointSlice(
          incoming.text,
          incoming.composing.start - incoming.windowStart,
          incoming.composing.end - incoming.windowStart,
        );
        if (text === nextText) {
          this.#context = {
            ...local,
            windowStart: local.windowStart + incoming.composing.start - local.composing.start,
            composing: incoming.composing,
          };
        }
      }
      return;
    }

    this.#context = incoming;
    if (this.editContext.text !== incoming.text) {
      this.editContext.updateText(0, this.editContext.text.length, incoming.text);
    }
    const start = flatOffsetToUtf16Index(incoming.text, incoming.windowStart, incoming.selection.start);
    const end = flatOffsetToUtf16Index(incoming.text, incoming.windowStart, incoming.selection.end);
    if (this.editContext.selectionStart !== start || this.editContext.selectionEnd !== end) {
      this.editContext.updateSelection(start, end);
    }
  }

  handleCompositionStart(): void {
    this.syncFromEditor();
    this.#composing = true;
  }

  handleCompositionEnd(): boolean {
    const committed = this.#composing && !!this.#context?.composing;
    this.#composing = false;
    if (committed) this.#enqueue(textInputMessage([{ type: 'commit_as_is' }]));
    return committed;
  }

  handleTextUpdate(event: TextUpdateEvent): void {
    const previous = this.#context;
    if (!previous) return;

    const selection = { start: this.flatOffset(event.selectionStart), end: this.flatOffset(event.selectionEnd) };
    // A selection-only textupdate uses an empty replacement range (Chromium
    // sends 0..0). It must not move insertion to that range or reset preedit.
    if (event.updateRangeStart === event.updateRangeEnd && event.text.length === 0) {
      this.#context = { ...previous, selection };
      if (!this.#composing) this.#enqueue(textInputMessage([{ type: 'set_selection', ...selection }]));
      return;
    }

    const start = utf16IndexToFlatOffset(previous.text, previous.windowStart, event.updateRangeStart);
    const end = utf16IndexToFlatOffset(previous.text, previous.windowStart, event.updateRangeEnd);
    const composing: ImeRange | null = this.#composing ? { start, end: start + codePointLength(event.text) } : null;
    this.#context = {
      ...previous,
      text: this.editContext.text,
      selection,
      composing,
    };

    this.#enqueue(
      textInputMessage(
        this.#composing
          ? [
              { type: 'set_composition', start, end },
              { type: 'compose', text: event.text },
            ]
          : [
              { type: 'set_selection', start, end },
              { type: 'replace_selection', text: event.text },
            ],
      ),
    );
  }

  handleBeforeInput(event: InputEvent): void {
    const messages = normalizeLineBreakBeforeInput(event.inputType);
    if (messages.length > 0 && !this.#composing) {
      event.preventDefault();
      this.#enqueue(messages);
    }
  }

  resetForResync(): void {
    this.#composing = false;
    this.#context = null;
    this.syncFromEditor();
  }
}
