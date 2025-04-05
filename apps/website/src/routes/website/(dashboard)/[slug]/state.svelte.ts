import type * as Y from 'yjs';

export class YState {
  #current = $state<string>() as string;

  #yDoc: Y.Doc;
  #yText: Y.Text;

  constructor(doc: Y.Doc, name: string) {
    this.#yDoc = doc;
    this.#yText = doc.getText(name);

    this.#current = this.#yText.toString();

    const handler = () => {
      this.#current = this.#yText.toString();
    };

    $effect(() => {
      this.#yText.observe(handler);
      return () => {
        this.#yText.unobserve(handler);
      };
    });
  }

  get current() {
    return this.#current;
  }

  set current(value: string) {
    this.#current = value;
    this.#yDoc.transact(() => {
      this.#yText.delete(0, this.#yText.length);
      this.#yText.insert(0, value);
    });
  }
}
