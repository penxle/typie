import {
  canPreserveNativeInputOnEditorSync,
  codePointLength,
  flatOffsetToUtf16Index,
  readInputUtf16Selection,
  replaceContextRange,
  syncInputElementToContext,
  updateContextFromInputElement,
  utf16SelectionToFlatRange,
} from './ime-context';
import { flatImeMessage, normalizeLineBreakBeforeInput, readDomComposingReplacement, readDomInputDiff } from './ime-normalizer';
import type { Message } from '@typie/editor-ffi/browser';
import type { ImeContext, ImeRange } from './ime-context';

type ImeInputAdapterDeps = {
  readContext: () => ImeContext | null;
  enqueue: (messages: Message[]) => void;
};

type ImeEditIntent = {
  inputType: 'insertText';
  text: string;
  replacementCandidate: ImeRange;
};

const isCollapsedRange = (range: ImeRange): boolean => range.start === range.end;

export class ImeInputAdapter {
  readonly #deps: ImeInputAdapterDeps;
  #context: ImeContext | null = null;
  #pendingEditIntent: ImeEditIntent | null = null;
  #pendingCompositionText: string | null = null;
  #compositionActive = false;
  #commitPending = false;

  constructor(deps: ImeInputAdapterDeps) {
    this.#deps = deps;
  }

  syncFromEditor(input: HTMLInputElement): void {
    if (this.#compositionActive) {
      return;
    }

    const context = this.#deps.readContext();
    if (!context) {
      return;
    }

    if (this.#context && canPreserveNativeInputOnEditorSync(this.#context, context)) {
      return;
    }

    this.#context = context;
    syncInputElementToContext(input, context);
  }

  handleBeforeInput(e: InputEvent & { currentTarget: HTMLInputElement }): void {
    if (this.#commitPending && (e.inputType === 'insertText' || e.inputType === 'insertCompositionText')) {
      this.#commitPending = false;
      this.#pendingEditIntent = null;
      this.#pendingCompositionText = null;
      e.preventDefault();
      return;
    }

    const context = this.#currentContext(e.currentTarget);

    const lineBreakMessages = normalizeLineBreakBeforeInput(e.inputType);
    if (lineBreakMessages.length > 0) {
      this.#pendingEditIntent = null;
      e.preventDefault();
      this.#deps.enqueue(lineBreakMessages);
      return;
    }

    if (this.#compositionActive && e.inputType === 'insertCompositionText') {
      this.#pendingCompositionText = e.data;
    }

    this.#pendingEditIntent =
      !this.#compositionActive && context && e.inputType === 'insertText' && e.data != null
        ? {
            inputType: e.inputType,
            text: e.data,
            replacementCandidate: utf16SelectionToFlatRange(context.text, context.windowStart, readInputUtf16Selection(e.currentTarget)),
          }
        : null;

    // Browser composition is not a Compose EditCommand stream. Let the native
    // input own the preedit buffer, then translate the mutated DOM value in input.
  }

  handleInput(e: Event & { currentTarget: HTMLInputElement }): void {
    const context = this.#currentContext(e.currentTarget, false);
    if (!context) {
      return;
    }

    const diff = readDomInputDiff(context, e.currentTarget.value);
    if (!diff) {
      this.#context = updateContextFromInputElement(context, e.currentTarget, context.composing);
      return;
    }

    if (this.#compositionActive) {
      const replacement = readDomComposingReplacement(context, e.currentTarget.value, diff);
      const text = this.#pendingCompositionText ?? replacement.text;
      this.#pendingCompositionText = null;
      const composing = { start: replacement.targetStart, end: replacement.targetStart + codePointLength(text) };
      const messages = flatImeMessage([
        { type: 'set_composition', start: replacement.targetStart, end: replacement.targetEnd },
        { type: 'compose', text },
      ]);
      this.#deps.enqueue(messages);

      const nextText = replaceContextRange(context, { start: replacement.targetStart, end: replacement.targetEnd }, text);
      if (e.currentTarget.value !== nextText) {
        e.currentTarget.value = nextText;
      }
      const selection = flatOffsetToUtf16Index(nextText, context.windowStart, composing.end);
      e.currentTarget.setSelectionRange(selection, selection);
      this.#context = updateContextFromInputElement(context, e.currentTarget, composing);
      return;
    }

    const intent = this.#pendingEditIntent;
    this.#pendingEditIntent = null;
    const replacement =
      intent &&
      intent.inputType === 'insertText' &&
      intent.text === diff.insertedText &&
      diff.start === diff.end &&
      isCollapsedRange(intent.replacementCandidate)
        ? intent.replacementCandidate
        : { start: diff.start, end: diff.end };
    const messages = flatImeMessage([
      { type: 'set_selection', start: replacement.start, end: replacement.end },
      { type: 'replace_selection', text: diff.insertedText },
    ]);
    this.#deps.enqueue(messages);
    this.#context = updateContextFromInputElement(context, e.currentTarget, null);
  }

  handleCompositionStart(e: CompositionEvent & { currentTarget: HTMLInputElement }): void {
    this.#clearCommitPending();
    this.#pendingCompositionText = null;
    this.#compositionActive = true;
    this.#currentContext(e.currentTarget);
  }

  handleCompositionUpdate(e: CompositionEvent): void {
    this.#pendingCompositionText = e.data;
  }

  handleCompositionEnd(): void {
    this.#compositionActive = false;
    this.#pendingCompositionText = null;
    const hadComposition = this.#context?.composing != null;
    if (this.#context?.composing) {
      this.#context = {
        ...this.#context,
        selection: { start: this.#context.composing.end, end: this.#context.composing.end },
        composing: null,
      };
    }
    if (hadComposition) {
      const messages: Message[] = [{ type: 'composition', op: { type: 'commit_as_is' } }];
      this.#deps.enqueue(messages);
      this.#setCommitPending();
      return;
    }
  }

  #currentContext(input: HTMLInputElement, syncDom = true): ImeContext | null {
    if (this.#context) {
      return this.#context;
    }

    const context = this.#deps.readContext();
    if (!context) {
      return null;
    }

    this.#context = context;
    if (syncDom) {
      syncInputElementToContext(input, context);
    }
    return context;
  }

  #clearCommitPending(): void {
    this.#commitPending = false;
  }

  #setCommitPending(): void {
    this.#commitPending = true;
    setTimeout(() => this.#clearCommitPending(), 0);
  }
}
