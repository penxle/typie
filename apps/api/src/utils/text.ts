import { disassemble } from 'es-hangul';

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export const encode = (text: string) => encoder.encode(text);
export const decode = (text: Uint8Array) => decoder.decode(text);

export const decompose = (text: string | null): string | null => {
  if (!text) return null;
  return disassemble(text);
};
