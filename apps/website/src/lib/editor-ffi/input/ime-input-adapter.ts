import {
  canPreserveNativeInputOnEditorSync,
  codePointLength,
  codePointSlice,
  flatOffsetToUtf16Index,
  readInputUtf16Selection,
  replaceContextRange,
  syncInputElementToContext,
  updateContextFromInputElement,
  utf16SelectionToFlatRange,
} from './ime-context';
import { normalizeLineBreakBeforeInput, readDomComposingReplacement, readDomInputDiff, textInputMessage } from './ime-normalizer';
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
  #pendingCompositionTarget: ImeRange | null = null;
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
      this.#pendingCompositionTarget = null;
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
      this.#pendingCompositionTarget =
        context && !context.composing
          ? utf16SelectionToFlatRange(context.text, context.windowStart, readInputUtf16Selection(e.currentTarget))
          : null;
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
      const intent = this.#pendingEditIntent;
      this.#pendingEditIntent = null;
      if (this.#compositionActive) {
        const pendingTarget = this.#pendingCompositionTarget;
        this.#pendingCompositionTarget = null;
        const text = this.#pendingCompositionText;
        this.#pendingCompositionText = null;
        const target = pendingTarget ?? context.composing;
        if (target && text != null) {
          const composing = { start: target.start, end: target.start + codePointLength(text) };
          const messages = textInputMessage([
            { type: 'set_composition', start: target.start, end: target.end },
            { type: 'compose', text },
          ]);
          this.#deps.enqueue(messages);

          const nextText = replaceContextRange(context, target, text);
          if (e.currentTarget.value !== nextText) {
            e.currentTarget.value = nextText;
          }
          const selection = flatOffsetToUtf16Index(nextText, context.windowStart, composing.end);
          e.currentTarget.setSelectionRange(selection, selection);
          this.#context = updateContextFromInputElement(context, e.currentTarget, composing);
          return;
        }
      }
      if (intent && !isCollapsedRange(intent.replacementCandidate)) {
        const messages = textInputMessage([
          { type: 'set_selection', start: intent.replacementCandidate.start, end: intent.replacementCandidate.end },
          { type: 'replace_selection', text: intent.text },
        ]);
        this.#deps.enqueue(messages);
      }
      this.#context = updateContextFromInputElement(context, e.currentTarget, context.composing);
      return;
    }

    if (this.#compositionActive) {
      const pendingTarget = this.#pendingCompositionTarget;
      this.#pendingCompositionTarget = null;
      const replacement = readDomComposingReplacement(context, e.currentTarget.value, diff);
      if (pendingTarget && !context.composing) {
        replacement.targetStart = pendingTarget.start;
        replacement.targetEnd = pendingTarget.end;
      }
      const text = this.#compositionText(context, replacement);
      this.#pendingCompositionText = null;
      const composing = { start: replacement.targetStart, end: replacement.targetStart + codePointLength(text) };
      const messages = textInputMessage([
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
    this.#pendingCompositionTarget = null;
    const replacement =
      intent &&
      intent.inputType === 'insertText' &&
      intent.text === diff.insertedText &&
      diff.start === diff.end &&
      isCollapsedRange(intent.replacementCandidate)
        ? intent.replacementCandidate
        : { start: diff.start, end: diff.end };
    const messages = textInputMessage([
      { type: 'set_selection', start: replacement.start, end: replacement.end },
      { type: 'replace_selection', text: diff.insertedText },
    ]);
    this.#deps.enqueue(messages);
    this.#context = updateContextFromInputElement(context, e.currentTarget, null);
  }

  handleCompositionStart(e: CompositionEvent & { currentTarget: HTMLInputElement }): void {
    this.#clearCommitPending();
    this.#pendingCompositionText = null;
    this.#pendingCompositionTarget = null;
    this.#compositionActive = true;
    this.#currentContext(e.currentTarget);
  }

  handleCompositionUpdate(e: CompositionEvent): void {
    this.#pendingCompositionText = e.data;
  }

  handleCompositionEnd(): void {
    this.#compositionActive = false;
    this.#pendingCompositionText = null;
    this.#pendingCompositionTarget = null;
    const hadComposition = this.#context?.composing != null;
    if (this.#context?.composing) {
      this.#context = {
        ...this.#context,
        selection: { start: this.#context.composing.end, end: this.#context.composing.end },
        composing: null,
      };
    }
    if (hadComposition) {
      const messages: Message[] = [{ type: 'text_input', ops: [{ type: 'commit_as_is' }] }];
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

  #compositionText(context: ImeContext, replacement: { text: string }): string {
    const pending = this.#pendingCompositionText;
    if (!pending || !context.composing) {
      return pending ?? replacement.text;
    }

    const current = codePointSlice(
      context.text,
      context.composing.start - context.windowStart,
      context.composing.end - context.windowStart,
    );
    if (replacement.text === `${current}${pending}` && current.endsWith(pending)) {
      return replacement.text;
    }

    return pending;
  }
}
