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
import type { ImeContext, ImeRange, ImeTextInput } from './ime-context';
import type { DomInputDiff } from './ime-normalizer';

type ImeInputAdapterDeps = {
  readContext: () => ImeContext | null;
  enqueue: (messages: Message[]) => void;
};

type ImeEditIntent = {
  inputType: 'insertText';
  text: string;
  replacementCandidate: ImeRange;
};

type ImeCompositionEdit = {
  target: ImeRange;
  text: string;
};

const isCollapsedRange = (range: ImeRange): boolean => range.start === range.end;

const readContextCompositionText = (context: ImeContext): string | null => {
  if (!context.composing) {
    return null;
  }

  return codePointSlice(context.text, context.composing.start - context.windowStart, context.composing.end - context.windowStart);
};

const resolveActiveCompositionSyncContext = (local: ImeContext, incoming: ImeContext): ImeContext | null => {
  const localText = readContextCompositionText(local);
  if (localText == null) {
    return null;
  }

  if (localText === readContextCompositionText(incoming)) {
    return incoming;
  }

  if (incoming.composing || incoming.selection.start !== incoming.selection.end || local.selection.start !== local.selection.end) {
    return null;
  }

  const textLength = codePointLength(localText);
  if (textLength === 0) {
    return null;
  }

  const end = incoming.selection.end;
  const start = end - textLength;
  if (start < incoming.windowStart) {
    return null;
  }

  const incomingText = codePointSlice(incoming.text, start - incoming.windowStart, end - incoming.windowStart);
  if (incomingText !== localText) {
    return null;
  }

  return {
    ...incoming,
    composing: { start, end },
  };
};

const rebaseNativeCompositionContext = (local: ImeContext, incoming: ImeContext, input: ImeTextInput): ImeContext | null => {
  if (!local.composing) {
    return null;
  }

  const syncContext = resolveActiveCompositionSyncContext(local, incoming);
  if (!syncContext?.composing) {
    return null;
  }

  const compositionText = readContextCompositionText(local);
  if (compositionText == null) {
    return null;
  }

  const localStart = local.composing.start - local.windowStart;
  const compositionLength = codePointLength(compositionText);
  if (localStart < 0 || localStart + compositionLength > codePointLength(input.value)) {
    return null;
  }

  if (codePointSlice(input.value, localStart, localStart + compositionLength) !== compositionText) {
    return null;
  }

  const windowStart = syncContext.composing.start - localStart;
  const composing = {
    start: syncContext.composing.start,
    end: syncContext.composing.start + compositionLength,
  };

  return {
    text: input.value,
    windowStart,
    selection: utf16SelectionToFlatRange(input.value, windowStart, readInputUtf16Selection(input)),
    composing,
  };
};

const readDuplicateCommittedPreeditTarget = (context: ImeContext, input: ImeTextInput, text: string | null): ImeRange | null => {
  if (!context.composing || text == null || readContextCompositionText(context) !== text) {
    return null;
  }

  const selection = utf16SelectionToFlatRange(context.text, context.windowStart, readInputUtf16Selection(input));
  return selection.start === selection.end && selection.start === context.composing.end ? selection : null;
};

