import { vi } from 'vitest';
import type { ImeContext } from './ime-context';

export const context = (text: string, windowStart = 20): ImeContext => ({
  text,
  windowStart,
  selection: { start: windowStart + [...text].length, end: windowStart + [...text].length },
  composing: null,
});

export const composingContext = (text: string, start: number, end: number, windowStart = 20): ImeContext => ({
  ...context(text, windowStart),
  composing: { start, end },
});

export const beforeInputEvent = (
  input: HTMLInputElement,
  inputType: string,
  data: string | null = null,
): InputEvent & { currentTarget: HTMLInputElement } =>
  ({
    currentTarget: input,
    inputType,
    data,
    preventDefault: vi.fn(),
  }) as unknown as InputEvent & { currentTarget: HTMLInputElement };

export const inputEvent = (input: HTMLInputElement): Event & { currentTarget: HTMLInputElement } =>
  ({ currentTarget: input }) as Event & { currentTarget: HTMLInputElement };

export const compositionEvent = (input: HTMLInputElement, data = ''): CompositionEvent & { currentTarget: HTMLInputElement } =>
  ({ currentTarget: input, data }) as CompositionEvent & { currentTarget: HTMLInputElement };
