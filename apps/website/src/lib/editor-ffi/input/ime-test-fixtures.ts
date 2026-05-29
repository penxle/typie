import { vi } from 'vitest';
import type { ImeContext, ImeTextInput } from './ime-context';

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
  input: ImeTextInput,
  inputType: string,
  data: string | null = null,
): InputEvent & { currentTarget: ImeTextInput } =>
  ({
    currentTarget: input,
    inputType,
    data,
    preventDefault: vi.fn(),
  }) as unknown as InputEvent & { currentTarget: ImeTextInput };

export const inputEvent = (input: ImeTextInput): Event & { currentTarget: ImeTextInput } =>
  ({ currentTarget: input }) as Event & { currentTarget: ImeTextInput };

export const compositionEvent = (input: ImeTextInput, data = ''): CompositionEvent & { currentTarget: ImeTextInput } =>
  ({ currentTarget: input, data }) as CompositionEvent & { currentTarget: ImeTextInput };
