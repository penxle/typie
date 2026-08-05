import type { CommandOutcome, EditorEvent, Message } from '@typie/editor-ffi/browser';
import type { EditorSnapshot } from './editor.svelte';

export type EditorPublicationResult = { type: 'published'; revision: number } | { type: 'no_host' };

export class EditorRequest {
  readonly #messages: Message[];
  #beforePublish: ((update: EditorUpdate) => void) | undefined;
  #discardBeforePublish: (() => void) | undefined;

  constructor(messages: readonly Message[] = []) {
    this.#messages = [...messages];
  }

  get messages(): readonly Message[] {
    return this.#messages;
  }

  enqueue(message: Message): void {
    this.#messages.push(message);
  }

  beforePublish(block: (update: EditorUpdate) => void, onDiscard?: () => void): void {
    const discardPrevious = this.#discardBeforePublish;
    this.#beforePublish = block;
    this.#discardBeforePublish = onDiscard;
    discardPrevious?.();
  }

  runBeforePublish(update: EditorUpdate): void {
    const block = this.#beforePublish;
    const onDiscard = this.#discardBeforePublish;
    this.#beforePublish = undefined;
    this.#discardBeforePublish = undefined;
    try {
      block?.(update);
    } catch (err) {
      onDiscard?.();
      throw err;
    }
  }

  discard(): void {
    const onDiscard = this.#discardBeforePublish;
    this.#beforePublish = undefined;
    this.#discardBeforePublish = undefined;
    onDiscard?.();
  }

  removeMessagesWhere(predicate: (message: Message) => boolean): boolean {
    const retained = this.#messages.filter((message) => !predicate(message));
    if (retained.length === this.#messages.length) return false;
    this.#messages.splice(0, this.#messages.length, ...retained);
    return true;
  }

  get empty(): boolean {
    return this.#messages.length === 0;
  }
}

export class EditorUpdate {
  readonly #awaitPublishedFn: (signal?: AbortSignal) => Promise<EditorPublicationResult>;
  readonly revision: number;
  readonly snapshot: EditorSnapshot;
  readonly commandOutcomes: readonly CommandOutcome[];
  readonly events: readonly EditorEvent[];

  constructor(
    revision: number,
    snapshot: EditorSnapshot,
    commandOutcomes: readonly CommandOutcome[],
    events: readonly EditorEvent[],
    awaitPublished: (signal?: AbortSignal) => Promise<EditorPublicationResult>,
  ) {
    this.revision = revision;
    this.snapshot = snapshot;
    this.commandOutcomes = commandOutcomes;
    this.events = events;
    this.#awaitPublishedFn = awaitPublished;
  }

  awaitPublished(signal?: AbortSignal): Promise<EditorPublicationResult> {
    return this.#awaitPublishedFn(signal);
  }
}
