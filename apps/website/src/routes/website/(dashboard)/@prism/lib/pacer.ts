const BASE_CPS = 45;
const MAX_CPS = 400;
const RUSH_HORIZON_MS = 2500;
const ADAPT_MS = 300;
const JOIN_LAG = 400;
const HARD_LAG = 5000;

export class Pacer {
  #text = '';
  #boundary = 0;
  #plain = 0;
  #rate = BASE_CPS;
  #budget = 0;
  #finalized = false;

  #jump(): void {
    this.#boundary = this.#text.length;
    this.#plain = this.#boundary;
    this.#budget = 0;
  }

  get text(): string {
    return this.#text;
  }

  get boundary(): number {
    return this.#boundary;
  }

  get plain(): number {
    return this.#plain;
  }

  get finalized(): boolean {
    return this.#finalized;
  }

  get done(): boolean {
    return this.#finalized && this.#boundary >= this.#text.length;
  }

  skip(): void {
    this.#jump();
  }

  retarget(text: string): void {
    if (this.#finalized) return;
    this.#text = text;
    if (text.length < this.#boundary || (this.#boundary === 0 && text.length > JOIN_LAG) || text.length - this.#boundary > HARD_LAG) {
      this.#jump();
    }
  }

  finalize(text: string): void {
    this.#finalized = true;
    this.#text = text;
    if (text.length < this.#boundary) this.#jump();
    this.#plain = Math.max(this.#plain, this.#boundary);
  }

  advance(dt: number): boolean {
    const before = this.#boundary;
    const pending = this.#text.length - this.#boundary;

    if (pending > 0) {
      const desired = Math.min(MAX_CPS, Math.max(BASE_CPS, (pending / RUSH_HORIZON_MS) * 1000));
      this.#rate += (desired - this.#rate) * (1 - Math.exp(-dt / ADAPT_MS));
      this.#budget += (this.#rate * dt) / 1000;
      const step = Math.floor(this.#budget);
      if (step > 0) {
        this.#budget -= step;
        this.#boundary = Math.min(this.#text.length, this.#boundary + step);
      }
    } else {
      this.#rate += (BASE_CPS - this.#rate) * (1 - Math.exp(-dt / ADAPT_MS));
      this.#budget = 0;
    }

    return this.#boundary !== before;
  }
}
