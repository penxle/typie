const encoder = new TextEncoder();
const decoder = new TextDecoder();

export const encode = (text: string) => encoder.encode(text);
export const decode = (text: Uint8Array) => decoder.decode(text);
