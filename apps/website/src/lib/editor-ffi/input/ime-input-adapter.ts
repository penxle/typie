import { initialCompositionTailState, resolveCompositionTail } from './composition-tail-resolver';
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
import type { CompositionTailEffect, CompositionTailObservation, CompositionTailState } from './composition-tail-resolver';
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

type DeferredCompositionEdit = {
  generation: number;
  context: ImeContext;
  input: ImeTextInput;
  edit: ImeCompositionEdit;
  timer: ReturnType<typeof setTimeout>;
};

const isCollapsedRange = (range: ImeRange): boolean => range.start === range.end;
const isSameRange = (left: ImeRange, right: ImeRange): boolean => left.start === right.start && left.end === right.end;

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
  if (text == null || !context.composing || readContextCompositionText(context) !== text) {
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
  #compositionTailState: CompositionTailState = initialCompositionTailState;
  #deferredCompositionEdit: DeferredCompositionEdit | null = null;
  #compositionActive = false;
  #commitPendingText: string | null = null;
  #resyncInProgress = false;

  constructor(deps: ImeInputAdapterDeps) {
    this.#deps = deps;
  }

  #takeDeferredCompositionEdit(generation: number): DeferredCompositionEdit | null {
    const deferred = this.#deferredCompositionEdit;
    if (deferred && deferred.generation !== generation) {
      return null;
    }
    this.#deferredCompositionEdit = null;
    if (deferred) {
      clearTimeout(deferred.timer);
    }
    return deferred;
  }

  #applyDeferredCompositionEdit(generation: number): DeferredCompositionEdit | null {
    const deferred = this.#takeDeferredCompositionEdit(generation);
    if (!deferred) {
      return null;
    }
    this.#applyCompositionEdit(deferred.context, deferred.input, deferred.edit);
    return deferred;
  }

  #observeCompositionTail(observation: CompositionTailObservation): CompositionTailEffect[] {
    const resolution = resolveCompositionTail(this.#compositionTailState, observation);
    this.#compositionTailState = resolution.state;
    return resolution.effects;
  }

  #applyCompositionTailEffects(
    effects: CompositionTailEffect[],
    currentEdit?: { context: ImeContext; input: ImeTextInput; edit: ImeCompositionEdit },
  ): Extract<CompositionTailEffect, { type: 'commit_then_insert' }> | null {
    let commit: Extract<CompositionTailEffect, { type: 'commit_then_insert' }> | null = null;
    let pendingCurrentEdit = currentEdit;
    for (const effect of effects) {
      switch (effect.type) {
        case 'apply_current_edit': {
          if (!pendingCurrentEdit) {
            throw new Error('Composition-tail resolution requires the current edit');
          }
          this.#applyCompositionEdit(pendingCurrentEdit.context, pendingCurrentEdit.input, pendingCurrentEdit.edit);
          break;
        }
        case 'defer_current_edit': {
          if (!currentEdit) {
            throw new Error('Composition-tail resolution requires the current edit');
          }
          this.#deferCompositionEdit(effect.generation, currentEdit.context, currentEdit.input, currentEdit.edit);
          break;
        }
        case 'apply_deferred_edit': {
          const deferred = this.#applyDeferredCompositionEdit(effect.generation);
          const context = this.#context;
          // Both effects were resolved from the pre-deferred range. A following edit of the
          // same composition must target the range produced by the deferred edit.
          if (
            deferred &&
            pendingCurrentEdit &&
            deferred.input === pendingCurrentEdit.input &&
            isSameRange(deferred.edit.target, pendingCurrentEdit.edit.target) &&
            context?.composing
          ) {
            pendingCurrentEdit = {
              ...pendingCurrentEdit,
              context,
              edit: { ...pendingCurrentEdit.edit, target: context.composing },
            };
          }
          break;
        }
        case 'discard_deferred_edit': {
          this.#takeDeferredCompositionEdit(effect.generation);
          break;
        }
        case 'commit_then_insert': {
          commit = effect;
          break;
        }
      }
    }
    return commit;
  }

  #continueCompositionTail(): void {
    this.#applyCompositionTailEffects(this.#observeCompositionTail({ type: 'composition_continues' }));
  }

  #deferCompositionEdit(generation: number, context: ImeContext, input: ImeTextInput, edit: ImeCompositionEdit): void {
    const timer = setTimeout(() => {
      this.#applyCompositionTailEffects(this.#observeCompositionTail({ type: 'timeout', generation }));
    }, 0);
    this.#deferredCompositionEdit = { generation, context, input, edit, timer };
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
        this.#applyCompositionTailEffects(this.#observeCompositionTail({ type: 'key_down', key: null }));
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
    const currentText = readContextCompositionText(context);
    const targetsCurrentComposition =
      !!context.composing && edit.target.start === context.composing.start && edit.target.end === context.composing.end;
    const effects = this.#observeCompositionTail({
      type: 'composition_edit',
      currentText,
      editText: edit.text,
      targetsCurrentComposition,
    });
    this.#applyCompositionTailEffects(effects, { context, input, edit });
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
    // The editor has no inline newline: multi-line insertions become
    // paragraph splits via the enter key path.
    const segments = diff.insertedText.replaceAll('\r\n', '\n').replaceAll('\r', '\n').split('\n');
    const messages = textInputMessage([
      { type: 'set_selection', start: replacement.start, end: replacement.end },
      { type: 'replace_selection', text: segments[0] },
    ]);
    for (const segment of segments.slice(1)) {
      messages.push({ type: 'key', event: { key: 'enter' } });
      if (segment.length > 0) {
        messages.push(...textInputMessage([{ type: 'replace_selection', text: segment }]));
      }
    }
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
    this.#commitPendingText = null;
  }

  #setCommitPending(text: string): void {
    this.#commitPendingText = text;
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

    if (targetsCurrentComposition && replacement.text === `${current}${pending}` && current.endsWith(pending)) {
      return { target, text: replacement.text };
    }

    return { target, text: pending };
  }

  resetForResync(input: ImeTextInput | null): void {
    const wasComposing = this.#compositionActive;

    this.#resyncInProgress = true;
    try {
      this.#context = null;
      this.#pendingEditIntent = null;
      this.#pendingCompositionText = null;
      this.#pendingCompositionTarget = null;
      this.#applyCompositionTailEffects(this.#observeCompositionTail({ type: 'reset' }));
      this.#compositionActive = false;
      this.#commitPendingText = null;

      if (!input) {
        return;
      }

      this.syncFromEditor(input);

      if (wasComposing && document.activeElement === input) {
        input.blur();
        input.focus({ preventScroll: true });
      }
    } finally {
      this.#resyncInProgress = false;
    }
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
    if (this.#resyncInProgress) {
      return;
    }

    this.#continueCompositionTail();

    if (
      this.#commitPendingText != null &&
      (e.inputType === 'insertText' || e.inputType === 'insertCompositionText') &&
      e.data === this.#commitPendingText
    ) {
      this.#commitPendingText = null;
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
      context && this.#compositionActive && this.#pendingCompositionText == null && e.inputType === 'insertText'
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

    if (this.#compositionActive && e.inputType === 'insertText' && e.data != null && context?.composing) {
      const committedText = readContextCompositionText(context);
      if (committedText != null) {
        e.preventDefault();
        this.#compositionActive = false;
        this.#pendingEditIntent = null;
        this.#pendingCompositionText = null;
        this.#pendingCompositionTarget = null;
        this.#applyCompositionTailEffects(this.#observeCompositionTail({ type: 'key_down', key: null }));
        this.#context = {
          ...context,
          selection: { start: context.composing.end, end: context.composing.end },
          composing: null,
        };
        this.#deps.enqueue([
          { type: 'text_input', ops: [{ type: 'commit_as_is' }] },
          ...textInputMessage([{ type: 'replace_selection', text: e.data }]),
        ]);
        this.#setCommitPending(committedText);
        return;
      }
    }

    this.#pendingEditIntent =
      context && !this.#compositionActive && e.inputType === 'insertText' && e.data != null
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
    if (this.#resyncInProgress) {
      return;
    }

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
    if (this.#resyncInProgress) {
      return;
    }

    this.#continueCompositionTail();
    this.#applyCompositionTailEffects(this.#observeCompositionTail({ type: 'key_down', key: null }));

    this.#clearCommitPending();
    const wasCompositionActive = this.#compositionActive;
    const pendingTarget = this.#pendingCompositionTarget;
    this.#pendingCompositionText = null;
    this.#compositionActive = true;
    const context = this.#currentContext(e.currentTarget);
    this.#pendingCompositionTarget =
      pendingTarget ?? (!wasCompositionActive && context?.composing ? { start: context.composing.end, end: context.composing.end } : null);
  }

  handleCompositionUpdate(e: CompositionEvent): void {
    if (this.#resyncInProgress) {
      return;
    }

    this.#continueCompositionTail();

    this.#pendingCompositionText = e.data;
  }

  handleKeyDown(e: KeyboardEvent): void {
    const key =
      this.#compositionActive && e.isComposing && !e.ctrlKey && !e.metaKey && !e.altKey && codePointLength(e.key) === 1 ? e.key : null;
    this.#applyCompositionTailEffects(this.#observeCompositionTail({ type: 'key_down', key }));
  }

  handleCompositionEnd(): boolean {
    if (this.#resyncInProgress) {
      return false;
    }

    const commitTail = this.#applyCompositionTailEffects(this.#observeCompositionTail({ type: 'composition_end' }));
    if (commitTail) {
      const deferred = this.#takeDeferredCompositionEdit(commitTail.generation);
      if (deferred) {
        const committedText = readContextCompositionText(deferred.context);
        this.#compositionActive = false;
        this.#pendingCompositionText = null;
        this.#pendingCompositionTarget = null;
        this.#context = updateContextFromInputElement(deferred.context, deferred.input, null);
        if (committedText != null) {
          this.#deps.enqueue([
            { type: 'text_input', ops: [{ type: 'commit_as_is' }] },
            ...textInputMessage([{ type: 'replace_selection', text: commitTail.text }]),
          ]);
          this.#setCommitPending(committedText);
          return true;
        }
      }
    }

    this.#compositionActive = false;
    this.#pendingCompositionText = null;
    this.#pendingCompositionTarget = null;
    const committedText = this.#context ? readContextCompositionText(this.#context) : null;
    if (this.#context?.composing) {
      this.#context = {
        ...this.#context,
        selection: { start: this.#context.composing.end, end: this.#context.composing.end },
        composing: null,
      };
    }
    if (committedText != null) {
      const messages: Message[] = [{ type: 'text_input', ops: [{ type: 'commit_as_is' }] }];
      this.#deps.enqueue(messages);
      this.#setCommitPending(committedText);
      return true;
    }
    return false;
  }
}
