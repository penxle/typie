import { parseMarkdown } from './markdown.ts';
import { Pacer } from './pacer.ts';
import type { BlockNode } from './markdown.ts';

export class PacedText {
  #pacer: Pacer;
  #instant: boolean;

  text = $state('');
  boundary = $state(0);
  plain = $state(0);
  finalized = $state(false);

  blocks: BlockNode[] = $derived(parseMarkdown(this.text.slice(0, this.boundary)));

  constructor(opts?: { instant?: boolean }) {
    this.#pacer = new Pacer();
    this.#instant = opts?.instant ?? false;
  }

  #sync(): void {
    this.text = this.#pacer.text;
    this.boundary = this.#instant ? this.#pacer.text.length : this.#pacer.boundary;
    this.plain = this.#instant ? this.#pacer.text.length : this.#pacer.plain;
  }

  retarget(text: string): void {
    this.#pacer.retarget(text);
    this.#sync();
  }

  skip(): void {
    this.#pacer.skip();
    this.#sync();
  }

  finalize(text: string): void {
    this.#pacer.finalize(text);
    this.finalized = true;
    this.#sync();
  }

  advance(dt: number): boolean {
    const moved = this.#pacer.advance(dt);
    if (moved) this.#sync();
    return moved;
  }

  get done(): boolean {
    return this.#instant ? this.finalized : this.#pacer.done;
  }
}
