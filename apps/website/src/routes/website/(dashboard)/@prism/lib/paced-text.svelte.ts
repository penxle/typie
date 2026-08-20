import { parseMarkdown } from './markdown.ts';
import { Pacer } from './pacer.ts';
import type { BlockNode } from './markdown.ts';

// Pacer의 Svelte 어댑터 — 페이서 상태를 $state로 비추고, 공개 경계까지의 마크다운 트리를 파생한다.
// instant는 reduced-motion용: 페이싱 없이 도착 즉시 전량 공개(무페이드).
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