const isDuplicateCommittedPreeditDiff = (context: ImeContext, diff: { start: number; end: number; insertedText: string }): boolean =>
  !!context.composing &&
  diff.start === context.composing.end &&
  diff.end === context.composing.end &&
  readContextCompositionText(context) === diff.insertedText;

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

  #handleInputWithoutDiff(context: ImeContext, input: ImeTextInput): void {
    const intent = this.#pendingEditIntent;
    this.#pendingEditIntent = null;

    if (this.#compositionActive) {
      const pendingTarget = this.#pendingCompositionTarget;
      this.#pendingCompositionTarget = null;
      const text = this.#pendingCompositionText;
      this.#pendingCompositionText = null;
      const target = pendingTarget ?? context.composing;
      if (target && text != null) {
        this.#applyCompositionEdit(context, input, { target, text });
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
    this.#context = updateContextFromInputElement(context, input, context.composing);
  }

  #handleCompositionInputWithDiff(context: ImeContext, input: ImeTextInput, diff: DomInputDiff): void {
    if (this.#pendingCompositionText == null && isDuplicateCommittedPreeditDiff(context, diff)) {
      if (input.value !== context.text) {
        input.value = context.text;
      }
      const selection = flatOffsetToUtf16Index(context.text, context.windowStart, context.selection.end);
      input.setSelectionRange(selection, selection);
      this.#context = context;
      return;
    }

    const pendingTarget = this.#pendingCompositionTarget;
    this.#pendingCompositionTarget = null;
    const replacement = readDomComposingReplacement(context, input.value, diff);
    if (pendingTarget) {
      replacement.targetStart = pendingTarget.start;
      replacement.targetEnd = pendingTarget.end;
    }
    const edit = this.#compositionEdit(context, replacement);
    this.#pendingCompositionText = null;
    this.#applyCompositionEdit(context, input, edit);
  }

  #handleTextInputWithDiff(context: ImeContext, input: ImeTextInput, diff: DomInputDiff): void {
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
    this.#context = updateContextFromInputElement(context, input, null);
  }

  #applyCompositionEdit(context: ImeContext, input: ImeTextInput, edit: ImeCompositionEdit): void {
    const composing = { start: edit.target.start, end: edit.target.start + codePointLength(edit.text) };
    const messages = textInputMessage([
      { type: 'set_composition', start: edit.target.start, end: edit.target.end },
      { type: 'compose', text: edit.text },
    ]);
    this.#deps.enqueue(messages);

    const nextText = replaceContextRange(context, edit.target, edit.text);
    if (input.value !== nextText) {
      input.value = nextText;
    }
    const selection = flatOffsetToUtf16Index(nextText, context.windowStart, composing.end);
    input.setSelectionRange(selection, selection);
    this.#context = updateContextFromInputElement(context, input, composing);
  }

  #currentContext(input: ImeTextInput, syncDom = true): ImeContext | null {
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

  #compositionEdit(context: ImeContext, replacement: { targetStart: number; targetEnd: number; text: string }): ImeCompositionEdit {
    const target = { start: replacement.targetStart, end: replacement.targetEnd };
    const pending = this.#pendingCompositionText;
    if (!pending || !context.composing) {
      const edit = { target, text: pending ?? replacement.text };
      return edit;
    }

    const current = readContextCompositionText(context) ?? '';
    const targetsCurrentComposition = target.start === context.composing.start && target.end === context.composing.end;

    if (replacement.text === `${current}${pending}` && current.endsWith(pending) && targetsCurrentComposition) {
      return { target, text: replacement.text };
    }

    return { target, text: pending };
  }

  syncFromEditor(input: ImeTextInput): void {
    const context = this.#deps.readContext();
    if (!context) {
      return;
    }

    if (this.#compositionActive) {
      if (!this.#context) {
        this.#context = context;
        return;
      }

      if (canPreserveNativeInputOnEditorSync(this.#context, context)) {
        return;
      }

      const rebasedContext = rebaseNativeCompositionContext(this.#context, context, input);
      if (rebasedContext) {
        this.#context = rebasedContext;
      }
      return;
    }

    if (this.#context && canPreserveNativeInputOnEditorSync(this.#context, context)) {
      return;
    }

    this.#context = context;
    syncInputElementToContext(input, context);
  }

  handleBeforeInput(e: InputEvent & { currentTarget: ImeTextInput }): void {
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

    const duplicateCommittedPreeditTarget =
      this.#compositionActive && this.#pendingCompositionText == null && e.inputType === 'insertText' && context
        ? readDuplicateCommittedPreeditTarget(context, e.currentTarget, e.data)
        : null;
    if (duplicateCommittedPreeditTarget) {
      this.#pendingEditIntent = null;
      this.#pendingCompositionTarget = duplicateCommittedPreeditTarget;
      e.preventDefault();
      return;
    }

    if (this.#compositionActive && e.inputType === 'insertCompositionText') {
      this.#pendingCompositionText = e.data;
      this.#pendingCompositionTarget ??=
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

  handleInput(e: Event & { currentTarget: ImeTextInput }): void {
    const context = this.#currentContext(e.currentTarget, false);
    if (!context) {
      return;
    }

    const diff = readDomInputDiff(context, e.currentTarget.value);
    if (!diff) {
      this.#handleInputWithoutDiff(context, e.currentTarget);
      return;
    }

    if (this.#compositionActive) {
      this.#handleCompositionInputWithDiff(context, e.currentTarget, diff);
      return;
    }
    this.#handleTextInputWithDiff(context, e.currentTarget, diff);
  }

  handleCompositionStart(e: CompositionEvent & { currentTarget: ImeTextInput }): void {
    this.#clearCommitPending();
    const wasCompositionActive = this.#compositionActive;
    const pendingTarget = this.#pendingCompositionTarget;
    this.#pendingCompositionText = null;
    this.#compositionActive = true;
    const context = this.#currentContext(e.currentTarget);
    this.#pendingCompositionTarget =
      pendingTarget ?? (context?.composing && !wasCompositionActive ? { start: context.composing.end, end: context.composing.end } : null);
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
}
