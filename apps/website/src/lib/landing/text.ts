export const sentences = (text: string): string[] => text.split(/(?<=[.!?…”])\s+/).filter((item) => item.length > 0);

export const words = (sentence: string): string[] => sentence.split(' ').filter((item) => item.length > 0);
