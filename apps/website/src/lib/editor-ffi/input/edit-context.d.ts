// EditContext is not yet included in TypeScript's DOM library.
// https://www.w3.org/TR/edit-context/
type TextUpdateEvent = Event & {
  readonly updateRangeStart: number;
  readonly updateRangeEnd: number;
  readonly text: string;
  readonly selectionStart: number;
  readonly selectionEnd: number;
};

declare const TextUpdateEvent: new (type: string, options: Partial<Omit<TextUpdateEvent, keyof Event>>) => TextUpdateEvent;

type TextFormat = {
  readonly rangeStart: number;
  readonly rangeEnd: number;
  readonly underlineStyle: 'none' | 'solid' | 'dotted' | 'dashed' | 'wavy';
  readonly underlineThickness: 'none' | 'thin' | 'thick';
};

declare const TextFormat: new (options: Partial<TextFormat>) => TextFormat;

type TextFormatUpdateEvent = Event & {
  getTextFormats(): TextFormat[];
};

declare const TextFormatUpdateEvent: new (type: string, options: { textFormats: TextFormat[] }) => TextFormatUpdateEvent;

type CharacterBoundsUpdateEvent = Event & {
  readonly rangeStart: number;
  readonly rangeEnd: number;
};

type EditContextEventMap = {
  textupdate: TextUpdateEvent;
  textformatupdate: TextFormatUpdateEvent;
  characterboundsupdate: CharacterBoundsUpdateEvent;
  compositionstart: CompositionEvent;
  compositionend: CompositionEvent;
};

type EditContext = Omit<EventTarget, 'addEventListener'> & {
  readonly text: string;
  readonly selectionStart: number;
  readonly selectionEnd: number;
  addEventListener<K extends keyof EditContextEventMap>(
    type: K,
    listener: (event: EditContextEventMap[K]) => void,
    options?: AddEventListenerOptions,
  ): void;
  updateText(start: number, end: number, text: string): void;
  updateSelection(start: number, end: number): void;
  updateControlBounds(bounds: DOMRect): void;
  updateSelectionBounds(bounds: DOMRect): void;
  updateCharacterBounds(start: number, bounds: DOMRect[]): void;
  characterBounds(): DOMRect[];
};

declare const EditContext: new (options?: { text?: string; selectionStart?: number; selectionEnd?: number }) => EditContext;

// eslint-disable-next-line @typescript-eslint/consistent-type-definitions -- augments the DOM interface
interface HTMLElement {
  editContext: EditContext | null;
}
