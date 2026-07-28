import type { CommandOutcome, EditorEvent, Message } from '@typie/editor-ffi/browser';
import type { EditorSnapshot } from './editor.svelte';

export type EditorPublicationResult = { type: 'published'; revision: number } | { type: 'no_host' };

export class EditorRequest {
  readonly #messages: Message[];

  constructor(messages: readonly Message[] = []) {
    this.#messages = [...messages];
  }

  get messages(): readonly Message[] {
    return this.#messages;
  }

  enqueue(message: Message): void {
    this.#messages.push(message);
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
